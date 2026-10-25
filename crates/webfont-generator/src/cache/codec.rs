//! Little-endian binary helpers for the persistent cache. Decoding never panics: any
//! truncated or malformed input yields `None`, which callers treat as a cache miss.

use std::slice::from_ref;
use std::str::from_utf8;
use std::sync::Arc;

use kurbo::{BezPath, PathEl, Point};
use usvg::FillRule;
use usvg::tiny_skia_path::{Path as TinyPath, PathBuilder, PathVerb};

use crate::svg::bezpath_hash;
use crate::svg::types::{CachedProcessedGlyph, ParsedColorLayer};
use crate::types::color::{ProcessedColorLayer, ResolvedLayerPaint};

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

    pub(super) fn processed_glyph(&mut self, glyph: &CachedProcessedGlyph) {
        self.f64(glyph.height);
        self.f64(glyph.width);
        self.bytes(glyph.path_data.as_bytes());
        // The outline hash isn't stored: it keys glyph deduplication, so it's recomputed on load.
        match &glyph.ttf_path {
            Some(path) => {
                self.u8(1);
                self.bez_path(path);
            }
            None => self.u8(0),
        }
        match glyph.color_layers.as_deref() {
            None => self.u8(0),
            Some(layers) => {
                self.u8(1);
                self.len(layers.len());
                for layer in layers {
                    self.bez_path(&layer.outline);
                    self.paint(layer.paint);
                }
            }
        }
    }

    pub(super) fn parsed_color_layers(&mut self, layers: Option<&[ParsedColorLayer]>) {
        let Some(layers) = layers else {
            return self.u8(0);
        };
        self.u8(1);
        self.len(layers.len());
        for layer in layers {
            self.len(layer.path_index);
            self.paint(layer.paint);
            self.u8(match layer.fill_rule {
                FillRule::NonZero => 0,
                FillRule::EvenOdd => 1,
            });
        }
    }

    fn paint(&mut self, paint: ResolvedLayerPaint) {
        match paint {
            ResolvedLayerPaint::Foreground { alpha } => {
                self.u8(0);
                self.f32(alpha);
            }
            ResolvedLayerPaint::Solid {
                red,
                green,
                blue,
                alpha,
            } => {
                self.u8(1);
                self.0.extend_from_slice(&[red, green, blue]);
                self.f32(alpha);
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

    pub(super) fn processed_glyph(&mut self) -> Option<CachedProcessedGlyph> {
        let height = self.f64()?;
        let width = self.f64()?;
        let path_data = from_utf8(self.bytes()?).ok()?.into();
        let (ttf_path_hash, ttf_path) = match self.u8()? {
            0 => (None, None),
            1 => {
                let path = self.bez_path()?;
                (Some(bezpath_hash(&path)), Some(path))
            }
            _ => return None,
        };
        let color_layers = match self.u8()? {
            0 => None,
            1 => {
                // At least an empty outline (4 bytes) and a foreground paint (5 bytes) per layer.
                let count = self.len(9)?;
                let mut layers = Vec::new();
                for _ in 0..count {
                    let outline = self.bez_path()?;
                    let paint = self.paint()?;
                    // Recomputed, never trusted: color tables deduplicate outlines by this hash.
                    let outline_hash = bezpath_hash(&outline);
                    layers.push(ProcessedColorLayer {
                        outline,
                        outline_hash,
                        paint,
                    });
                }
                Some(layers.into())
            }
            _ => return None,
        };
        Some(CachedProcessedGlyph {
            color_layers,
            height,
            path_data,
            ttf_path,
            ttf_path_hash,
            width,
        })
    }
}

impl Reader<'_> {
    /// Color layers of a parsed glyph with `path_count` paths; `Some(None)` for a monochrome one.
    pub(super) fn parsed_color_layers(
        &mut self,
        path_count: usize,
    ) -> Option<Option<Arc<[ParsedColorLayer]>>> {
        match self.u8()? {
            0 => return Some(None),
            1 => {}
            _ => return None,
        }
        // A path index (4 bytes), a foreground paint (5 bytes) and a fill rule (1 byte) per layer.
        let count = self.len(10)?;
        let mut layers = Vec::new();
        for _ in 0..count {
            let path_index = self.u32()? as usize;
            (path_index < path_count).then_some(())?;
            let paint = self.paint()?;
            let fill_rule = match self.u8()? {
                0 => FillRule::NonZero,
                1 => FillRule::EvenOdd,
                _ => return None,
            };
            layers.push(ParsedColorLayer {
                path_index,
                paint,
                fill_rule,
            });
        }
        Some(Some(layers.into()))
    }

    fn paint(&mut self) -> Option<ResolvedLayerPaint> {
        Some(match self.u8()? {
            0 => ResolvedLayerPaint::Foreground { alpha: self.f32()? },
            1 => ResolvedLayerPaint::Solid {
                red: self.u8()?,
                green: self.u8()?,
                blue: self.u8()?,
                alpha: self.f32()?,
            },
            _ => return None,
        })
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
