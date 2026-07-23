use std::collections::HashSet;
use std::hash::Hasher;
use std::io::{Cursor, Error, ErrorKind};

use crate::sfnt::{SerializedFontTables, SerializedTable};
use crate::ttf::{Woff2TransformCache, Woff2TransformPayload};
use brotli::enc::backward_references::BrotliEncoderMode;
use brotli::enc::{
    BrotliCompress, BrotliEncoderMaxCompressedSizeMulti, BrotliEncoderParams, Owned, SendAlloc,
    SliceWrapper, StandardAlloc, UnionHasher,
};
use rustc_hash::FxHasher;
use write_fonts::read::tables::compute_checksum;
use write_fonts::read::tables::glyf::{Glyf, Glyph, PointFlags, SimpleGlyph};
use write_fonts::read::tables::head::Head;
use write_fonts::read::tables::loca::Loca;
use write_fonts::read::tables::maxp::Maxp;
use write_fonts::read::types::{GlyphId, Point};
use write_fonts::read::{FontData, FontRead};

const HEADER_SIZE: usize = 48;
const GLYF_ENCODER_VERSION: u8 = 2;
const GLYF_TRANSFORM_HEADER_SIZE: usize = 36;
const KNOWN_TAGS: [[u8; 4]; 63] = [
    *b"cmap", *b"head", *b"hhea", *b"hmtx", *b"maxp", *b"name", *b"OS/2", *b"post", *b"cvt ",
    *b"fpgm", *b"glyf", *b"loca", *b"prep", *b"CFF ", *b"VORG", *b"EBDT", *b"EBLC", *b"gasp",
    *b"hdmx", *b"kern", *b"LTSH", *b"PCLT", *b"VDMX", *b"vhea", *b"vmtx", *b"BASE", *b"GDEF",
    *b"GPOS", *b"GSUB", *b"EBSC", *b"JSTF", *b"MATH", *b"CBDT", *b"CBLC", *b"COLR", *b"CPAL",
    *b"SVG ", *b"sbix", *b"acnt", *b"avar", *b"bdat", *b"bloc", *b"bsln", *b"cvar", *b"fdsc",
    *b"feat", *b"fmtx", *b"fvar", *b"gvar", *b"hsty", *b"just", *b"lcar", *b"mort", *b"morx",
    *b"opbd", *b"prop", *b"trak", *b"Zapf", *b"Silf", *b"Glat", *b"Gloc", *b"Feat", *b"Sill",
];

pub(super) struct PreparedWoff2 {
    directory: Vec<u8>,
    stream: Vec<u8>,
    table_count: u16,
    total_sfnt_size: u32,
}

pub(super) fn encode(
    tables: &SerializedFontTables,
    quality: u8,
    cache: Option<&mut Woff2TransformCache>,
) -> Result<Vec<u8>, Error> {
    let prepared = prepare(tables, cache)?;
    let compressed = compress(&prepared, quality)?;
    assemble(&prepared, &compressed)
}

