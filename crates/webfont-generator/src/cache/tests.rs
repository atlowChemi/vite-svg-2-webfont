use std::env::temp_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::*;
use crate::test_helpers::resolve_options;
use crate::{
    ColorGlyphSelection, FormatOptions, GenerateWebfontsOptions, GenerateWebfontsResult,
    GlyphChange, RegenerationFiles, TtfFormatOptions, Woff2FormatOptions, WoffFormatOptions,
    generate_sync,
};

const ALL_TYPES: [FontType; 5] = [
    FontType::Svg,
    FontType::Ttf,
    FontType::Eot,
    FontType::Woff,
    FontType::Woff2,
];

fn svg(width: u32) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2H{width}V20H2Z M12 4 C16 4 20 8 20 12 Q16 18 12 20Z"/></svg>"#
    )
}

struct Project {
    root: PathBuf,
}

impl Project {
    fn new(name: &str) -> Self {
        let root = temp_dir().join(format!(
            "wfg-cache-{name}-{}-{}",
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("icons")).unwrap();
        for (name, width) in [("add", 10), ("remove", 14), ("star", 18)] {
            fs::write(root.join("icons").join(format!("{name}.svg")), svg(width)).unwrap();
        }
        Self { root }
    }
    fn icons(&self) -> Vec<String> {
        let mut files: Vec<_> = fs::read_dir(self.root.join("icons"))
            .unwrap()
            .map(|entry| entry.unwrap().path().to_string_lossy().into_owned())
            .collect();
        files.sort();
        files
    }
    fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
    fn options(&self, cache: Option<&Path>) -> GenerateWebfontsOptions {
        GenerateWebfontsOptions {
            cache_location: cache.map(|path| path.to_string_lossy().into_owned()),
            css: Some(false),
            dest: self.root.join("out").to_string_lossy().into_owned(),
            files: self.icons(),
            format_options: Some(FormatOptions {
                ttf: Some(TtfFormatOptions {
                    copyright: None,
                    description: None,
                    ts: Some(0),
                    url: None,
                    version: None,
                }),
                ..Default::default()
            }),
            html: Some(false),
            incremental: Some(true),
            types: Some(ALL_TYPES.to_vec()),
            write_files: Some(false),
            ..Default::default()
        }
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run(options: GenerateWebfontsOptions) -> GenerateWebfontsResult {
    generate_sync(options, None).unwrap()
}

fn fonts(result: &GenerateWebfontsResult) -> Vec<Vec<u8>> {
    let outputs = &result.fonts;
    vec![
        outputs.svg_font.as_deref().unwrap().as_bytes().to_vec(),
        outputs.ttf_font.as_deref().unwrap().clone(),
        outputs.eot_font.as_deref().unwrap().clone(),
        outputs.woff_font.as_deref().unwrap().clone(),
        outputs.woff2_font.as_deref().unwrap().clone(),
    ]
}

/// (parsed, processed) counts of the glyph work done to produce `result`.
fn work(result: &GenerateWebfontsResult) -> (usize, usize) {
    let state = result.regeneration_state.lock().unwrap();
    let cache = &state.as_ref().unwrap().glyph_cache;
    (cache.parse_count, cache.process_count)
}

fn cache_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn warm_runs_are_byte_identical_and_skip_glyph_work() {
    let project = Project::new("identity");
    let uncached = fonts(&run(project.options(None)));

    let cold = run(project.options(Some(&project.cache())));
    assert_eq!(fonts(&cold), uncached);
    assert_eq!(work(&cold), (3, 3));

    let warm = run(project.options(Some(&project.cache())));
    assert_eq!(fonts(&warm), uncached);
    assert_eq!(work(&warm), (0, 0));
}

#[test]
fn an_edited_icon_reprocesses_only_that_icon() {
    let project = Project::new("edit");
    run(project.options(Some(&project.cache())));
    fs::write(project.root.join("icons/add.svg"), svg(12)).unwrap();

    let warm = run(project.options(Some(&project.cache())));
    assert_eq!(fonts(&warm), fonts(&run(project.options(None))));
    assert_eq!(work(&warm), (1, 1));
}

#[test]
fn css_only_changes_reuse_every_font() {
    let project = Project::new("css-only");
    run(project.options(Some(&project.cache())));
    let mut options = project.options(Some(&project.cache()));
    options.css = Some(true);
    options.css_fonts_url = Some("/fonts/".to_owned());

    let warm = run(options);
    assert_eq!(work(&warm), (0, 0));
    assert_eq!(fonts(&warm), fonts(&run(project.options(None))));
}

#[test]
fn a_woff2_quality_change_keeps_glyphs_and_other_formats_warm() {
    let project = Project::new("quality");
    let with_quality = |quality| {
        let mut options = project.options(Some(&project.cache()));
        options.format_options.as_mut().unwrap().woff2 = Some(Woff2FormatOptions {
            compression_quality: Some(quality),
        });
        options
    };
    run(with_quality(11));

    let dev = run(with_quality(9));
    assert_eq!(work(&dev), (0, 0));
    let mut uncached = with_quality(9);
    uncached.cache_location = None;
    assert_eq!(fonts(&dev), fonts(&run(uncached)));

    // Both qualities stay cached; a third evicts the older one.
    let entries = |cache: &Path| {
        let disk = DiskCache::open(
            &cache.to_string_lossy(),
            &resolve_options(with_quality(11)),
            &[],
        );
        assert!(disk.loaded.is_some());
        disk.fonts
            .iter()
            .filter(|entry| entry.font.font_type() == FontType::Woff2)
            .count()
    };
    assert_eq!(entries(&project.cache()), 2);
    run(with_quality(10));
    assert_eq!(entries(&project.cache()), 2);
}

#[test]
fn format_options_invalidate_only_the_formats_built_from_them() {
    let project = Project::new("per-format");
    run(project.options(Some(&project.cache())));
    let entries = |options: GenerateWebfontsOptions| {
        let disk = DiskCache::open(
            &project.cache().to_string_lossy(),
            &resolve_options(options),
            &[],
        );
        assert!(disk.loaded.is_some());
        FONT_TYPES.map(|font_type| {
            disk.fonts
                .iter()
                .filter(|entry| entry.font.font_type() == font_type)
                .count()
        })
    };

    // SVG, TTF, EOT, WOFF, WOFF2: WOFF metadata only feeds the WOFF.
    let mut woff = project.options(Some(&project.cache()));
    woff.format_options.as_mut().unwrap().woff = Some(WoffFormatOptions {
        metadata: Some("<metadata/>".to_owned()),
    });
    run(woff.clone());
    assert_eq!(entries(woff.clone()), [1, 1, 1, 2, 1]);

    // TTF options feed every format built from the TTF, but not the SVG font.
    woff.format_options
        .as_mut()
        .unwrap()
        .ttf
        .as_mut()
        .unwrap()
        .version = Some("2.0".to_owned());
    let result = run(woff.clone());
    assert_eq!(entries(woff.clone()), [1, 2, 2, 2, 2]);
    woff.cache_location = None;
    assert_eq!(fonts(&result), fonts(&run(woff)));
}

#[test]
fn unset_and_empty_format_options_share_one_cache_file() {
    let project = Project::new("empty-formats");
    let mut unset = project.options(Some(&project.cache()));
    unset.format_options = None;
    let mut quality = unset.clone();
    quality.format_options = Some(FormatOptions {
        ttf: Some(TtfFormatOptions {
            copyright: None,
            description: None,
            ts: None,
            url: None,
            version: None,
        }),
        woff2: Some(Woff2FormatOptions {
            compression_quality: Some(9),
        }),
        ..Default::default()
    });

    let first = run(unset.clone());
    let file = cache_files(&project.cache());
    // Same file and glyphs; only the WOFF2 is encoded again.
    assert_eq!(work(&run(quality)), (0, 0));
    assert_eq!(cache_files(&project.cache()), file);
    // Every font comes back from the cache, down to the unset TTF timestamp of the first run.
    let back = run(unset);
    assert_eq!(work(&back), (0, 0));
    assert_eq!(fonts(&back), fonts(&first));
}

#[test]
fn no_requested_formats_still_validate_sources() {
    let project = Project::new("no-types");
    let mut options = project.options(Some(&project.cache()));
    options.types = Some(Vec::new());
    run(options.clone());
    run(options.clone());

    // An empty format list is never a cache hit, so the SVGs are parsed as without a cache.
    fs::write(project.root.join("icons/broken.svg"), "<svg").unwrap();
    options.files = project.icons();
    let mut uncached = options.clone();
    uncached.cache_location = None;
    let error = |options| generate_sync(options, None).err().unwrap().to_string();
    assert_eq!(error(options), error(uncached));
}

#[test]
fn a_moved_checkout_still_hits() {
    let first = Project::new("moved-a");
    let second = Project::new("moved-b");
    let shared = first.cache();
    run(first.options(Some(&shared)));

    let warm = run(second.options(Some(&shared)));
    assert_eq!(work(&warm), (0, 0));
    assert_eq!(fonts(&warm), fonts(&run(second.options(None))));
}

#[test]
fn removed_icons_are_pruned_and_old_settings_files_replaced() {
    let project = Project::new("prune");
    run(project.options(Some(&project.cache())));
    fs::remove_file(project.root.join("icons/star.svg")).unwrap();
    run(project.options(Some(&project.cache())));

    let file = cache_files(&project.cache()).pop().unwrap();
    let options = resolve_options(project.options(Some(&project.cache())));
    let disk = DiskCache::open(&project.cache().to_string_lossy(), &options, &[]);
    assert!(disk.loaded.is_some());
    assert_eq!((disk.parsed.len(), disk.processed.len()), (2, 2));

    let mut changed = project.options(Some(&project.cache()));
    changed.font_height = Some(500.0);
    run(changed);
    let files = cache_files(&project.cache());
    assert_eq!(files.len(), 1);
    assert_ne!(files[0], file);
}

#[test]
fn unusable_cache_files_and_locations_fall_back_to_a_correct_cold_run() {
    let project = Project::new("corrupt");
    let expected = fonts(&run(project.options(None)));
    run(project.options(Some(&project.cache())));
    let file = project
        .cache()
        .join(cache_files(&project.cache()).pop().unwrap());
    let valid = fs::read(&file).unwrap();

    let mut wrong_version = valid.clone();
    wrong_version[MAGIC.len()] ^= 0xff;
    // Valid header, corrupted body: a flipped byte inside the last stored font still parses,
    // so only the checksum can catch it.
    let mut flipped_payload = valid.clone();
    let last = flipped_payload.len() - 10;
    flipped_payload[last] ^= 0xff;
    let mut trailing = valid.clone();
    trailing.extend_from_slice(b"extra");
    for contents in [
        b"garbage".to_vec(),
        valid[..valid.len() / 2].to_vec(),
        wrong_version,
        flipped_payload,
        trailing,
        Vec::new(),
    ] {
        fs::write(&file, contents).unwrap();
        let result = run(project.options(Some(&project.cache())));
        assert_eq!(fonts(&result), expected);
        assert_eq!(work(&result), (3, 3));
        // The run replaces the unusable file with a valid one.
        assert_eq!(fs::read(&file).unwrap(), valid);
    }

    // A path under a regular file can't be created: generation still succeeds, uncached.
    let blocked = file.join("nested");
    let result = run(project.options(Some(&blocked)));
    assert_eq!(fonts(&result), expected);
}

#[test]
fn identical_runs_do_not_rewrite_the_file() {
    let project = Project::new("no-rewrite");
    let mut one_shot = project.options(Some(&project.cache()));
    one_shot.incremental = Some(false);
    run(one_shot.clone());
    let file = project
        .cache()
        .join(cache_files(&project.cache()).pop().unwrap());
    let before = fs::metadata(&file).unwrap().modified().unwrap();
    sleep(Duration::from_millis(20));
    let result = run(one_shot);
    assert!(result.regeneration_state.lock().unwrap().is_none());
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), before);
}

#[test]
fn pruning_keeps_files_the_cache_did_not_create() {
    let project = Project::new("prune-foreign");
    run(project.options(Some(&project.cache())));
    let cache = project.cache();
    // Named like an older cache file for the same font, but not written by the cache.
    fs::write(cache.join("iconfont.0123456789abcdef.wfcache"), "not ours").unwrap();
    fs::create_dir(cache.join("iconfont.fedcba9876543210.wfcache")).unwrap();

    let mut changed = project.options(Some(&cache));
    changed.font_height = Some(500.0);
    run(changed);
    let files = cache_files(&cache);
    assert_eq!(files.len(), 3);
    assert!(files.contains(&"iconfont.0123456789abcdef.wfcache".to_owned()));
    assert!(files.contains(&"iconfont.fedcba9876543210.wfcache".to_owned()));
}

#[test]
fn clear_cache_removes_only_engine_files() {
    let project = Project::new("clear");
    run(project.options(Some(&project.cache())));
    let cache = project.cache();
    fs::write(cache.join("notes.txt"), "keep").unwrap();
    fs::write(cache.join("impostor.wfcache"), "not ours").unwrap();
    fs::write(
        cache.join("iconfont.0123456789abcdef.wfcache.1-2.tmp"),
        "partial",
    )
    .unwrap();
    // Temporary-looking names the cache never writes.
    for name in [
        "notes.wfcache.backup.tmp",
        "iconfont.wfcache.1-2.tmp",
        "iconfont.0123456789abcdef.wfcache.x-2.tmp",
        "iconfont.0123456789abcdef.wfcache.1-.tmp",
    ] {
        fs::write(cache.join(name), "keep").unwrap();
    }

    let report = clear_cache(&cache).unwrap();
    assert_eq!(report.files_removed, 2);
    assert!(report.bytes_removed > 0);
    assert_eq!(
        cache_files(&cache),
        [
            "iconfont.0123456789abcdef.wfcache.1-.tmp",
            "iconfont.0123456789abcdef.wfcache.x-2.tmp",
            "iconfont.wfcache.1-2.tmp",
            "impostor.wfcache",
            "notes.txt",
            "notes.wfcache.backup.tmp",
        ]
    );

    assert_eq!(
        clear_cache(project.root.join("missing")).unwrap(),
        ClearCacheReport::default()
    );
    let result = run(project.options(Some(&cache)));
    assert_eq!(work(&result), (3, 3));
}

#[test]
fn tiny_paths_round_trip_exactly() {
    use usvg::tiny_skia_path::PathBuilder;
    let mut builder = PathBuilder::new();
    builder.move_to(0.0, 0.0);
    builder.line_to(10.0, 0.0);
    builder.quad_to(12.0, 4.0, 10.0, 8.0);
    builder.close();
    builder.line_to(4.0, 4.0);
    builder.cubic_to(1.0, 2.0, 3.0, 4.0, 5.0, 6.5);
    builder.close();
    let path = builder.finish().unwrap();

    let mut writer = Writer::default();
    writer.tiny_path(&path);
    let decoded = Reader(&writer.0).tiny_path().unwrap();
    assert_eq!(decoded.verbs(), path.verbs());
    assert_eq!(decoded.points(), path.points());
    assert_eq!(decoded.bounds(), path.bounds());
}

#[test]
fn a_fully_cached_incremental_result_regenerates_correctly() {
    let project = Project::new("regenerate");
    run(project.options(Some(&project.cache())));
    let mut result = run(project.options(Some(&project.cache())));
    assert_eq!(work(&result), (0, 0));

    let edited = project.root.join("icons/add.svg");
    fs::write(&edited, svg(12)).unwrap();
    result
        .regenerate(
            &RegenerationFiles::Single(project.icons()),
            &[(
                edited.to_string_lossy().into_owned(),
                GlyphChange::Changed { name: None },
            )],
        )
        .unwrap();
    assert_eq!(fonts(&result), fonts(&run(project.options(None))));
}

#[test]
fn color_glyphs_are_recomputed_while_monochrome_glyphs_stay_cached() {
    let project = Project::new("color");
    fs::write(
        project.root.join("icons/star.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#ff9900" d="M2 2H18V20H2Z"/></svg>"##,
    )
    .unwrap();
    let options = |cache: Option<&Path>| GenerateWebfontsOptions {
        color_glyphs: Some(ColorGlyphSelection::Named(vec!["star".to_owned()])),
        types: Some(vec![FontType::Ttf, FontType::Woff2]),
        ..project.options(cache)
    };
    let bytes = |result: &GenerateWebfontsResult| {
        (
            result.fonts.ttf_font.clone(),
            result.fonts.woff2_font.clone(),
        )
    };
    run(options(Some(&project.cache())));

    // An option-only change misses level 1, so glyph work runs: only the color glyph repeats it.
    let mut changed = options(Some(&project.cache()));
    changed.format_options.as_mut().unwrap().woff2 = Some(Woff2FormatOptions {
        compression_quality: Some(9),
    });
    let warm = run(changed.clone());
    assert_eq!(work(&warm), (1, 1));
    changed.cache_location = None;
    assert_eq!(bytes(&warm), bytes(&run(changed)));
}

#[test]
fn svg_only_fonts_round_trip_glyphs_without_ttf_outlines() {
    let project = Project::new("svg-only");
    let options = |cache: Option<&Path>| GenerateWebfontsOptions {
        types: Some(vec![FontType::Svg]),
        font_name: Some("my icons".to_owned()),
        ..project.options(cache)
    };
    run(options(Some(&project.cache())));
    assert!(cache_files(&project.cache())[0].starts_with("my_icons."));

    let warm = run(options(Some(&project.cache())));
    assert_eq!(work(&warm), (0, 0));
    assert_eq!(warm.fonts.svg_font, run(options(None)).fonts.svg_font);
}

#[test]
fn a_failed_cache_write_leaves_no_temporary_file() {
    let project = Project::new("rename-fails");
    run(project.options(Some(&project.cache())));
    let file = project
        .cache()
        .join(cache_files(&project.cache()).pop().unwrap());
    // A directory where the cache file goes makes the final rename fail.
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();

    let result = run(project.options(Some(&project.cache())));
    assert_eq!(work(&result), (3, 3));
    assert!(file.is_dir());
    assert_eq!(cache_files(&project.cache()).len(), 1);
}

#[cfg(unix)]
#[test]
fn clear_cache_reports_unreadable_or_locked_locations() {
    use std::os::unix::fs::PermissionsExt;
    let project = Project::new("clear-errors");
    run(project.options(Some(&project.cache())));
    let file = project
        .cache()
        .join(cache_files(&project.cache()).pop().unwrap());
    assert!(clear_cache(&file).is_err());

    fs::set_permissions(project.cache(), fs::Permissions::from_mode(0o555)).unwrap();
    let result = clear_cache(project.cache());
    fs::set_permissions(project.cache(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err());
    assert!(file.exists());
}

#[test]
fn malformed_path_tags_decode_to_none() {
    // One verb with an unknown tag, then zero points.
    let mut writer = Writer::default();
    writer.len(1);
    writer.u8(9);
    writer.len(0);
    assert!(Reader(&writer.0).tiny_path().is_none());

    let mut writer = Writer::default();
    writer.len(1);
    writer.u8(9);
    assert!(Reader(&writer.0).bez_path().is_none());
}
