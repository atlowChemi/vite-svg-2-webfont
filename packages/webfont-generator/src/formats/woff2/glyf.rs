use std::hash::Hasher;
use std::io::{Error, ErrorKind};

use rustc_hash::FxHasher;
use write_fonts::read::tables::compute_checksum;
use write_fonts::read::tables::glyf::{Glyf, Glyph, PointFlags, SimpleGlyph};
use write_fonts::read::tables::loca::Loca;
use write_fonts::read::types::{GlyphId, Point};
use write_fonts::read::{FontData, FontRead};

use crate::byte_helpers::BigEndian;
use crate::sfnt::SerializedTable;

use super::Woff2TransformPayload;

pub(super) const GLYF_ENCODER_VERSION: u8 = 2;
const GLYF_TRANSFORM_HEADER_SIZE: usize = 36;

pub(super) struct NormalizedGlyfLoca {
    pub(super) glyf_len: usize,
    pub(super) glyf_checksum: u32,
    pub(super) loca_format: i16,
    pub(super) loca_len: usize,
    pub(super) loca_checksum: u32,
}

pub(super) fn transform_glyf_loca(
    glyf_table: &SerializedTable,
    loca_table: &SerializedTable,
    source_index_format: i16,
    num_glyphs: u16,
) -> Result<Woff2TransformPayload, Error> {
    let glyf = Glyf::read(FontData::new(&glyf_table.bytes))
        .map_err(|_| invalid_data("invalid glyf table"))?;
    let loca = Loca::read(FontData::new(&loca_table.bytes), source_index_format == 1)
        .map_err(|_| invalid_data("invalid loca table"))?;
    if loca.len() != usize::from(num_glyphs) {
        return Err(invalid_data("maxp numGlyphs does not match loca length"));
    }
    if !loca.all_offsets_are_ascending()
        || loca.get_raw(usize::from(num_glyphs)).unwrap_or(u32::MAX) as usize
            > glyf_table.bytes.len()
    {
        return Err(invalid_data(
            "loca offsets must be ascending and within glyf",
        ));
    }

    let mut normalized_glyf = Vec::new();
    let mut offsets = Vec::with_capacity(usize::from(num_glyphs) + 1);
    let mut streams = GlyfStreams {
        bbox_bitmap: vec![0; usize::from(num_glyphs).div_ceil(32) * 4],
        ..Default::default()
    };
    let mut points = Vec::new();
    let mut flags = Vec::new();
    let mut normalized_scratch = NormalizedGlyphScratch::default();
    for glyph_id in 0..num_glyphs {
        offsets.push(normalized_glyf.len());
        let glyph = loca
            .get_glyf(GlyphId::new(u32::from(glyph_id)), &glyf)
            .map_err(|_| invalid_data("invalid glyph record"))?;
        let Some(glyph) = glyph else {
            streams.contours.extend_from_slice(&0_u16.to_be_bytes());
            continue;
        };
        let Glyph::Simple(glyph) = glyph else {
            return Err(invalid_data("composite glyphs are not supported"));
        };
        let contour_count = glyph.number_of_contours();
        if contour_count <= 0 {
            return Err(invalid_data("non-empty zero-contour glyph is malformed"));
        }
        let end_points = glyph.end_pts_of_contours();
        if end_points.len() != contour_count as usize
            || end_points
                .windows(2)
                .any(|pair| pair[0].get() >= pair[1].get())
        {
            return Err(invalid_data("invalid simple glyph contour endpoints"));
        }
        let point_count = glyph.num_points();
        points.resize(point_count, Point::<i32>::default());
        flags.resize(point_count, PointFlags::default());
        glyph
            .read_points_fast(&mut points, &mut flags)
            .map_err(|_| invalid_data("invalid simple glyph coordinates"))?;

        write_normalized_simple_glyph(
            &glyph,
            &points,
            &flags,
            &mut normalized_scratch,
            &mut normalized_glyf,
        )?;
        normalized_glyf.resize((normalized_glyf.len() + 3) & !3, 0);

        streams
            .contours
            .extend_from_slice(&(contour_count as u16).to_be_bytes());
        let mut contour_start = 0_usize;
        for end in end_points {
            let contour_end = usize::from(end.get()) + 1;
            write_255_u16(
                u16::try_from(contour_end - contour_start)
                    .map_err(|_| invalid_data("simple glyph contour is too large"))?,
                &mut streams.points,
            );
            contour_start = contour_end;
        }

        let mut last_x = 0;
        let mut last_y = 0;
        for (point, flag) in points.iter().zip(&flags) {
            write_triplet(
                flag.is_on_curve(),
                point.x - last_x,
                point.y - last_y,
                &mut streams.flags,
                &mut streams.glyphs,
            );
            last_x = point.x;
            last_y = point.y;
        }
        write_255_u16(glyph.instruction_length(), &mut streams.glyphs);
        streams.instructions.extend_from_slice(glyph.instructions());

        if glyph.has_overlapping_contours() {
            if streams.overlap_bitmap.is_empty() {
                streams
                    .overlap_bitmap
                    .resize(usize::from(num_glyphs).div_ceil(8), 0);
            }
            set_bitmap_bit(&mut streams.overlap_bitmap, usize::from(glyph_id));
        }
        let computed_bbox = points.iter().fold(
            (points[0].x, points[0].y, points[0].x, points[0].y),
            |(x_min, y_min, x_max, y_max), point| {
                (
                    x_min.min(point.x),
                    y_min.min(point.y),
                    x_max.max(point.x),
                    y_max.max(point.y),
                )
            },
        );
        if computed_bbox
            != (
                i32::from(glyph.x_min()),
                i32::from(glyph.y_min()),
                i32::from(glyph.x_max()),
                i32::from(glyph.y_max()),
            )
        {
            set_bitmap_bit(&mut streams.bbox_bitmap, usize::from(glyph_id));
            let mut writer = BigEndian::new(&mut streams.bboxes);
            for value in [glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max()] {
                writer.push_i16(value);
            }
        }
    }
    offsets.push(normalized_glyf.len());
    let loca_format =
        i16::from(source_index_format == 1 || normalized_glyf.len() / 2 > usize::from(u16::MAX));
    let mut normalized_loca =
        Vec::with_capacity(offsets.len() * if loca_format == 0 { 2 } else { 4 });
    let mut writer = BigEndian::new(&mut normalized_loca);
    for offset in offsets {
        if loca_format == 0 {
            writer.push_u16((offset / 2) as u16);
        } else {
            writer.push_u32(
                u32::try_from(offset)
                    .map_err(|_| invalid_data("normalized glyf size exceeds u32"))?,
            );
        }
    }
    Ok(Woff2TransformPayload {
        transformed: finish_glyf_transform(streams, num_glyphs, loca_format)?,
        normalized_glyf_len: normalized_glyf.len(),
        normalized_glyf_checksum: compute_checksum(&normalized_glyf),
        normalized_loca_format: loca_format,
        normalized_loca_len: normalized_loca.len(),
        normalized_loca_checksum: compute_checksum(&normalized_loca),
    })
}