pub(super) fn prepare(
    tables: &SerializedFontTables,
    mut cache: Option<&mut Woff2TransformCache>,
) -> Result<PreparedWoff2, Error> {
    let mut ordered = Vec::with_capacity(tables.tables().len());
    let mut glyf = None;
    let mut loca = None;
    let mut head = None;
    let mut maxp = None;
    for table in tables.tables() {
        let required = match &table.tag {
            b"glyf" => Some(&mut glyf),
            b"loca" => Some(&mut loca),
            b"head" => Some(&mut head),
            b"maxp" => Some(&mut maxp),
            b"DSIG" => continue,
            _ => None,
        };
        if let Some(required) = required
            && required.replace(table).is_some()
        {
            return Err(invalid_data("duplicate required WOFF2 table"));
        }
        ordered.push(table);
    }
    if ordered.is_empty() {
        return Err(Error::new(ErrorKind::InvalidInput, "WOFF2 requires tables"));
    }
    let (Some(glyf), Some(loca), Some(head), Some(maxp)) = (glyf, loca, head, maxp) else {
        return Err(invalid_data(
            "transformed WOFF2 requires head, maxp, glyf, and loca",
        ));
    };
    ordered.sort_unstable_by_key(|table| table.tag);
    move_loca_after_glyf(&mut ordered);
    let head_table =
        Head::read(FontData::new(&head.bytes)).map_err(|_| invalid_data("invalid head table"))?;
    let maxp_table =
        Maxp::read(FontData::new(&maxp.bytes)).map_err(|_| invalid_data("invalid maxp table"))?;
    let index_format = head_table.index_to_loc_format();
    if !matches!(index_format, 0 | 1) {
        return Err(invalid_data("head indexToLocFormat must be 0 or 1"));
    }
    let num_glyphs = maxp_table.num_glyphs();
    let expected_loca_len = (usize::from(num_glyphs) + 1) * if index_format == 0 { 2 } else { 4 };
    if loca.bytes.len() != expected_loca_len {
        return Err(invalid_data("maxp numGlyphs does not match loca length"));
    }

    let key = transform_cache_key(
        *b"glyf",
        GLYF_ENCODER_VERSION,
        &[
            b"woff2-glyf-loca",
            &glyf.bytes,
            &loca.bytes,
            &index_format.to_be_bytes(),
            &num_glyphs.to_be_bytes(),
        ],
    );
    let payload = if let Some(hit) = cache.as_deref().and_then(|cache| cache.transformed(&key)) {
        hit
    } else {
        let payload = transform_glyf_loca(glyf, loca, index_format, num_glyphs)?;
        if let Some(cache) = cache.as_deref_mut() {
            cache.insert(key, payload.clone());
        }
        payload
    };
    if let Some(cache) = cache {
        cache.retain(&HashSet::from([key]));
    }

    let normalized = NormalizedGlyfLoca {
        glyf_len: payload.normalized_glyf_len,
        glyf_checksum: payload.normalized_glyf_checksum,
        loca_format: payload.normalized_loca_format,
        loca_len: payload.normalized_loca_len,
        loca_checksum: payload.normalized_loca_checksum,
    };
    let normalized_head = normalized_head(&ordered, &normalized)?;
    let mut directory = Vec::new();
    let mut stream = Vec::new();
    for table in &ordered {
        write_directory_entry(
            &mut directory,
            table,
            payload.transformed.len(),
            normalized.glyf_len,
            normalized.loca_len,
        )?;
        match &table.tag {
            b"glyf" => stream.extend_from_slice(&payload.transformed),
            b"loca" => {}
            b"head" => stream.extend_from_slice(&normalized_head),
            _ => stream.extend_from_slice(&table.bytes),
        }
    }

    Ok(PreparedWoff2 {
        directory,
        stream,
        table_count: u16::try_from(ordered.len())
            .map_err(|_| invalid_data("WOFF2 table count exceeds u16"))?,
        total_sfnt_size: total_sfnt_size(&ordered, &normalized)?,
    })
}

fn total_sfnt_size(
    tables: &[&SerializedTable],
    normalized: &NormalizedGlyfLoca,
) -> Result<u32, Error> {
    12_usize
        .checked_add(16 * tables.len())
        .and_then(|size| {
            tables.iter().try_fold(size, |size, table| {
                let length = match &table.tag {
                    b"glyf" => normalized.glyf_len,
                    b"loca" => normalized.loca_len,
                    _ => table.bytes.len(),
                };
                size.checked_add((length + 3) & !3)
            })
        })
        .and_then(|size| u32::try_from(size).ok())
        .ok_or_else(|| invalid_data("SFNT size exceeds u32"))
}

struct NormalizedGlyfLoca {
    glyf_len: usize,
    glyf_checksum: u32,
    loca_format: i16,
    loca_len: usize,
    loca_checksum: u32,
}

