use std::io;

use write_fonts::FontBuilder;
use write_fonts::types::Tag;

use crate::formats::woff2::Woff2TransformCache;
use crate::input::{
    LoadedSvgFile, finalize_generate_webfonts_options, resolve_generate_webfonts_options,
};
use crate::pipeline::build_font_outputs;
use crate::sfnt::{self, SerializedFontTables};
use crate::svg::types::{GlyphCache, ParsedGlyph, PreparedSvgFont};
use crate::svg::{
    finalize_glyphs, parse_glyphs, prepare_svg_font, prepare_svg_font_incremental,
    svg_options_from_options,
};
use crate::types::ResolvedGenerateWebfontsOptions;
use crate::{GenerateWebfontsOptions, GenerateWebfontsResult};

/// Source fixture used by Rust benchmarks without exposing generator internals.
#[derive(Clone)]
pub struct BenchSvgSource {
    pub path: String,
    pub glyph_name: String,
    pub contents: String,
}

/// Opaque parsed-glyph cache used by incremental SVG prepare benchmarks.
#[derive(Clone, Default)]
pub struct BenchGlyphCache(GlyphCache);

/// Opaque loaded sources and resolved options for incremental SVG preparation benchmarks.
pub struct BenchSvgPrepareInput {
    options: ResolvedGenerateWebfontsOptions,
    sources: Vec<LoadedSvgFile>,
}

/// Opaque parsed glyph set used to isolate parse and finalize stages.
#[derive(Clone)]
pub struct BenchParsedGlyphs(Vec<ParsedGlyph>);

/// Opaque prepared SVG font used to isolate font-output generation stages.
#[derive(Clone)]
pub struct BenchPreparedSvgFont(PreparedSvgFont);

/// Opaque serialized TTF table set used to isolate SFNT assembly costs.
#[derive(Clone)]
pub struct BenchSerializedFontTables(SerializedFontTables);

/// Opaque WOFF2 transform cache used by preparation benchmarks.
#[derive(Clone, Default)]
pub struct BenchWoff2TransformCache(Woff2TransformCache);

/// Opaque prepared WOFF2 directory and table stream.
pub struct BenchPreparedWoff2(crate::formats::woff2::PreparedWoff2);

fn load_sources(sources: &[BenchSvgSource]) -> Vec<LoadedSvgFile> {
    sources
        .iter()
        .map(|source| LoadedSvgFile {
            contents: source.contents.clone().into(),
            glyph_name: source.glyph_name.clone(),
            path: source.path.clone(),
        })
        .collect()
}

fn resolve(
    options: GenerateWebfontsOptions,
    sources: &[LoadedSvgFile],
) -> io::Result<ResolvedGenerateWebfontsOptions> {
    let mut options = resolve_generate_webfonts_options(options)?;
    finalize_generate_webfonts_options(&mut options, sources)?;
    Ok(options)
}

/// Run the SVG parse+process preparation path and return the number of prepared glyphs.
pub fn svg_prepare_input(
    options: GenerateWebfontsOptions,
    sources: &[BenchSvgSource],
) -> io::Result<BenchSvgPrepareInput> {
    let sources = load_sources(sources);
    let options = resolve(options, &sources)?;
    Ok(BenchSvgPrepareInput { options, sources })
}

/// Run the full SVG preparation path and return the number of prepared glyphs.
pub fn prepare_svg_full(input: &BenchSvgPrepareInput) -> io::Result<usize> {
    let svg_options = svg_options_from_options(&input.options);
    let prepared = prepare_svg_font(&svg_options, &input.sources)?;
    Ok(prepared.processed_glyphs.len())
}

/// Parse SVG glyph geometry without running set-wide finalization/processing.
pub fn parse_svg_only(
    options: GenerateWebfontsOptions,
    sources: &[BenchSvgSource],
) -> io::Result<BenchParsedGlyphs> {
    let sources = load_sources(sources);
    let options = resolve(options, &sources)?;
    let svg_options = svg_options_from_options(&options);
    parse_glyphs(&svg_options, &sources).map(BenchParsedGlyphs)
}

/// Run set-wide SVG finalization/processing from already parsed glyph geometry.
pub fn finalize_svg_only(
    options: GenerateWebfontsOptions,
    sources: &[BenchSvgSource],
    parsed: BenchParsedGlyphs,
) -> io::Result<BenchPreparedSvgFont> {
    let sources = load_sources(sources);
    let options = resolve(options, &sources)?;
    let svg_options = svg_options_from_options(&options);
    finalize_glyphs(&svg_options, parsed.0).map(BenchPreparedSvgFont)
}

