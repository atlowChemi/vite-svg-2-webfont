use super::*;
use crate::FontType;
use crate::input::options::{GenerationOptions, finalize_generate_webfonts_options};
use crate::svg::{self, svg_options_from_options};

fn source(id: &str, name: &str) -> LoadedSvgSource {
    LoadedSvgSource {
        id: id.to_owned(),
        glyph_name: name.to_owned(),
        contents: r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M2 2H20V20H2Z"/></svg>"#.into(),
    }
}

#[test]
fn opaque_ids_and_renames_reuse_geometry_without_source_paths() {
    // The NUL makes this unusable as a filesystem path even on a permissive host.
    let mut sources = vec![source("../not-a-file\0.svg", "chosen-name")];
    let mut options = GenerationOptions {
        types: Some(vec![FontType::Svg]),
        ..Default::default()
    }
    .resolve(false)
    .unwrap();
    finalize_generate_webfonts_options(&mut options, &sources).unwrap();
    let mut cache = svg::types::GlyphCache::default();
    svg::prepare_svg_font_incremental(&svg_options_from_options(&options), &sources, &mut cache)
        .unwrap();
    let geometry = cache.entries[&sources[0].id].clone();
    let payload = sources[0].contents.clone();
    let process_count = cache.process_count;

    sources[0].glyph_name = "renamed".into();
    finalize_generate_webfonts_options(&mut options, &sources).unwrap();
    svg::prepare_svg_font_incremental(&svg_options_from_options(&options), &sources, &mut cache)
        .unwrap();
    assert_eq!(cache.parse_count, 1);
    assert_eq!(cache.process_count, process_count);
    assert!(Arc::ptr_eq(&geometry, &cache.entries[&sources[0].id]));

    sources.push(LoadedSvgSource {
        id: "C:\\not-a-file\\other.svg".into(),
        glyph_name: "copy".into(),
        contents: payload.clone(),
    });
    finalize_generate_webfonts_options(&mut options, &sources).unwrap();
    let svg_options = svg_options_from_options(&options);
    let cached = svg::prepare_svg_font_incremental(&svg_options, &sources, &mut cache).unwrap();
    let fresh = svg::prepare_svg_font(&svg_options, &sources).unwrap();
    assert_eq!(
        cache.parse_count, 1,
        "identical content under a new ID is reused"
    );
    assert!(Arc::ptr_eq(&payload, &sources[0].contents));
    assert!(Arc::ptr_eq(&geometry, &cache.entries[&sources[1].id]));
    assert_eq!(cached.processed_glyphs[0].name, "renamed");
    assert_eq!(cached.processed_glyphs[1].name, "copy");
    assert_eq!(
        svg::build_svg_font(&svg_options, &cached),
        svg::build_svg_font(&svg_options, &fresh)
    );
}

#[test]
fn shared_ids_preserve_per_design_names_and_caches() {
    let first = source("shared-id", "first-name");
    let mut second = first.clone();
    second.glyph_name = "second-name".into();
    let (mut family, codepoints) =
        build_variant_family_sources(vec![vec![first], vec![second]], &BTreeMap::new(), 0xf101)
            .unwrap();
    resolve_missing_glyphs(
        &mut family,
        MissingGlyphBehavior::Blank,
        None,
        &["light", "bold"],
    )
    .unwrap();
    let mut options = GenerationOptions::default().resolve(true).unwrap();
    options.codepoints = codepoints;
    let svg_options = svg_options_from_options(&options);
    let mut cache = svg::VariantGlyphCache::default();
    let prepared =
        svg::prepare_variant_svg_family_cached(&svg_options, &family, Some(&mut cache)).unwrap();
    assert_eq!(prepared.glyphs[0].name, "first-name");
    assert!(prepared.glyphs[0].outlines[0].is_some());
    assert!(prepared.glyphs[0].outlines[1].is_none());
    assert_eq!(prepared.glyphs[1].name, "second-name");
    assert!(prepared.glyphs[1].outlines[0].is_none());
    assert!(prepared.glyphs[1].outlines[1].is_some());
    let process_count = cache.process_count;
    svg::prepare_variant_svg_family_cached(&svg_options, &family, Some(&mut cache)).unwrap();
    assert_eq!(cache.parsed[0].parse_count, 1);
    assert_eq!(cache.parsed[1].parse_count, 1);
    assert_eq!(cache.process_count, process_count);
}
