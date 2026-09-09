use std::collections::HashMap;
use std::sync::Arc;

use rayon::join;

use crate::formats::woff1::Woff1PayloadCache;
use crate::formats::woff2::Woff2TransformCache;
use crate::formats::{eot, woff1, woff2};
use crate::incremental::RegenerationState;
use crate::input::{LoadedSvgFile, ResolvedGenerateWebfontsOptions};
use crate::result::{FontOutputs, GenerateWebfontsResult};
use crate::sfnt;
use crate::sfnt::CachedCompiledGlyph;
use crate::svg::types::{GlyphCache, PreparedSvgFont, SvgOptions};
use crate::svg::{
    build_svg_font, prepare_svg_font, prepare_svg_font_incremental, svg_options_from_options,
};
use crate::types::FontType;

#[cfg(test)]
mod variant_tests {
    use std::io::Read;

    use flate2::read::ZlibDecoder;
    use kurbo::{BezPath, Point};
    use write_fonts::read::tables::gsub::SubstitutionLookup as ReadSubstitutionLookup;
    use write_fonts::read::{FontRef, TableProvider};
    use write_fonts::types::Tag;

    use super::*;
    use crate::input::resolve_generate_webfonts_options;
    use crate::svg::types::{PreparedVariantFamily, ProcessedGlyph, ProcessedVariantGlyph};
    use crate::{FontVariant, GenerateWebfontsOptions};

    fn options(types: Vec<FontType>) -> ResolvedGenerateWebfontsOptions {
        resolve_generate_webfonts_options(GenerateWebfontsOptions {
            css: Some(false),
            dest: "artifacts".to_owned(),
            files: vec![],
            html: Some(false),
            types: Some(types),
            variants: Some(vec![
                FontVariant {
                    name: "Light".to_owned(),
                    files: vec!["light.svg".to_owned()],
                    weight: Some(300),
                    default: None,
                },
                FontVariant {
                    name: "Bold".to_owned(),
                    files: vec!["bold.svg".to_owned()],
                    weight: Some(700),
                    default: Some(true),
                },
            ]),
            write_files: Some(false),
            ..Default::default()
        })
        .unwrap()
    }

    fn family() -> PreparedVariantFamily {
        let outline = |name: &str, codepoint, shape_width| {
            let mut path = BezPath::new();
            path.move_to(Point::new(0.0, 0.0));
            path.line_to(Point::new(shape_width, 0.0));
            path.line_to(Point::new(0.0, 10.0));
            path.close_path();
            ProcessedGlyph {
                codepoint,
                height: 10.0,
                index: 0,
                name: name.to_owned(),
                path_data: Arc::from(""),
                ttf_path: Some(Arc::new(path)),
                ttf_path_hash: None,
                width: 20.0,
            }
        };
        PreparedVariantFamily {
            ascent: 10.0,
            descent: 0.0,
            font_height: 1000.0,
            glyphs: [("icon", 0xe001), ("mark", 0xe002)]
                .into_iter()
                .map(|(name, codepoint)| ProcessedVariantGlyph {
                    name: name.to_owned(),
                    codepoint,
                    advance_width: 20.0,
                    outlines: vec![
                        Some(outline(name, codepoint, 10.0)),
                        Some(outline(name, codepoint, 20.0)),
                    ]
                    .into_boxed_slice(),
                })
                .collect(),
        }
    }

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

    #[test]
    fn variant_bundle_preserves_variable_tables_and_builds_static_eots() {
        let options = options(vec![
            FontType::Ttf,
            FontType::Woff,
            FontType::Woff2,
            FontType::Eot,
        ]);
        let outputs = build_variant_font_outputs(&options, &family()).unwrap();
        let ttf = outputs.ttf_font.unwrap();
        let variable = FontRef::new(&ttf).unwrap();
        let decoded = ::woff::version2::decompress(outputs.woff2_font.as_ref().unwrap())
            .expect("variant WOFF2 should decode");
        let decoded = FontRef::new(&decoded).unwrap();

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

        assert_eq!(outputs.legacy_eot_fonts.len(), 2);
        let mut identities = Vec::new();
        for (((eot, weight), x_max), variant_name) in outputs
            .legacy_eot_fonts
            .iter()
            .zip([300, 700])
            .zip([10, 20])
            .zip(["Light", "Bold"])
        {
            assert_eq!(u32::from_le_bytes(eot[28..32].try_into().unwrap()), weight);
            let ttf_len = u32::from_le_bytes(eot[4..8].try_into().unwrap()) as usize;
            let font = FontRef::new(&eot[eot.len() - ttf_len..]).unwrap();
            assert_eq!(font.os2().unwrap().us_weight_class(), weight as u16);
            assert!(font.fvar().is_err());
            assert_eq!(font.head().unwrap().x_max(), x_max);

            let name = font.name().unwrap();
            let string_data = name.string_data();
            let read_name = |id| {
                name.name_record()
                    .iter()
                    .find(|record| record.name_id().to_u16() == id)
                    .unwrap()
                    .string(string_data)
                    .unwrap()
                    .to_string()
            };
            assert_eq!(read_name(2), variant_name);
            identities.push((read_name(4), read_name(6)));

            let gsub = font.gsub().unwrap();
            let lookup = gsub.lookup_list().unwrap().lookups().get(0).unwrap();
            let ReadSubstitutionLookup::Ligature(lookup) = lookup else {
                panic!("expected static ligature lookup")
            };
            let sets = lookup.subtables().get(0).unwrap().ligature_sets();
            let mut targets = Vec::new();
            for index in 0..sets.len() {
                targets.extend(
                    sets.get(index)
                        .unwrap()
                        .ligatures()
                        .iter()
                        .map(|ligature| ligature.unwrap().ligature_glyph()),
                );
            }
            assert_eq!(targets.len(), 2);
            assert!(targets.iter().all(|target| target.to_u16() == 1));
        }
        assert_ne!(identities[0], identities[1]);
        assert!(Arc::ptr_eq(
            outputs.eot_font.as_ref().unwrap(),
            &outputs.legacy_eot_fonts[1]
        ));
    }