/// Build requested font outputs from an already prepared SVG font and return total output bytes.
pub fn build_outputs_only(
    options: GenerateWebfontsOptions,
    sources: &[BenchSvgSource],
    prepared: &BenchPreparedSvgFont,
) -> io::Result<usize> {
    let sources = load_sources(sources);
    let options = resolve(options, &sources)?;
    let svg_options = svg_options_from_options(&options);
    let fonts = build_font_outputs(&options, &svg_options, &prepared.0, None)?;
    Ok(fonts.svg_font.as_ref().map_or(0, |v| v.len())
        + fonts.ttf_font.as_ref().map_or(0, |v| v.len())
        + fonts.woff_font.as_ref().map_or(0, |v| v.len())
        + fonts.woff2_font.as_ref().map_or(0, |v| v.len())
        + fonts.eot_font.as_ref().map_or(0, |v| v.len()))
}

/// Build serialized TTF tables from an already prepared SVG font.
pub fn build_serialized_ttf_tables(
    options: GenerateWebfontsOptions,
    sources: &[BenchSvgSource],
    prepared: &BenchPreparedSvgFont,
) -> io::Result<BenchSerializedFontTables> {
    let sources = load_sources(sources);
    let options = resolve(options, &sources)?;
    sfnt::build(
        sfnt::ttf_options_from_options(&options),
        &prepared.0.processed_glyphs,
        None,
    )
    .map(BenchSerializedFontTables)
}

/// Rebuild serialized table metadata from already dumped table bytes.
pub fn rewrap_serialized_ttf_tables(
    tables: &BenchSerializedFontTables,
) -> io::Result<BenchSerializedFontTables> {
    SerializedFontTables::new(tables.0.clone_raw_tables()).map(BenchSerializedFontTables)
}

/// Assemble final TTF bytes with the current serialized-table SFNT writer, without cache reuse.
pub fn serialized_ttf_uncached(tables: &BenchSerializedFontTables) -> Vec<u8> {
    tables.0.uncached_ttf()
}

/// Encode serialized tables with the internal WOFF2 encoder.
pub fn internal_woff2(tables: &BenchSerializedFontTables, quality: u8) -> io::Result<Vec<u8>> {
    crate::formats::woff2::tables_to_woff2(&tables.0, quality, None)
}

/// Prepare the internal WOFF2 stream without Brotli compression.
pub fn prepare_internal_woff2(
    tables: &BenchSerializedFontTables,
    cache: &mut BenchWoff2TransformCache,
) -> io::Result<BenchPreparedWoff2> {
    crate::formats::woff2::prepare_woff2(&tables.0, &mut cache.0).map(BenchPreparedWoff2)
}

/// Brotli-compress an already prepared internal WOFF2 stream and return its byte length.
pub fn compress_prepared_internal_woff2(
    prepared: &BenchPreparedWoff2,
    quality: u8,
) -> io::Result<usize> {
    crate::formats::woff2::compress_prepared_woff2(&prepared.0, quality)
}

/// Assemble final TTF bytes with write-fonts FontBuilder from the same serialized tables.
pub fn fontbuilder_ttf(tables: &BenchSerializedFontTables) -> Vec<u8> {
    let mut builder = FontBuilder::new();
    for table in tables.0.tables() {
        builder.add_raw(Tag::new(&table.tag), table.bytes.as_slice());
    }
    builder.build()
}

/// Clear retained WOFF1 payloads so benchmarks can compare warm vs cold compression cache.
pub fn clear_woff1_payload_cache(result: &mut GenerateWebfontsResult) {
    if let Some(state) = result.regeneration_state.lock().unwrap().as_mut()
        && let Some(cache) = state.ttf_cache.as_mut()
    {
        cache.woff1_payloads.clear();
    }
}

/// Run the incremental SVG preparation path and return the number of prepared glyphs.
pub fn prepare_svg_incremental(
    input: &BenchSvgPrepareInput,
    cache: &mut BenchGlyphCache,
) -> io::Result<usize> {
    let svg_options = svg_options_from_options(&input.options);
    let prepared = prepare_svg_font_incremental(&svg_options, &input.sources, &mut cache.0)?;
    Ok(prepared.processed_glyphs.len())
}
