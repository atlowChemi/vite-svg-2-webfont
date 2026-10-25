use std::collections::HashMap;
use std::sync::Arc;

use rayon::join;

#[cfg(feature = "fs")]
use crate::cache::DiskCache;
use crate::cache::{CacheState, CachedFont};
use crate::formats::woff1::Woff1PayloadCache;
use crate::formats::woff2::Woff2TransformCache;
use crate::formats::{eot, woff1, woff2};
#[cfg(feature = "fs")]
use crate::incremental::RegenerationState;
#[cfg(feature = "fs")]
use crate::input::ResolvedFileGenerationOptions;
use crate::input::{
    LoadedSvgSource, ResolvedGenerationOptions, ResolvedVariants, VariantFamilySources,
};
use crate::result::FontOutputs;
#[cfg(feature = "fs")]
use crate::result::GenerateWebfontsResult;
use crate::sfnt;
use crate::sfnt::CachedCompiledGlyph;
use crate::svg::types::{GlyphCache, PreparedSvgFont, SvgOptions};
use crate::svg::{
    VariantGlyphCache, build_svg_font, prepare_svg_font_incremental,
    prepare_variant_svg_family_cached,
};
#[cfg(feature = "fs")]
use crate::svg::{prepare_svg_font, svg_options_from_options};
use crate::types::FontType;
#[cfg(feature = "fs")]
use crate::{prepare_variant_family, resolve_content_variant_sources};

#[cfg(all(test, feature = "fs"))]
mod variant_tests {
    use super::*;
    use crate::input::{load_variant_svg_files, resolve_generate_webfonts_options};
    use crate::{FontVariant, GenerateWebfontsOptions};
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    use write_fonts::read::FontRef;
    use write_fonts::types::Tag;

    fn woff1_table(woff: &[u8], wanted: [u8; 4]) -> Vec<u8> {
        let count = usize::from(u16::from_be_bytes(woff[12..14].try_into().unwrap()));
        let entry = (0..count)
            .map(|index| 44 + index * 20)
            .find(|entry| woff[*entry..*entry + 4] == wanted)
            .unwrap();
        let offset = u32::from_be_bytes(woff[entry + 4..entry + 8].try_into().unwrap()) as usize;
        let compressed =
            u32::from_be_bytes(woff[entry + 8..entry + 12].try_into().unwrap()) as usize;
        let original =
            u32::from_be_bytes(woff[entry + 12..entry + 16].try_into().unwrap()) as usize;
        let bytes = &woff[offset..offset + compressed];
        if compressed == original {
            bytes.to_vec()
        } else {
            let mut decoded = Vec::with_capacity(original);
            ZlibDecoder::new(bytes).read_to_end(&mut decoded).unwrap();
            decoded
        }
    }