#[derive(Default)]
struct NormalizedGlyphScratch {
    flags: Vec<u8>,
    x_coordinates: Vec<u8>,
    y_coordinates: Vec<u8>,
}

fn write_normalized_simple_glyph(
    glyph: &SimpleGlyph<'_>,
    points: &[Point<i32>],
    source_flags: &[PointFlags],
    scratch: &mut NormalizedGlyphScratch,
    output: &mut Vec<u8>,
) -> Result<(), Error> {
    let contour_count = glyph.number_of_contours();
    if contour_count <= 0 {
        return Err(invalid_data("non-empty zero-contour glyph is malformed"));
    }
    {
        let mut writer = BigEndian::new(&mut *output);
        writer.push_i16(contour_count);
        for value in [glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max()] {
            writer.push_i16(value);
        }
        for end_point in glyph.end_pts_of_contours() {
            writer.push_u16(end_point.get());
        }
        writer.push_u16(glyph.instruction_length());
    }
    output.extend_from_slice(glyph.instructions());

    scratch.flags.clear();
    scratch.flags.reserve(points.len());
    scratch.x_coordinates.clear();
    scratch.y_coordinates.clear();
    let mut last_x = 0_i32;
    let mut last_y = 0_i32;
    for (index, (point, source_flag)) in points.iter().zip(source_flags).enumerate() {
        let dx = point.x - last_x;
        let dy = point.y - last_y;
        let mut flag = u8::from(source_flag.is_on_curve());
        if index == 0 && glyph.has_overlapping_contours() {
            flag |= 1 << 6;
        }
        write_normalized_coordinate(dx, 1 << 1, 1 << 4, &mut flag, &mut scratch.x_coordinates)?;
        write_normalized_coordinate(dy, 1 << 2, 1 << 5, &mut flag, &mut scratch.y_coordinates)?;
        scratch.flags.push(flag);
        last_x = point.x;
        last_y = point.y;
    }
    let mut index = 0;
    while index < scratch.flags.len() {
        let flag = scratch.flags[index];
        let run_len = scratch.flags[index..]
            .iter()
            .take(256)
            .take_while(|next| **next == flag)
            .count();
        output.push(flag | u8::from(run_len > 1) << 3);
        if run_len > 1 {
            output.push((run_len - 1) as u8);
        }
        index += run_len;
    }
    output.extend_from_slice(&scratch.x_coordinates);
    output.extend_from_slice(&scratch.y_coordinates);
    Ok(())
}

