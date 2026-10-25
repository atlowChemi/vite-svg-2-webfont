//! Little-endian binary helpers for the persistent cache. Decoding never panics: any
//! truncated or malformed input yields `None`, which callers treat as a cache miss.

use std::slice::from_ref;
use std::sync::Arc;

use kurbo::{BezPath, PathEl, Point};
use usvg::tiny_skia_path::{Path as TinyPath, PathBuilder, PathVerb};

#[derive(Default)]
pub(super) struct Writer(pub(super) Vec<u8>);

impl Writer {
    pub(super) fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    pub(super) fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    pub(super) fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    pub(super) fn f32(&mut self, value: f32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    pub(super) fn f64(&mut self, value: f64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    pub(super) fn len(&mut self, value: usize) {
        self.u32(u32::try_from(value).expect("cache sections stay below 4 GiB"));
    }
    pub(super) fn bytes(&mut self, value: &[u8]) {
        self.len(value.len());
        self.0.extend_from_slice(value);
    }
    pub(super) fn array<const N: usize>(&mut self, value: &[u8; N]) {
        self.0.extend_from_slice(value);
    }

    pub(super) fn tiny_path(&mut self, path: &TinyPath) {
        self.len(path.verbs().len());
        for verb in path.verbs() {
            self.u8(match verb {
                PathVerb::Move => 0,
                PathVerb::Line => 1,
                PathVerb::Quad => 2,
                PathVerb::Cubic => 3,
                PathVerb::Close => 4,
            });
        }
        self.len(path.points().len());
        for point in path.points() {
            self.f32(point.x);
            self.f32(point.y);
        }
    }

    pub(super) fn bez_path(&mut self, path: &BezPath) {
        self.len(path.elements().len());
        for element in path.elements() {
            let points: &[Point] = match element {
                PathEl::MoveTo(point) => {
                    self.u8(0);
                    from_ref(point)
                }
                PathEl::LineTo(point) => {
                    self.u8(1);
                    from_ref(point)
                }
                PathEl::QuadTo(a, b) => {
                    self.u8(2);
                    &[*a, *b]
                }
                PathEl::CurveTo(a, b, c) => {
                    self.u8(3);
                    &[*a, *b, *c]
                }
                PathEl::ClosePath => {
                    self.u8(4);
                    &[]
                }
            };
            for point in points {
                self.f64(point.x);
                self.f64(point.y);
            }
        }
    }
}

pub(super) struct Reader<'a>(pub(super) &'a [u8]);

impl<'a> Reader<'a> {
    pub(super) fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        if self.0.len() < count {
            return None;
        }
        let (head, tail) = self.0.split_at(count);
        self.0 = tail;
        Some(head)
    }
    pub(super) fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|bytes| bytes[0])
    }
    pub(super) fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }
    pub(super) fn u32(&mut self) -> Option<u32> {
        self.array().map(u32::from_le_bytes)
    }
    pub(super) fn u64(&mut self) -> Option<u64> {
        self.array().map(u64::from_le_bytes)
    }
    pub(super) fn f32(&mut self) -> Option<f32> {
        self.array().map(f32::from_le_bytes)
    }
    pub(super) fn f64(&mut self) -> Option<f64> {
        self.array().map(f64::from_le_bytes)
    }
    /// A length prefix, rejected when it can't fit in the remaining input at `unit` bytes each.
    pub(super) fn len(&mut self, unit: usize) -> Option<usize> {
        let count = self.u32()? as usize;
        (count.checked_mul(unit)? <= self.0.len()).then_some(count)
    }
    pub(super) fn bytes(&mut self) -> Option<&'a [u8]> {
        let count = self.len(1)?;
        self.take(count)
    }

    pub(super) fn tiny_path(&mut self) -> Option<TinyPath> {
        let verb_count = self.len(1)?;
        let verbs = verbs_from(self.take(verb_count)?)?;
        let point_count = self.len(8)?;
        let mut points = Vec::with_capacity(point_count);
        for _ in 0..point_count {
            points.push((self.f32()?, self.f32()?));
        }
        // Replaying the recorded verbs through the same builder reproduces the original path;
        // the final comparison rejects input that doesn't.
        let mut points = points.into_iter();
        let mut next = || points.next();
        let mut builder = PathBuilder::with_capacity(verbs.len(), point_count);
        for verb in &verbs {
            match verb {
                PathVerb::Move => {
                    let (x, y) = next()?;
                    builder.move_to(x, y);
                }
                PathVerb::Line => {
                    let (x, y) = next()?;
                    builder.line_to(x, y);
                }
                PathVerb::Quad => {
                    let ((x1, y1), (x, y)) = (next()?, next()?);
                    builder.quad_to(x1, y1, x, y);
                }
                PathVerb::Cubic => {
                    let ((x1, y1), (x2, y2), (x, y)) = (next()?, next()?, next()?);
                    builder.cubic_to(x1, y1, x2, y2, x, y);
                }
                PathVerb::Close => builder.close(),
            }
        }
        let path = builder.finish()?;
        (path.verbs() == verbs.as_slice()).then_some(path)
    }

    pub(super) fn bez_path(&mut self) -> Option<Arc<BezPath>> {
        let count = self.len(1)?;
        // Not reserved up front: an element takes far more memory than its minimum encoded size.
        let mut elements = Vec::new();
        for _ in 0..count {
            let tag = self.u8()?;
            let mut point = || Some(Point::new(self.f64()?, self.f64()?));
            elements.push(match tag {
                0 => PathEl::MoveTo(point()?),
                1 => PathEl::LineTo(point()?),
                2 => PathEl::QuadTo(point()?, point()?),
                3 => PathEl::CurveTo(point()?, point()?, point()?),
                4 => PathEl::ClosePath,
                _ => return None,
            });
        }
        Some(Arc::new(BezPath::from_vec(elements)))
    }
}

fn verbs_from(tags: &[u8]) -> Option<Vec<PathVerb>> {
    tags.iter()
        .map(|tag| match tag {
            0 => Some(PathVerb::Move),
            1 => Some(PathVerb::Line),
            2 => Some(PathVerb::Quad),
            3 => Some(PathVerb::Cubic),
            4 => Some(PathVerb::Close),
            _ => None,
        })
        .collect()
}
