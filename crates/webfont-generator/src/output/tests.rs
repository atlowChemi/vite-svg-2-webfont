use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use super::collect::OutputContents;
use super::write::output_hash;
use super::*;
use crate::result::FontOutputs;
use crate::test_helpers::{resolve_options, webfont_fixture};
use crate::{FontType, GenerateWebfontsOptions};

fn temp_root(name: &str) -> std::path::PathBuf {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "webfont-write-{name}-{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn output_contents_exposes_shared_text_shared_bytes_and_owned_bytes() {
    let bytes = Arc::new(vec![1, 2, 3]);
    let text = Arc::new("css".to_owned());

    assert_eq!(OutputContents::Bytes(bytes).as_bytes(), &[1, 2, 3]);
    assert_eq!(OutputContents::Text(text).as_bytes(), b"css");
    assert_eq!(OutputContents::Owned(vec![4, 5, 6]).as_bytes(), &[4, 5, 6]);
}

#[test]
fn records_output_hash_only_for_incremental_writes() {
    let mut untracked = None;
    record_written_output(&mut untracked, "icons.css", b"css");
    assert!(untracked.is_none());

    let mut tracked = Some(HashMap::new());
    record_written_output(&mut tracked, "icons.css", b"css");
    assert_eq!(tracked.unwrap()["icons.css"], output_hash(b"css"));
}

#[tokio::test]
async fn async_writer_creates_parents_and_reports_filesystem_errors() {
    let root = temp_root("async");
    let output = root.join("nested/font.woff2");

    write_output_file(output.to_string_lossy().into_owned(), b"font")
        .await
        .unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), b"font");

    let blocker = root.join("blocker");
    std::fs::write(&blocker, b"file").unwrap();
    assert!(
        write_output_file(
            blocker.join("child").to_string_lossy().into_owned(),
            b"font"
        )
        .await
        .is_err()
    );

    let directory = root.join("directory");
    std::fs::create_dir(&directory).unwrap();
    assert!(
        write_output_file(directory.to_string_lossy().into_owned(), b"font")
            .await
            .is_err()
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn async_writer_dispatches_every_font_format() {
    let root = temp_root("formats");
    let options = resolve_options(GenerateWebfontsOptions {
        css: Some(false),
        dest: root.to_string_lossy().into_owned(),
        files: vec![webfont_fixture("add.svg")],
        font_name: Some("icons".to_owned()),
        html: Some(false),
        incremental: Some(true),
        types: Some(vec![
            FontType::Svg,
            FontType::Ttf,
            FontType::Woff,
            FontType::Woff2,
            FontType::Eot,
        ]),
        ..Default::default()
    });
    let result = GenerateWebfontsResult {
        cached: OnceLock::new(),
        carried_render: None,
        css_context: None,
        fonts: FontOutputs {
            svg_font: Some(Arc::new("svg".to_owned())),
            ttf_font: Some(Arc::new(b"ttf".to_vec())),
            woff_font: Some(Arc::new(b"woff".to_vec())),
            woff2_font: Some(Arc::new(b"woff2".to_vec())),
            eot_font: Some(Arc::new(b"eot".to_vec())),
        },
        html_context: None,
        options: Arc::new(options),
        regeneration_state: Arc::new(Mutex::new(None)),
        source_files: Arc::new(Vec::new()),
    };

    assert!(
        write_generate_webfonts_result(&result)
            .await
            .unwrap()
            .unwrap()
            .is_empty()
    );
    for (extension, expected) in [
        ("svg", b"svg".as_slice()),
        ("ttf", b"ttf".as_slice()),
        ("woff", b"woff".as_slice()),
        ("woff2", b"woff2".as_slice()),
        ("eot", b"eot".as_slice()),
    ] {
        assert_eq!(
            std::fs::read(root.join(format!("icons.{extension}"))).unwrap(),
            expected
        );
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn reports_panicking_write_tasks_as_io_errors() {
    let error = tokio::spawn(async { panic!("write task panic") })
        .await
        .unwrap_err();
    let error = native_write_task_error(error);

    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    assert!(error.to_string().contains("Native write task failed"));
    assert!(error.to_string().contains("write task panic"));
}

#[test]
fn sync_writer_skips_known_content_and_preserves_hash_after_failure() {
    let root = temp_root("sync");
    let font = root.join("fonts/icons.woff2");
    write_output_file_sync(font.to_string_lossy().into_owned(), b"font").unwrap();
    assert_eq!(std::fs::read(font).unwrap(), b"font");

    let output = root.join("nested/icons.css");
    let path = output.to_string_lossy().into_owned();
    let mut written = HashMap::new();

    write_output_file_if_changed(&mut written, path.clone(), b"first").unwrap();
    assert_eq!(written.get(&path), Some(&output_hash(b"first")));

    std::fs::write(&output, b"sentinel").unwrap();
    write_output_file_if_changed(&mut written, path.clone(), b"first").unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), b"sentinel");

    write_output_file_if_changed(&mut written, path.clone(), b"second").unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), b"second");
    assert_eq!(written.get(&path), Some(&output_hash(b"second")));

    let directory = root.join("directory");
    std::fs::create_dir(&directory).unwrap();
    let directory = directory.to_string_lossy().into_owned();
    written.insert(directory.clone(), output_hash(b"old"));
    assert!(write_output_file_if_changed(&mut written, directory.clone(), b"new").is_err());
    assert_eq!(written.get(&directory), Some(&output_hash(b"old")));

    std::fs::remove_dir_all(root).unwrap();
}