fn transform_glyf_loca(
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

        write_normalized_simple_glyph(&glyph, &points, &flags, &mut normalized_glyf)?;
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
            for value in [glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max()] {
                streams.bboxes.extend_from_slice(&value.to_be_bytes());
            }
        }
    }
    offsets.push(normalized_glyf.len());
    let loca_format =
        i16::from(source_index_format == 1 || normalized_glyf.len() / 2 > usize::from(u16::MAX));
    let mut normalized_loca =
        Vec::with_capacity(offsets.len() * if loca_format == 0 { 2 } else { 4 });
    for offset in offsets {
        if loca_format == 0 {
            normalized_loca.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
        } else {
            normalized_loca.extend_from_slice(
                &u32::try_from(offset)
                    .map_err(|_| invalid_data("normalized glyf size exceeds u32"))?
                    .to_be_bytes(),
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

fn write_normalized_simple_glyph(
    glyph: &SimpleGlyph<'_>,
    points: &[Point<i32>],
    source_flags: &[PointFlags],
    output: &mut Vec<u8>,
) -> Result<(), Error> {
    let contour_count = glyph.number_of_contours();
    if contour_count <= 0 {
        return Err(invalid_data("non-empty zero-contour glyph is malformed"));
    }
    output.extend_from_slice(&contour_count.to_be_bytes());
    for value in [glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max()] {
        output.extend_from_slice(&value.to_be_bytes());
    }
    for end_point in glyph.end_pts_of_contours() {
        output.extend_from_slice(&end_point.get().to_be_bytes());
    }
    output.extend_from_slice(&glyph.instruction_length().to_be_bytes());
    output.extend_from_slice(glyph.instructions());

    let mut flags = Vec::with_capacity(points.len());
    let mut x_coordinates = Vec::new();
    let mut y_coordinates = Vec::new();
    let mut last_x = 0_i32;
    let mut last_y = 0_i32;
    for (index, (point, source_flag)) in points.iter().zip(source_flags).enumerate() {
        let dx = point.x - last_x;
        let dy = point.y - last_y;
        let mut flag = u8::from(source_flag.is_on_curve());
        if index == 0 && glyph.has_overlapping_contours() {
            flag |= 1 << 6;
        }
        write_normalized_coordinate(dx, 1 << 1, 1 << 4, &mut flag, &mut x_coordinates)?;
        write_normalized_coordinate(dy, 1 << 2, 1 << 5, &mut flag, &mut y_coordinates)?;
        flags.push(flag);
        last_x = point.x;
        last_y = point.y;
    }
    let mut index = 0;
    while index < flags.len() {
        let flag = flags[index];
        let run_len = flags[index..]
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
    output.extend_from_slice(&x_coordinates);
    output.extend_from_slice(&y_coordinates);
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
        output.extend_from_slice(
            &i16::try_from(delta)
                .map_err(|_| invalid_data("simple glyph coordinate delta exceeds i16"))?
                .to_be_bytes(),
        );
    }
    Ok(())
}

fn normalized_head(
    tables: &[&SerializedTable],
    normalized: &NormalizedGlyfLoca,
) -> Result<Vec<u8>, Error> {
    let head = tables
        .iter()
        .find(|table| table.tag == *b"head")
        .ok_or_else(|| invalid_data("WOFF2 requires a head table"))?;
    if head.bytes.len() < 52 {
        return Err(invalid_data("head table is too short"));
    }

    let mut bytes = head.bytes.clone();
    bytes[8..12].fill(0);
    let flags = u16::from_be_bytes(bytes[16..18].try_into().unwrap()) | (1 << 11);
    bytes[16..18].copy_from_slice(&flags.to_be_bytes());
    bytes[50..52].copy_from_slice(&normalized.loca_format.to_be_bytes());
    let head_checksum = compute_checksum(&bytes);

    let table_count =
        u32::try_from(tables.len()).map_err(|_| invalid_data("WOFF2 table count exceeds u32"))?;
    let max_power = if table_count == 0 {
        0
    } else {
        1 << table_count.ilog2()
    };
    let search_range = max_power * 16;
    let entry_selector = if table_count == 0 {
        0
    } else {
        table_count.ilog2()
    };
    let range_shift = table_count * 16 - search_range;
    let mut checksum = 0x0001_0000_u32
        .wrapping_add((table_count << 16).wrapping_add(search_range))
        .wrapping_add((entry_selector << 16).wrapping_add(range_shift));
    let mut offset = 12_usize
        .checked_add(
            16_usize
                .checked_mul(tables.len())
                .ok_or_else(|| invalid_data("SFNT size overflow"))?,
        )
        .ok_or_else(|| invalid_data("SFNT size overflow"))?;
    for table in tables {
        let (table_checksum, table_len) = match &table.tag {
            b"head" => (head_checksum, table.bytes.len()),
            b"glyf" => (normalized.glyf_checksum, normalized.glyf_len),
            b"loca" => (normalized.loca_checksum, normalized.loca_len),
            _ => (table.checksum, table.bytes.len()),
        };
        checksum = checksum
            .wrapping_add(u32::from_be_bytes(table.tag))
            .wrapping_add(table_checksum)
            .wrapping_add(
                u32::try_from(offset).map_err(|_| invalid_data("SFNT offset exceeds u32"))?,
            )
            .wrapping_add(
                u32::try_from(table_len).map_err(|_| invalid_data("table size exceeds u32"))?,
            )
            .wrapping_add(table_checksum);
        offset = offset
            .checked_add((table_len + 3) & !3)
            .ok_or_else(|| invalid_data("SFNT size overflow"))?;
    }
    bytes[8..12].copy_from_slice(&0xb1b0_afba_u32.wrapping_sub(checksum).to_be_bytes());
    Ok(bytes)
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
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&u16::from(!streams.overlap_bitmap.is_empty()).to_be_bytes());
    output.extend_from_slice(&num_glyphs.to_be_bytes());
    output.extend_from_slice(&(transformed_index_format as u16).to_be_bytes());
    for length in lengths {
        output.extend_from_slice(
            &u32::try_from(length)
                .map_err(|_| invalid_data("transformed glyf stream exceeds u32"))?
                .to_be_bytes(),
        );
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

fn write_255_u16(value: u16, output: &mut Vec<u8>) {
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

fn write_triplet(on_curve: bool, x: i32, y: i32, flags: &mut Vec<u8>, glyphs: &mut Vec<u8>) {
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

fn invalid_data(message: &'static str) -> Error {
    Error::new(ErrorKind::InvalidData, message)
}

pub(super) fn compress(prepared: &PreparedWoff2, quality: u8) -> Result<Vec<u8>, Error> {
    dropbox_brotli_compress(&prepared.stream, quality.min(11))
}

fn assemble(prepared: &PreparedWoff2, compressed: &[u8]) -> Result<Vec<u8>, Error> {
    let compressed_size = u32::try_from(compressed.len())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "compressed stream exceeds u32"))?;
    let unaligned_length = HEADER_SIZE
        .checked_add(prepared.directory.len())
        .and_then(|size| size.checked_add(compressed.len()))
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 size overflow"))?;
    let length = unaligned_length
        .checked_add(3)
        .map(|size| size & !3)
        .and_then(|size| u32::try_from(size).ok())
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "WOFF2 size exceeds u32"))?;

    let mut output = Vec::with_capacity(length as usize);
    output.extend_from_slice(b"wOF2");
    output.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(&prepared.table_count.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&prepared.total_sfnt_size.to_be_bytes());
    output.extend_from_slice(&compressed_size.to_be_bytes());
    output.extend_from_slice(&1_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&[0; 20]);
    output.extend_from_slice(&prepared.directory);
    output.extend_from_slice(compressed);
    output.resize(length as usize, 0);
    Ok(output)
}

fn transform_cache_key(tag: [u8; 4], transform_version: u8, inputs: &[&[u8]]) -> u64 {
    let mut hasher = FxHasher::default();
    hasher.write(&tag);
    hasher.write_u8(transform_version);
    for input in inputs {
        hasher.write(input);
    }
    hasher.finish()
}

fn move_loca_after_glyf(tables: &mut Vec<&SerializedTable>) {
    let loca_index = tables
        .iter()
        .position(|table| table.tag == *b"loca")
        .expect("required loca table must be present");
    let loca = tables.remove(loca_index);
    let glyf_index = tables
        .iter()
        .position(|table| table.tag == *b"glyf")
        .expect("required glyf table must be present");
    tables.insert(glyf_index + 1, loca);
}

fn write_directory_entry(
    output: &mut Vec<u8>,
    table: &SerializedTable,
    transformed_glyf_len: usize,
    normalized_glyf_len: usize,
    normalized_loca_len: usize,
) -> Result<(), Error> {
    let index = KNOWN_TAGS.iter().position(|tag| tag == &table.tag);
    output.push(index.unwrap_or(63) as u8);
    if index.is_none() {
        output.extend_from_slice(&table.tag);
    }
    write_base128(
        u32::try_from(match &table.tag {
            b"glyf" => normalized_glyf_len,
            b"loca" => normalized_loca_len,
            _ => table.bytes.len(),
        })
        .map_err(|_| invalid_data("table size exceeds u32"))?,
        output,
    );
    if table.tag == *b"glyf" {
        write_base128(
            u32::try_from(transformed_glyf_len)
                .map_err(|_| invalid_data("transformed glyf size exceeds u32"))?,
            output,
        );
    } else if table.tag == *b"loca" {
        write_base128(0, output);
    }
    Ok(())
}

fn write_base128(value: u32, output: &mut Vec<u8>) {
    let bits = (32 - value.leading_zeros()).max(1);
    let groups = bits.div_ceil(7);
    for group in (0..groups).rev() {
        let byte = ((value >> (group * 7)) & 0x7f) as u8;
        output.push(byte | u8::from(group != 0) << 7);
    }
}

struct BrotliInput(Vec<u8>);

impl SliceWrapper<u8> for BrotliInput {
    fn slice(&self) -> &[u8] {
        &self.0
    }
}

fn dropbox_brotli_compress(input: &[u8], quality: u8) -> Result<Vec<u8>, Error> {
    let params = BrotliEncoderParams {
        mode: BrotliEncoderMode::BROTLI_MODE_FONT,
        quality: i32::from(quality),
        lgwin: 22,
        size_hint: input.len(),
        ..Default::default()
    };

    if quality < 10 {
        let mut output = Vec::new();
        BrotliCompress(&mut Cursor::new(input), &mut output, &params)?;
        return Ok(output);
    }

    const THREADS: usize = 2;
    let mut output = vec![0; BrotliEncoderMaxCompressedSizeMulti(input.len(), THREADS)];
    let mut allocators = (0..THREADS)
        .map(|_| SendAlloc::new(StandardAlloc::default(), UnionHasher::Uninit))
        .collect::<Vec<_>>();
    let length = brotli::enc::compress_multi(
        &params,
        &mut Owned::new(BrotliInput(input.to_vec())),
        &mut output,
        &mut allocators,
    )
    .map_err(|_| invalid_data("Brotli compression failed"))?;
    output.truncate(length);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::fixture_font_tables;
    use write_fonts::read::tables::compute_checksum;
    use write_fonts::read::{FontRef, TableProvider};

    #[test]
    fn writes_canonical_base128_values() {
        for (value, expected) in [
            (0, &[0][..]),
            (127, &[0x7f]),
            (128, &[0x81, 0]),
            (16_383, &[0xff, 0x7f]),
            (16_384, &[0x81, 0x80, 0]),
            (u32::MAX, &[0x8f, 0xff, 0xff, 0xff, 0x7f]),
        ] {
            let mut actual = Vec::new();
            write_base128(value, &mut actual);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn writes_canonical_255_u16_boundaries() {
        for (value, expected) in [
            (0, &[0][..]),
            (252, &[252]),
            (253, &[255, 0]),
            (505, &[255, 252]),
            (506, &[254, 0]),
            (761, &[254, 255]),
            (762, &[253, 2, 250]),
            (u16::MAX, &[253, 255, 255]),
        ] {
            let mut actual = Vec::new();
            write_255_u16(value, &mut actual);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn triplets_cover_google_branch_boundaries_signs_and_curve_flag() {
        for (on_curve, x, y, encoded_len) in [
            (true, 0, 1279, 1),
            (true, 0, 1280, 3),
            (true, 1279, 0, 1),
            (true, 1280, 0, 3),
            (true, 64, 64, 1),
            (true, 65, 65, 2),
            (true, 768, 768, 2),
            (true, 769, 769, 3),
            (true, 4095, 4095, 3),
            (true, 4096, 4096, 4),
            (false, -64, 64, 1),
            (false, 64, -64, 1),
        ] {
            let mut flags = Vec::new();
            let mut glyphs = Vec::new();
            write_triplet(on_curve, x, y, &mut flags, &mut glyphs);
            assert_eq!(glyphs.len(), encoded_len);
            assert_eq!(decode_triplet(flags[0], &glyphs), (on_curve, x, y));
        }
    }

    #[test]
    fn writes_known_and_unknown_directory_entries() {
        for (tag, expected) in [
            (*b"glyf", &[10, 127, 64][..]),
            (*b"loca", &[11, 127, 0][..]),
        ] {
            let mut output = Vec::new();
            write_directory_entry(
                &mut output,
                &SerializedTable {
                    tag,
                    checksum: 0,
                    bytes: vec![0; 127],
                },
                64,
                127,
                127,
            )
            .unwrap();
            assert_eq!(output, expected);
        }

        let mut output = Vec::new();
        write_directory_entry(
            &mut output,
            &SerializedTable {
                tag: *b"TEST",
                checksum: 0,
                bytes: vec![0; 128],
            },
            64,
            127,
            127,
        )
        .unwrap();
        assert_eq!(output, [63, b'T', b'E', b'S', b'T', 0x81, 0]);
    }

    #[test]
    fn transform_cache_key_hashes_tag_version_and_full_bodies() {
        let body = b"body".as_slice();
        let key = transform_cache_key(*b"glyf", 0, &[body]);
        assert_ne!(key, transform_cache_key(*b"loca", 0, &[body]));
        assert_ne!(key, transform_cache_key(*b"glyf", 1, &[body]));
        assert_ne!(key, transform_cache_key(*b"glyf", 0, &[b"Body"]));

        let first = [0, 0, 0, 1, 0, 0, 0, 2];
        let second = [0, 0, 0, 2, 0, 0, 0, 1];
        assert_eq!(compute_checksum(&first), compute_checksum(&second));
        assert_ne!(
            transform_cache_key(*b"glyf", 0, &[&first]),
            transform_cache_key(*b"glyf", 0, &[&second])
        );
    }

    #[test]
    fn transform_cache_hits_and_prunes_unused_entries() {
        let mut cache = Woff2TransformCache::default();
        cache.insert(1, cached_payload(1));
        cache.insert(2, cached_payload(2));
        assert_eq!(cache.transformed(&1), Some(cached_payload(1)));
        assert_eq!(cache.compile_count, 2);

        cache.retain(&HashSet::from([2]));
        assert_eq!(cache.transformed(&1), None);
        assert_eq!(cache.transformed(&2), Some(cached_payload(2)));
    }

    #[test]
    fn cache_hits_invalidates_and_prunes() {
        let tables = fixture_font_tables();
        let mut cache = Woff2TransformCache::default();
        prepare(&tables, Some(&mut cache)).unwrap();
        assert_eq!(cache.compile_count, 1);

        cache.insert(99, cached_payload(99));
        prepare(&tables, Some(&mut cache)).unwrap();
        assert_eq!(cache.compile_count, 2);
        assert_eq!(cache.transformed(&99), None);

        let mut raw = raw_tables(&tables);
        raw.iter_mut()
            .find(|(tag, _)| tag == b"glyf")
            .unwrap()
            .1
            .push(0);
        let changed = SerializedFontTables::new(raw).unwrap();
        prepare(&changed, Some(&mut cache)).unwrap();
        assert_eq!(cache.compile_count, 3);
    }

    #[test]
    fn rejects_malformed_required_tables() {
        let tables = fixture_font_tables();

        let mut missing_pair = raw_tables(&tables);
        missing_pair.retain(|(tag, _)| tag != b"loca");
        assert!(prepare(&SerializedFontTables::new(missing_pair).unwrap(), None).is_err());

        let mut invalid_format = raw_tables(&tables);
        table_bytes_mut(&mut invalid_format, b"head")[50..52].copy_from_slice(&2_i16.to_be_bytes());
        assert!(prepare(&SerializedFontTables::new(invalid_format).unwrap(), None).is_err());

        let mut wrong_count = raw_tables(&tables);
        table_bytes_mut(&mut wrong_count, b"loca").truncate(2);
        assert!(prepare(&SerializedFontTables::new(wrong_count).unwrap(), None).is_err());

        let mut duplicate = raw_tables(&tables);
        let head = duplicate
            .iter()
            .find(|(tag, _)| tag == b"head")
            .unwrap()
            .1
            .clone();
        duplicate.push((*b"head", head));
        assert!(prepare(&SerializedFontTables::new(duplicate).unwrap(), None).is_err());
    }

    #[test]
    fn rejects_composites_and_nonempty_zero_contours() {
        for contour_count in [-1_i16, 0] {
            let tables = fixture_font_tables();
            let mut raw = raw_tables(&tables);
            let head = table_bytes_mut(&mut raw, b"head").clone();
            let loca = table_bytes_mut(&mut raw, b"loca").clone();
            let offsets = loca_offsets(&loca, i16::from_be_bytes(head[50..52].try_into().unwrap()));
            let start = offsets.windows(2).find(|pair| pair[0] != pair[1]).unwrap()[0];
            table_bytes_mut(&mut raw, b"glyf")[start..start + 2]
                .copy_from_slice(&contour_count.to_be_bytes());
            let malformed = SerializedFontTables::new(raw).unwrap();
            assert!(prepare(&malformed, None).is_err());
        }
    }

    #[test]
    fn split_preparation_compression_and_assembly_matches_encode() {
        let tables = fixture_font_tables();
        let prepared = prepare(&tables, None).unwrap();
        let compressed = compress(&prepared, 11).unwrap();
        assert_eq!(
            assemble(&prepared, &compressed).unwrap(),
            encode(&tables, 11, None).unwrap()
        );
    }

    #[test]
    fn woff2_reference_decodes_semantically() {
        let tables = fixture_font_tables();
        let output = encode(&tables, 11, None).unwrap();
        let decoded =
            ::woff::version2::decompress(&output).expect("transformed WOFF2 should decode");
        assert_same_semantics(tables.ttf(), &decoded);

        let entries = directory_entries(&output);
        let glyf = entries.iter().find(|entry| entry.0 == *b"glyf").unwrap();
        let loca = entries.iter().find(|entry| entry.0 == *b"loca").unwrap();
        assert_eq!(glyf.1, 0);
        assert!(glyf.3.unwrap() > 0);
        assert_eq!(loca.1, 0);
        assert_eq!(loca.3, Some(0));
        assert_eq!(
            entries.iter().position(|entry| entry.0 == *b"loca"),
            entries
                .iter()
                .position(|entry| entry.0 == *b"glyf")
                .map(|index| index + 1)
        );
    }

    #[test]
    fn promotes_short_loca_when_normalization_overflows() {
        const GLYPH_COUNT: u16 = 6_554;
        const GLYPH_SIZE: usize = 18;

        let mut glyph = vec![0, 1];
        glyph.extend_from_slice(&[0; 8]);
        glyph.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0x31, 0]);
        assert_eq!(glyph.len(), GLYPH_SIZE);

        let tables = fixture_font_tables();
        let mut raw = raw_tables(&tables);
        table_bytes_mut(&mut raw, b"head")[50..52].copy_from_slice(&0_i16.to_be_bytes());
        table_bytes_mut(&mut raw, b"maxp")[4..6].copy_from_slice(&GLYPH_COUNT.to_be_bytes());
        *table_bytes_mut(&mut raw, b"glyf") = glyph.repeat(usize::from(GLYPH_COUNT));
        *table_bytes_mut(&mut raw, b"loca") = (0..=GLYPH_COUNT)
            .flat_map(|index| ((usize::from(index) * GLYPH_SIZE / 2) as u16).to_be_bytes())
            .collect();
        let tables = SerializedFontTables::new(raw).unwrap();

        let output = encode(&tables, 0, None).unwrap();
        let decoded =
            ::woff::version2::decompress(&output).expect("transformed WOFF2 should decode");
        let head = sfnt_table(&decoded, b"head").unwrap();
        let loca = sfnt_table(&decoded, b"loca").unwrap();
        assert_eq!(i16::from_be_bytes(head[50..52].try_into().unwrap()), 1);
        assert_eq!(loca.len(), (usize::from(GLYPH_COUNT) + 1) * 4);
    }

    #[test]
    fn removes_dsig_and_marks_the_font_as_transformed() {
        let tables = fixture_font_tables();
        let mut raw_tables = tables
            .tables()
            .iter()
            .map(|table| (table.tag, table.bytes.clone()))
            .collect::<Vec<_>>();
        raw_tables.push((*b"DSIG", vec![0; 8]));
        let tables = SerializedFontTables::new(raw_tables).unwrap();

        let output = encode(&tables, 11, None).unwrap();
        assert_eq!(
            u16::from_be_bytes(output[12..14].try_into().unwrap()) as usize,
            tables.tables().len() - 1
        );
        let decoded = ::woff::version2::decompress(&output).unwrap();
        assert!(sfnt_table(&decoded, b"DSIG").is_none());
        let head = sfnt_table(&decoded, b"head").unwrap();
        assert_ne!(
            u16::from_be_bytes(head[16..18].try_into().unwrap()) & (1 << 11),
            0
        );
    }

    fn assert_same_semantics(source: &[u8], decoded: &[u8]) {
        let source = FontRef::new(source).expect("source should parse");
        let decoded = FontRef::new(decoded).expect("decoded font should parse");
        assert_eq!(
            source.maxp().unwrap().num_glyphs(),
            decoded.maxp().unwrap().num_glyphs()
        );
        assert_eq!(
            source.cmap().unwrap().map_codepoint(0xe001_u32),
            decoded.cmap().unwrap().map_codepoint(0xe001_u32)
        );
        assert_eq!(
            source.hhea().unwrap().ascender(),
            decoded.hhea().unwrap().ascender()
        );
        assert_eq!(
            source.name().unwrap().offset_data().as_bytes(),
            decoded.name().unwrap().offset_data().as_bytes()
        );
    }

    fn read_base128(input: &[u8], offset: &mut usize) -> u32 {
        let mut value = 0_u32;
        loop {
            let byte = input[*offset];
            *offset += 1;
            value = (value << 7) | u32::from(byte & 0x7f);
            if byte & 0x80 == 0 {
                return value;
            }
        }
    }

    fn decode_triplet(flag: u8, input: &[u8]) -> (bool, i32, i32) {
        let on_curve = flag & 0x80 == 0;
        let flag = flag & 0x7f;
        let with_sign = |flag: u8, value: i32| if flag & 1 != 0 { value } else { -value };
        let (x, y) = if flag < 10 {
            (
                0,
                with_sign(flag, (i32::from(flag & 14) << 7) + i32::from(input[0])),
            )
        } else if flag < 20 {
            (
                with_sign(
                    flag,
                    (i32::from((flag - 10) & 14) << 7) + i32::from(input[0]),
                ),
                0,
            )
        } else if flag < 84 {
            let b0 = flag - 20;
            (
                with_sign(flag, 1 + i32::from(b0 & 0x30) + i32::from(input[0] >> 4)),
                with_sign(
                    flag >> 1,
                    1 + i32::from((b0 & 0x0c) << 2) + i32::from(input[0] & 0x0f),
                ),
            )
        } else if flag < 120 {
            let b0 = flag - 84;
            (
                with_sign(flag, 1 + i32::from(b0 / 12) * 256 + i32::from(input[0])),
                with_sign(
                    flag >> 1,
                    1 + i32::from((b0 % 12) >> 2) * 256 + i32::from(input[1]),
                ),
            )
        } else if flag < 124 {
            (
                with_sign(flag, i32::from(input[0]) * 16 + i32::from(input[1] >> 4)),
                with_sign(
                    flag >> 1,
                    i32::from(input[1] & 0x0f) * 256 + i32::from(input[2]),
                ),
            )
        } else {
            (
                with_sign(flag, i32::from(u16::from_be_bytes([input[0], input[1]]))),
                with_sign(
                    flag >> 1,
                    i32::from(u16::from_be_bytes([input[2], input[3]])),
                ),
            )
        };
        (on_curve, x, y)
    }

    fn raw_tables(tables: &SerializedFontTables) -> Vec<([u8; 4], Vec<u8>)> {
        tables
            .tables()
            .iter()
            .map(|table| (table.tag, table.bytes.clone()))
            .collect()
    }

    fn cached_payload(byte: u8) -> Woff2TransformPayload {
        Woff2TransformPayload {
            transformed: vec![byte],
            normalized_glyf_len: usize::from(byte),
            normalized_glyf_checksum: u32::from(byte),
            normalized_loca_format: 0,
            normalized_loca_len: usize::from(byte),
            normalized_loca_checksum: u32::from(byte),
        }
    }

    fn table_bytes_mut<'a>(tables: &'a mut [([u8; 4], Vec<u8>)], tag: &[u8; 4]) -> &'a mut Vec<u8> {
        &mut tables.iter_mut().find(|table| &table.0 == tag).unwrap().1
    }

    fn loca_offsets(loca: &[u8], format: i16) -> Vec<usize> {
        if format == 0 {
            loca.chunks_exact(2)
                .map(|bytes| usize::from(u16::from_be_bytes(bytes.try_into().unwrap())) * 2)
                .collect()
        } else {
            loca.chunks_exact(4)
                .map(|bytes| u32::from_be_bytes(bytes.try_into().unwrap()) as usize)
                .collect()
        }
    }

    fn directory_entries(output: &[u8]) -> Vec<([u8; 4], u8, u32, Option<u32>)> {
        let count = u16::from_be_bytes(output[12..14].try_into().unwrap()) as usize;
        let mut offset = HEADER_SIZE;
        (0..count)
            .map(|_| {
                let flags = output[offset];
                offset += 1;
                let index = usize::from(flags & 0x3f);
                let tag = if index == 63 {
                    let tag = output[offset..offset + 4].try_into().unwrap();
                    offset += 4;
                    tag
                } else {
                    KNOWN_TAGS[index]
                };
                let length = read_base128(output, &mut offset);
                let transformed_length =
                    matches!(&tag, b"glyf" | b"loca").then(|| read_base128(output, &mut offset));
                (tag, flags >> 6, length, transformed_length)
            })
            .collect()
    }

    fn sfnt_table<'a>(sfnt: &'a [u8], wanted: &[u8; 4]) -> Option<&'a [u8]> {
        let count = u16::from_be_bytes(sfnt[4..6].try_into().ok()?) as usize;
        (0..count).find_map(|index| {
            let entry = 12 + index * 16;
            if &sfnt[entry..entry + 4] != wanted {
                return None;
            }
            let offset = u32::from_be_bytes(sfnt[entry + 8..entry + 12].try_into().ok()?) as usize;
            let length = u32::from_be_bytes(sfnt[entry + 12..entry + 16].try_into().ok()?) as usize;
            sfnt.get(offset..offset + length)
        })
    }
}