fn write_normalized_coordinate(
    delta: i32,
    short_bit: u8,
    same_or_positive_bit: u8,
    flag: &mut u8,
    output: &mut Vec<u8>,
) -> Result<(), Error> {
    if delta == 0 {
        *flag |= same_or_positive_bit;
    } else if (-255..=255).contains(&delta) {
        *flag |= short_bit;
        if delta > 0 {
            *flag |= same_or_positive_bit;
        }
        output.push(delta.unsigned_abs() as u8);
    } else {
        BigEndian::new(output).push_i16(
            i16::try_from(delta)
                .map_err(|_| invalid_data("simple glyph coordinate delta exceeds i16"))?,
        );
    }
    Ok(())
}

#[derive(Default)]
struct GlyfStreams {
    contours: Vec<u8>,
    points: Vec<u8>,
    flags: Vec<u8>,
    glyphs: Vec<u8>,
    composites: Vec<u8>,
    bbox_bitmap: Vec<u8>,
    bboxes: Vec<u8>,
    instructions: Vec<u8>,
    overlap_bitmap: Vec<u8>,
}

fn finish_glyf_transform(
    streams: GlyfStreams,
    num_glyphs: u16,
    transformed_index_format: i16,
) -> Result<Vec<u8>, Error> {
    let bbox_len = streams
        .bbox_bitmap
        .len()
        .checked_add(streams.bboxes.len())
        .ok_or_else(|| invalid_data("transformed glyf size overflow"))?;
    let lengths = [
        streams.contours.len(),
        streams.points.len(),
        streams.flags.len(),
        streams.glyphs.len(),
        streams.composites.len(),
        bbox_len,
        streams.instructions.len(),
    ];
    let payload_len = lengths
        .iter()
        .try_fold(GLYF_TRANSFORM_HEADER_SIZE, |size, length| {
            size.checked_add(*length)
        })
        .and_then(|size| size.checked_add(streams.overlap_bitmap.len()))
        .ok_or_else(|| invalid_data("transformed glyf size overflow"))?;
    let mut output = Vec::with_capacity(payload_len);
    {
        let mut writer = BigEndian::new(&mut output);
        writer.push_u16(0);
        writer.push_u16(u16::from(!streams.overlap_bitmap.is_empty()));
        writer.push_u16(num_glyphs);
        writer.push_i16(transformed_index_format);
        for length in lengths {
            writer.push_u32(
                u32::try_from(length)
                    .map_err(|_| invalid_data("transformed glyf stream exceeds u32"))?,
            );
        }
    }
    output.extend_from_slice(&streams.contours);
    output.extend_from_slice(&streams.points);
    output.extend_from_slice(&streams.flags);
    output.extend_from_slice(&streams.glyphs);
    output.extend_from_slice(&streams.composites);
    output.extend_from_slice(&streams.bbox_bitmap);
    output.extend_from_slice(&streams.bboxes);
    output.extend_from_slice(&streams.instructions);
    output.extend_from_slice(&streams.overlap_bitmap);
    Ok(output)
}