    #[test]
    fn variant_bundle_only_returns_requested_formats() {
        for (types, expected) in [
            (vec![FontType::Ttf], (true, false, false, 0)),
            (vec![FontType::Woff2], (false, false, true, 0)),
            (vec![FontType::Eot], (false, false, false, 2)),
        ] {
            let outputs = build_variant_font_outputs(&options(types), &family()).unwrap();
            assert_eq!(outputs.ttf_font.is_some(), expected.0);
            assert_eq!(outputs.woff_font.is_some(), expected.1);
            assert_eq!(outputs.woff2_font.is_some(), expected.2);
            assert_eq!(outputs.legacy_eot_fonts.len(), expected.3);
            assert_eq!(outputs.eot_font.is_some(), expected.3 > 0);
            assert!(outputs.svg_font.is_none());
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
    #[cfg(test)]
    pub(crate) fn woff2_transform_compile_count(&self) -> usize {
        self.woff2_transforms.compile_count
    }
}

pub(crate) fn generate_webfonts_sync(
    options: ResolvedGenerateWebfontsOptions,
    source_files: Vec<LoadedSvgFile>,
) -> std::io::Result<GenerateWebfontsResult> {
    let svg_options = svg_options_from_options(&options);
    // When incremental, retain the parsed-glyph cache so a later `regenerate` can reuse the
    // glyphs whose source didn't change. Otherwise the geometry is dropped as soon as the font
    // is built, so one-shot builds carry no extra memory.
    let (prepared, glyph_cache, mut ttf_cache) = if options.incremental {
        let mut cache = GlyphCache::default();
        let prepared = prepare_svg_font_incremental(&svg_options, &source_files, &mut cache)?;
        (prepared, Some(cache), Some(TtfGlyphCache::default()))
    } else {
        (prepare_svg_font(&svg_options, &source_files)?, None, None)
    };
    let fonts = build_font_outputs(&options, &svg_options, &prepared, ttf_cache.as_mut())?;
    let regeneration_state = glyph_cache.map(|glyph_cache| RegenerationState {
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

/// Build every requested output format from an already-prepared glyph set.
pub(crate) fn build_font_outputs(
    options: &ResolvedGenerateWebfontsOptions,
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
        legacy_eot_fonts: vec![],
    })
}

#[allow(dead_code, reason = "variant result wiring is completed in Phase 9")]
pub(crate) fn build_variant_font_outputs(
    options: &ResolvedGenerateWebfontsOptions,
    family: &crate::svg::types::PreparedVariantFamily,
) -> std::io::Result<FontOutputs> {
    let wants_ttf = options.types.contains(&FontType::Ttf);
    let wants_woff = options.types.contains(&FontType::Woff);
    let wants_woff2 = options.types.contains(&FontType::Woff2);
    let wants_eot = options.types.contains(&FontType::Eot);
    let variants = options
        .variants
        .as_ref()
        .expect("variant outputs require resolved variants");

    let variable = if wants_ttf || wants_woff || wants_woff2 {
        Some(Arc::new(
            sfnt::build_variant(sfnt::ttf_options_from_options(options), family, variants)?.tables,
        ))
    } else {
        None
    };
    let ttf_font = wants_ttf.then(|| variable.as_ref().unwrap().ttf_arc());
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
    let woff_font = wants_woff
        .then(|| woff1::tables_to_woff1(variable.as_ref().unwrap(), woff_metadata))
        .transpose()?
        .map(Arc::new);
    let woff2_font = wants_woff2
        .then(|| woff2::tables_to_woff2(variable.as_ref().unwrap(), woff2_quality, None))
        .transpose()?
        .map(Arc::new);
    let legacy_eot_fonts = if wants_eot {
        variants
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                sfnt::build_static_variant(
                    sfnt::ttf_options_from_options(options),
                    family,
                    index,
                    &variant.name,
                    variant.weight,
                )
                .and_then(|tables| eot::tables_to_eot(&tables))
                .map(Arc::new)
            })
            .collect::<std::io::Result<Vec<_>>>()?
    } else {
        vec![]
    };
    let eot_font = legacy_eot_fonts.get(variants.default_index).cloned();

    Ok(FontOutputs {
        svg_font: None,
        ttf_font,
        woff_font,
        woff2_font,
        eot_font,
        legacy_eot_fonts,
    })
}