    #[tokio::test]
    async fn variant_bundle_preserves_modern_tables_and_filters_formats() {
        let paths = vec![vec![crate::test_helpers::webfont_fixture("add.svg")]; 2];
        let mut options = resolve_generate_webfonts_options(GenerateWebfontsOptions {
            dest: "artifacts".to_owned(),
            variants: Some(vec![
                FontVariant {
                    name: "Light".to_owned(),
                    files: paths[0].clone(),
                    weight: Some(300),
                    default: None,
                },
                FontVariant {
                    name: "Bold".to_owned(),
                    files: paths[1].clone(),
                    weight: Some(700),
                    default: Some(true),
                },
            ]),
            types: Some(vec![FontType::Ttf, FontType::Woff, FontType::Woff2]),
            write_files: Some(false),
            ..Default::default()
        })
        .unwrap();
        let files = load_variant_svg_files(&paths, None).await.unwrap();
        let (family, _, _) = crate::prepare_variant_family(&mut options, files).unwrap();
        let outputs = build_variant_font_outputs(&options, &family).unwrap();
        let variable = FontRef::new(outputs.ttf_font.as_ref().unwrap()).unwrap();
        let decoded = ::woff::version2::decompress(outputs.woff2_font.as_ref().unwrap()).unwrap();
        let decoded = FontRef::new(&decoded).unwrap();
        // WOFF1 preserves loca byte-for-byte, including small uncompressed tables.
        // WOFF2 reconstruction may change glyf padding and therefore loca offsets.
        assert_eq!(
            woff1_table(outputs.woff_font.as_ref().unwrap(), *b"loca"),
            variable.table_data(Tag::new(b"loca")).unwrap().as_bytes()
        );
        for tag in [*b"fvar", *b"STAT", *b"GSUB", *b"maxp"] {
            let expected = variable.table_data(Tag::new(&tag)).unwrap();
            assert_eq!(
                woff1_table(outputs.woff_font.as_ref().unwrap(), tag),
                expected.as_bytes()
            );
            assert_eq!(
                decoded.table_data(Tag::new(&tag)).unwrap().as_bytes(),
                expected.as_bytes()
            );
        }
        for types in [
            vec![],
            vec![FontType::Ttf],
            vec![FontType::Woff],
            vec![FontType::Woff2],
        ] {
            options.types = types;
            let outputs = build_variant_font_outputs(&options, &family).unwrap();
            assert_eq!(
                outputs.ttf_font.is_some(),
                options.types.contains(&FontType::Ttf)
            );
            assert_eq!(
                outputs.woff_font.is_some(),
                options.types.contains(&FontType::Woff)
            );
            assert_eq!(
                outputs.woff2_font.is_some(),
                options.types.contains(&FontType::Woff2)
            );
            assert!(outputs.svg_font.is_none());
            assert!(outputs.eot_font.is_none());
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct TtfGlyphCache {
    pub(crate) entries: HashMap<u64, Arc<CachedCompiledGlyph>>,
    pub(crate) tables: HashMap<u64, ([u8; 4], Vec<u8>)>,
    pub(crate) woff1_payloads: Woff1PayloadCache,
    pub(crate) woff2_transforms: Woff2TransformCache,
    #[cfg(test)]
    pub compile_count: usize,
    #[cfg(test)]
    pub table_compile_count: usize,
}

impl TtfGlyphCache {
    #[cfg(all(test, feature = "fs"))]
    pub(crate) fn woff2_transform_compile_count(&self) -> usize {
        self.woff2_transforms.compile_count
    }
}

#[cfg(feature = "fs")]
pub(crate) fn generate_webfonts_sync(
    options: ResolvedFileGenerationOptions,
    source_files: Vec<LoadedSvgSource>,
) -> std::io::Result<GenerateWebfontsResult> {
    let svg_options = svg_options_from_options(&options);
    let disk = options
        .cache_location
        .as_deref()
        .map(|dir| DiskCache::open(dir, &options, &source_files));

    // When incremental, retain the parsed-glyph cache so a later `regenerate` can reuse the
    // glyphs whose source didn't change. Otherwise the geometry is dropped as soon as the font
    // is built, so one-shot builds carry no extra memory.
    let mut ttf_cache = options.incremental.then(TtfGlyphCache::default);
    let (fonts, glyph_cache) = if let Some(disk) = disk {
        let outcome = build_with_cache(
            &options,
            &svg_options,
            &source_files,
            &disk,
            ttf_cache.as_mut(),
        );
        let (fonts, glyph_cache, built) = outcome?;
        if built && let Some(glyph_cache) = &glyph_cache {
            disk.store(&source_files, glyph_cache, &fonts, &options.types);
        }
        (fonts, glyph_cache.filter(|_| options.incremental))
    } else if options.incremental {
        let mut cache = GlyphCache::default();
        let prepared = prepare_svg_font_incremental(&svg_options, &source_files, &mut cache)?;
        let fonts = build_font_outputs(&options, &svg_options, &prepared, ttf_cache.as_mut())?;
        (fonts, Some(cache))
    } else {
        let prepared = prepare_svg_font(&svg_options, &source_files)?;
        (
            build_font_outputs(&options, &svg_options, &prepared, None)?,
            None,
        )
    };
    let regeneration_state = glyph_cache.map(|glyph_cache| RegenerationState {
        variant_cache: None,
        variant_write_pending: false,
        caches_dirty: false,
        glyph_cache,
        ttf_cache,
        written_outputs: std::collections::HashMap::new(),
    });

    Ok(GenerateWebfontsResult {
        cached: std::sync::OnceLock::new(),
        carried_render: None,
        css_context: None,
        fonts,
        html_context: None,
        options: std::sync::Arc::new(options),
        regeneration_state: std::sync::Arc::new(std::sync::Mutex::new(regeneration_state)),
        source_files: std::sync::Arc::new(source_files),
    })
}

/// Generate with a persistent cache: reuse every cached format, seed glyph work from cached
/// glyphs, and build only the formats that missed. Returns the outputs, the glyph cache used
/// (seeded from `state`, then filled by any glyph work; `None` when every format hit and the
/// result isn't incremental, so nothing needs it), and whether anything was built.
pub(crate) fn build_with_cache(
    options: &ResolvedGenerationOptions,
    svg_options: &SvgOptions<'_>,
    sources: &[LoadedSvgSource],
    state: &CacheState,
    ttf_cache: Option<&mut TtfGlyphCache>,
) -> std::io::Result<(FontOutputs, Option<GlyphCache>, bool)> {
    let (hits, missing, full_hit) = split_hits(state, options);
    // Hydrating hashes every source and clones every cached glyph; skip it when nothing uses it.
    let mut glyph_cache =
        (options.incremental || !full_hit).then(|| state.hydrate(svg_options, sources));
    let mut fonts = if let Some(glyph_cache) = glyph_cache.as_mut().filter(|_| !full_hit) {
        let prepared = prepare_svg_font_incremental(svg_options, sources, glyph_cache)?;
        let mut subset = options.clone();
        subset.types.clone_from(&missing);
        build_font_outputs(&subset, svg_options, &prepared, ttf_cache)?
    } else {
        FontOutputs::default()
    };
    for font in hits {
        font.place(&mut fonts);
    }
    Ok((fonts, glyph_cache, !full_hit))
}

/// Cached output for each requested format with a current entry, the formats still to build, and
/// whether every font hit. Only a run whose fonts all came from the cache may skip glyph work:
/// with no formats requested, the sources must still be prepared (and so validated).
fn split_hits(
    state: &CacheState,
    options: &ResolvedGenerationOptions,
) -> (Vec<CachedFont>, Vec<FontType>, bool) {
    let mut hits = state.font_hits(&options.types);
    let needs_glyphs = options.incremental || hits.is_empty() || hits.len() < options.types.len();
    // A run that needs the saved glyphs and finds them corrupt treats the whole cache as a miss,
    // like a corrupt file, so the rebuild replaces every section.
    if needs_glyphs && state.glyphs().is_none() {
        hits.clear();
    }
    let missing: Vec<FontType> = options
        .types
        .iter()
        .copied()
        .filter(|font_type| hits.iter().all(|hit| hit.font_type() != *font_type))
        .collect();
    let full_hit = !hits.is_empty() && missing.is_empty();
    (hits, missing, full_hit)
}

/// Variant counterpart of [`build_with_cache`]. When every format hits, the family is never
/// prepared.
pub(crate) fn build_variant_with_cache<Files: AsRef<[LoadedSvgSource]>>(
    options: &ResolvedGenerationOptions,
    variants: &ResolvedVariants,
    svg_options: &SvgOptions<'_>,
    family: &VariantFamilySources<Files>,
    state: &CacheState,
) -> std::io::Result<(FontOutputs, Option<VariantGlyphCache>, bool)> {
    let (hits, missing, full_hit) = split_hits(state, options);
    let mut glyph_cache = (options.incremental || !full_hit)
        .then(|| state.hydrate_variants(svg_options, &family.variants));
    let mut fonts = if let Some(glyph_cache) = glyph_cache.as_mut().filter(|_| !full_hit) {
        let prepared = prepare_variant_svg_family_cached(svg_options, family, Some(glyph_cache))?;
        let mut subset = options.clone();
        subset.types.clone_from(&missing);
        build_content_variant_outputs(&subset, variants, &prepared)?
    } else {
        FontOutputs::default()
    };
    for font in hits {
        font.place(&mut fonts);
    }
    Ok((fonts, glyph_cache, !full_hit))
}

#[cfg(feature = "fs")]
pub(crate) fn generate_variant_webfonts_sync(
    mut options: ResolvedFileGenerationOptions,
    source_files: Vec<Vec<LoadedSvgSource>>,
) -> std::io::Result<GenerateWebfontsResult> {
    let (fonts, cache, source_files) = if let Some(dir) = options.cache_location.clone() {
        let variants = options
            .variants
            .as_ref()
            .expect("variant generation requires resolved variants");
        let family = resolve_content_variant_sources(
            &mut options.common,
            variants,
            source_files.iter().map(Vec::as_slice).collect(),
        )?;
        let disk = DiskCache::open_variant(&dir, &options, variants, &family);
        let svg_options = svg_options_from_options(&options);
        let outcome = build_variant_with_cache(&options, variants, &svg_options, &family, &disk);
        let (fonts, glyph_cache, built) = outcome?;
        if built && let Some(glyph_cache) = &glyph_cache {
            disk.store_variants(&family.variants, glyph_cache, &fonts, &options.types);
        }
        let glyph_cache = glyph_cache.filter(|_| options.incremental);
        (
            fonts,
            glyph_cache,
            source_files.into_iter().flatten().collect(),
        )
    } else {
        let (family, source_files, cache) = prepare_variant_family(&mut options, source_files)?;
        (
            build_variant_font_outputs(&options, &family)?,
            cache,
            source_files,
        )
    };
    let regeneration_state = if options.incremental {
        Some(RegenerationState {
            variant_cache: cache,
            variant_write_pending: false,
            caches_dirty: false,
            glyph_cache: GlyphCache::default(),
            ttf_cache: None,
            written_outputs: HashMap::new(),
        })
    } else {
        None
    };
    Ok(GenerateWebfontsResult {
        cached: std::sync::OnceLock::new(),
        carried_render: None,
        css_context: None,
        fonts,
        html_context: None,
        options: Arc::new(options),
        regeneration_state: Arc::new(std::sync::Mutex::new(regeneration_state)),
        source_files: Arc::new(source_files),
    })
}

/// Build every requested output format from an already-prepared glyph set.
pub(crate) fn build_font_outputs(
    options: &ResolvedGenerationOptions,
    svg_options: &SvgOptions<'_>,
    prepared: &PreparedSvgFont,
    mut ttf_cache: Option<&mut TtfGlyphCache>,
) -> std::io::Result<FontOutputs> {
    let wants_svg = options.types.contains(&FontType::Svg);
    let wants_ttf = options.types.contains(&FontType::Ttf);
    let wants_woff = options.types.contains(&FontType::Woff);
    let wants_woff2 = options.types.contains(&FontType::Woff2);
    let wants_eot = options.types.contains(&FontType::Eot);

    let (svg_font, ttf_tables) = join(
        || -> std::io::Result<Option<String>> {
            if wants_svg {
                Ok(Some(build_svg_font(svg_options, prepared)))
            } else {
                Ok(None)
            }
        },
        || -> std::io::Result<Option<sfnt::SerializedFontTables>> {
            if wants_ttf || wants_woff || wants_woff2 || wants_eot {
                sfnt::build(
                    sfnt::ttf_options_from_options(options),
                    &prepared.processed_glyphs,
                    ttf_cache.as_deref_mut(),
                )
                .map(Some)
            } else {
                Ok(None)
            }
        },
    );

    let svg_font = svg_font?.map(Arc::new);
    let ttf_tables = ttf_tables?;

    let (ttf_font, woff_font, woff2_font, eot_font) = if let Some(ttf_tables) = ttf_tables {
        let woff_metadata = options
            .format_options
            .as_ref()
            .and_then(|value| value.woff.as_ref())
            .and_then(|value| value.metadata.as_deref());
        let woff2_quality = options
            .format_options
            .as_ref()
            .and_then(|value| value.woff2.as_ref())
            .and_then(|value| value.compression_quality)
            .unwrap_or(11);

        let ttf_tables = Arc::new(ttf_tables);
        let ttf_font = wants_ttf.then(|| ttf_tables.ttf_arc());
        let (woff1_cache, woff2_cache) = match ttf_cache {
            Some(cache) => (
                Some(&mut cache.woff1_payloads),
                Some(&mut cache.woff2_transforms),
            ),
            None => (None, None),
        };
        let (woff_font, (woff2_font, eot_font)) = join(
            || -> std::io::Result<Option<Vec<u8>>> {
                if wants_woff {
                    match woff1_cache {
                        Some(cache) => {
                            woff1::tables_to_woff1_cached(&ttf_tables, woff_metadata, cache)
                        }
                        None => woff1::tables_to_woff1(&ttf_tables, woff_metadata),
                    }
                    .map(Some)
                } else {
                    Ok(None)
                }
            },
            || {
                join(
                    || -> std::io::Result<Option<Vec<u8>>> {
                        if wants_woff2 {
                            woff2::tables_to_woff2(&ttf_tables, woff2_quality, woff2_cache)
                                .map(Some)
                        } else {
                            Ok(None)
                        }
                    },
                    || -> std::io::Result<Option<Vec<u8>>> {
                        if wants_eot {
                            eot::tables_to_eot(&ttf_tables).map(Some)
                        } else {
                            Ok(None)
                        }
                    },
                )
            },
        );

        (
            ttf_font,
            woff_font?.map(Arc::new),
            woff2_font?.map(Arc::new),
            eot_font?.map(Arc::new),
        )
    } else {
        (None, None, None, None)
    };

    Ok(FontOutputs {
        svg_font,
        ttf_font,
        woff_font,
        woff2_font,
        eot_font,
    })
}

#[cfg(feature = "fs")]
pub(crate) fn build_variant_font_outputs(
    options: &ResolvedFileGenerationOptions,
    family: &crate::svg::types::PreparedVariantFamily,
) -> std::io::Result<FontOutputs> {
    build_content_variant_outputs(
        options,
        options
            .variants
            .as_ref()
            .expect("variant outputs require resolved variants"),
        family,
    )
}

pub(crate) fn build_content_variant_outputs(
    options: &ResolvedGenerationOptions,
    variants: &ResolvedVariants,
    family: &crate::svg::types::PreparedVariantFamily,
) -> std::io::Result<FontOutputs> {
    let wants_ttf = options.types.contains(&FontType::Ttf);
    let wants_woff = options.types.contains(&FontType::Woff);
    let wants_woff2 = options.types.contains(&FontType::Woff2);
    if !wants_ttf && !wants_woff && !wants_woff2 {
        return Ok(FontOutputs::default());
    }
    let variable =
        sfnt::build_variant(sfnt::ttf_options_from_options(options), family, variants)?.tables;
    let metadata = options
        .format_options
        .as_ref()
        .and_then(|formats| formats.woff.as_ref())
        .and_then(|woff| woff.metadata.as_deref());
    let quality = options
        .format_options
        .as_ref()
        .and_then(|formats| formats.woff2.as_ref())
        .and_then(|woff2| woff2.compression_quality)
        .unwrap_or(11);
    let ttf_font = wants_ttf.then(|| variable.ttf_arc());
    let (woff_font, woff2_font) = join(
        || {
            wants_woff
                .then(|| woff1::tables_to_woff1(&variable, metadata))
                .transpose()
        },
        || {
            wants_woff2
                .then(|| woff2::tables_to_woff2(&variable, quality, None))
                .transpose()
        },
    );
    Ok(FontOutputs {
        ttf_font,
        woff_font: woff_font?.map(Arc::new),
        woff2_font: woff2_font?.map(Arc::new),
        ..Default::default()
    })
}
