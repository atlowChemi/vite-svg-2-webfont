use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use webfont_generator::{
    FontType, FormatOptions, GenerateWebfontsOptions, GenerateWebfontsResult, GlyphChange,
    TtfFormatOptions, Woff2FormatOptions,
};

mod support;

const TEST_TTF_TIMESTAMP: i64 = 1_700_000_000;

struct Fixtures {
    dir: PathBuf,
    paths: Vec<String>,
}

impl Drop for Fixtures {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("{name} must be an integer"))
        })
        .unwrap_or(default)
}

fn fixtures(icon_set: &str, size: usize) -> Fixtures {
    let icons = support::iconify_svgs(size).unwrap_or_else(|| {
        panic!(
            "real @iconify-json/{icon_set} fixtures are required; run vp install before profiling"
        )
    });
    let dir = std::env::temp_dir().join(format!(
        "webfont-generator-incremental-profile-{}-{}",
        std::process::id(),
        icon_set
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = icons
        .into_iter()
        .map(|(name, contents)| {
            let path = dir.join(format!("{name}.svg"));
            std::fs::write(&path, contents).unwrap();
            path.to_string_lossy().into_owned()
        })
        .collect();
    Fixtures { dir, paths }
}

fn options(paths: Vec<String>) -> GenerateWebfontsOptions {
    GenerateWebfontsOptions {
        css: Some(false),
        dest: std::env::temp_dir()
            .join("webfont-generator-incremental-profile-output")
            .to_string_lossy()
            .into_owned(),
        files: paths,
        font_name: Some("incremental-profile".to_owned()),
        format_options: Some(FormatOptions {
            ttf: Some(TtfFormatOptions {
                copyright: None,
                description: None,
                ts: Some(TEST_TTF_TIMESTAMP),
                url: None,
                version: None,
            }),
            woff2: Some(Woff2FormatOptions {
                compression_quality: Some(10),
            }),
            ..Default::default()
        }),
        html: Some(false),
        incremental: Some(true),
        ligature: Some(false),
        types: Some(vec![
            FontType::Svg,
            FontType::Ttf,
            FontType::Eot,
            FontType::Woff,
            FontType::Woff2,
        ]),
        write_files: Some(false),
        ..Default::default()
    }
}

fn output(name: &str, bytes: &[u8]) -> Value {
    json!({
        "name": name,
        "bytes": bytes.len(),
        "md5": format!("{:x}", md5::compute(bytes)),
    })
}

fn outputs(result: &GenerateWebfontsResult) -> Vec<Value> {
    vec![
        output("svg", result.svg_string().unwrap().as_bytes()),
        output("ttf", result.ttf_bytes().unwrap()),
        output("eot", result.eot_bytes().unwrap()),
        output("woff", result.woff_bytes().unwrap()),
        output("woff2", result.woff2_bytes().unwrap()),
    ]
}

fn main() {
    let icon_set = std::env::var("BENCH_ICON_SET").unwrap_or_else(|_| "simple-icons".to_owned());
    let size = env_usize("INCREMENTAL_PROFILE_SIZE", 600);
    let edits = env_usize("INCREMENTAL_PROFILE_EDITS", 100);
    assert!(
        edits.is_multiple_of(2),
        "INCREMENTAL_PROFILE_EDITS must be even"
    );

    let fixtures = fixtures(&icon_set, size);
    let changed_path = fixtures.paths[size / 2].clone();
    let original = std::fs::read_to_string(&changed_path).unwrap();
    let changed = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M1 1L23 1L12 23Z\"/></svg>";

    println!(
        "{}",
        json!({
            "event": "metadata",
            "source_sha": std::env::var("GITHUB_SHA").ok(),
            "package_version": env!("CARGO_PKG_VERSION"),
            "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "logical_cpus": std::thread::available_parallelism().map(usize::from).ok(),
            "rayon_threads": std::env::var("RAYON_NUM_THREADS").ok(),
            "icon_set": icon_set,
            "fixture_source": "iconify-json",
            "glyphs": size,
            "formats": ["svg", "ttf", "eot", "woff", "woff2"],
            "woff2_quality": 10,
            "edits": edits,
        })
    );

    let started = Instant::now();
    let mut result =
        webfont_generator::generate_sync(options(fixtures.paths.clone()), None).unwrap();
    let initial_outputs = outputs(&result);
    println!(
        "{}",
        json!({
            "event": "initial_generation",
            "elapsed_ms": started.elapsed().as_secs_f64() * 1_000.0,
            "outputs": initial_outputs,
        })
    );

    let started = Instant::now();
    for edit in 0..edits {
        std::fs::write(
            &changed_path,
            if edit % 2 == 0 { changed } else { &original },
        )
        .unwrap();
        result
            .regenerate(
                &fixtures.paths,
                &[(changed_path.clone(), GlyphChange::Changed { name: None })],
            )
            .unwrap();
    }
    let final_outputs = outputs(&result);
    assert_eq!(final_outputs, initial_outputs);
    println!(
        "{}",
        json!({
            "event": "after_edits",
            "elapsed_ms": started.elapsed().as_secs_f64() * 1_000.0,
            "outputs": final_outputs,
        })
    );

    std::fs::write(&changed_path, "<not-svg>").unwrap();
    let started = Instant::now();
    let error = result
        .regenerate(
            &fixtures.paths,
            &[(changed_path.clone(), GlyphChange::Changed { name: None })],
        )
        .expect_err("invalid SVG regeneration must fail");
    assert_eq!(outputs(&result), initial_outputs);
    std::fs::write(&changed_path, &original).unwrap();
    println!(
        "{}",
        json!({
            "event": "failed_regeneration",
            "elapsed_ms": started.elapsed().as_secs_f64() * 1_000.0,
            "error_kind": format!("{:?}", error.kind()),
        })
    );

    let hold_seconds = env_usize("INCREMENTAL_PROFILE_HOLD_SECONDS", 0);
    if hold_seconds > 0 {
        println!("{}", json!({ "event": "holding", "seconds": hold_seconds }));
        std::thread::sleep(Duration::from_secs(hold_seconds as u64));
    }
}