fn set_bitmap_bit(bitmap: &mut [u8], glyph_id: usize) {
    bitmap[glyph_id >> 3] |= 0x80 >> (glyph_id & 7);
}

pub(super) fn write_255_u16(value: u16, output: &mut Vec<u8>) {
    match value {
        0..=252 => output.push(value as u8),
        253..=505 => output.extend_from_slice(&[255, (value - 253) as u8]),
        506..=761 => output.extend_from_slice(&[254, (value - 506) as u8]),
        _ => {
            output.push(253);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

pub(super) fn write_triplet(
    on_curve: bool,
    x: i32,
    y: i32,
    flags: &mut Vec<u8>,
    glyphs: &mut Vec<u8>,
) {
    let abs_x = x.unsigned_abs();
    let abs_y = y.unsigned_abs();
    let on_curve_bit = if on_curve { 0 } else { 128 };
    let x_sign_bit = u8::from(x >= 0);
    let y_sign_bit = u8::from(y >= 0);
    let signs = x_sign_bit + 2 * y_sign_bit;
    if x == 0 && abs_y < 1280 {
        flags.push(on_curve_bit + ((abs_y & 0xf00) >> 7) as u8 + y_sign_bit);
        glyphs.push(abs_y as u8);
    } else if y == 0 && abs_x < 1280 {
        flags.push(on_curve_bit + 10 + ((abs_x & 0xf00) >> 7) as u8 + x_sign_bit);
        glyphs.push(abs_x as u8);
    } else if abs_x < 65 && abs_y < 65 {
        flags.push(
            on_curve_bit
                + 20
                + ((abs_x.wrapping_sub(1) & 0x30) as u8)
                + (((abs_y.wrapping_sub(1) & 0x30) >> 2) as u8)
                + signs,
        );
        glyphs.push((((abs_x.wrapping_sub(1) & 0xf) << 4) | (abs_y.wrapping_sub(1) & 0xf)) as u8);
    } else if abs_x < 769 && abs_y < 769 {
        flags.push(
            on_curve_bit
                + 84
                + (12 * ((abs_x.wrapping_sub(1) & 0x300) >> 8)) as u8
                + ((abs_y.wrapping_sub(1) & 0x300) >> 6) as u8
                + signs,
        );
        glyphs.extend_from_slice(&[abs_x.wrapping_sub(1) as u8, abs_y.wrapping_sub(1) as u8]);
    } else if abs_x < 4096 && abs_y < 4096 {
        flags.push(on_curve_bit + 120 + signs);
        glyphs.extend_from_slice(&[
            (abs_x >> 4) as u8,
            (((abs_x & 0xf) << 4) | (abs_y >> 8)) as u8,
            abs_y as u8,
        ]);
    } else {
        flags.push(on_curve_bit + 124 + signs);
        glyphs.extend_from_slice(&[
            (abs_x >> 8) as u8,
            abs_x as u8,
            (abs_y >> 8) as u8,
            abs_y as u8,
        ]);
    }
}

pub(super) fn transform_cache_key(tag: [u8; 4], transform_version: u8, inputs: &[&[u8]]) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(&tag);
    hasher.write_u8(transform_version);
    for input in inputs {
        hasher.write(input);
    }
    hasher.finish()
}

pub(super) fn invalid_data(message: &'static str) -> Error {
    Error::new(ErrorKind::InvalidData, message)
}
