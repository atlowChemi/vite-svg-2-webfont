use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::task::JoinSet;

use crate::input::default_output_dest;
use crate::templates::{render_css_with_hbs_context, render_html_with_hbs_context};
use crate::types::GenerateWebfontsResult;

enum OutputContents {
    Bytes(Arc<Vec<u8>>),
    Text(Arc<String>),
    Owned(Vec<u8>),
}

impl OutputContents {
    fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) => bytes.as_slice(),
            Self::Text(text) => text.as_bytes(),
            Self::Owned(bytes) => bytes.as_slice(),
        }
    }
}

impl AsRef<[u8]> for OutputContents {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

struct OutputFile {
    path: String,
    contents: OutputContents,
    skip_unchanged: bool,
}

fn collect_write_outputs(result: &GenerateWebfontsResult) -> std::io::Result<Vec<OutputFile>> {
    let mut outputs = Vec::new();
    let font_name = result.options.font_name.clone();
    let dest = result.options.dest.clone();

    if let Some(svg_font) = &result.fonts.svg_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "svg"),
            contents: OutputContents::Text(Arc::clone(svg_font)),
            skip_unchanged: false,
        });
    }
    if let Some(ttf_font) = &result.fonts.ttf_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "ttf"),
            contents: OutputContents::Bytes(Arc::clone(ttf_font)),
            skip_unchanged: false,
        });
    }
    if let Some(woff_font) = &result.fonts.woff_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "woff"),
            contents: OutputContents::Bytes(Arc::clone(woff_font)),
            skip_unchanged: false,
        });
    }
    if let Some(woff2_font) = &result.fonts.woff2_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "woff2"),
            contents: OutputContents::Bytes(Arc::clone(woff2_font)),
            skip_unchanged: false,
        });
    }
    if let Some(eot_font) = &result.fonts.eot_font {
        outputs.push(OutputFile {
            path: default_output_dest(&dest, &font_name, "eot"),
            contents: OutputContents::Bytes(Arc::clone(eot_font)),
            skip_unchanged: false,
        });
    }

    // Only render CSS/HTML templates when those files need to be written.
    if result.options.css || result.options.html {
        let cached = result.get_cached_io()?;
        if result.options.css {
            let ctx = cached.css_hbs_context.lock().unwrap();
            let css = render_css_with_hbs_context(&cached.shared, &ctx, &cached.css_context)?;
            drop(ctx);
            outputs.push(OutputFile {
                path: result.options.css_dest.clone(),
                contents: OutputContents::Owned(css.into_bytes()),
                skip_unchanged: true,
            });
        }
        if result.options.html {
            let ctx = cached.html_hbs_context.lock().unwrap();
            let html = render_html_with_hbs_context(
                cached.html_registry.as_ref(),
                &ctx,
                &cached.html_context,
            )?;
            drop(ctx);
            outputs.push(OutputFile {
                path: result.options.html_dest.clone(),
                contents: OutputContents::Owned(html.into_bytes()),
                skip_unchanged: true,
            });
        }
    }

    Ok(outputs)
}

/// Write every output to disk concurrently. For incremental results, also return hashes for
/// CSS/HTML outputs so a later `regenerate` can skip companion files unchanged from this initial
/// write.
pub(crate) async fn write_generate_webfonts_result(
    result: &GenerateWebfontsResult,
) -> std::io::Result<Option<HashMap<String, [u8; 16]>>> {
    let mut tasks = JoinSet::new();
    let mut written = result.options.incremental.then(HashMap::new);

    for output in collect_write_outputs(result)? {
        if output.skip_unchanged {
            record_written_output(&mut written, &output.path, output.contents.as_bytes());
        }
        tasks.spawn(async move { write_output_file(output.path, output.contents).await });
    }

    while let Some(result) = tasks.join_next().await {
        result.map_err(native_write_task_error)??;
    }

    Ok(written)
}

fn native_write_task_error(error: tokio::task::JoinError) -> std::io::Error {
    std::io::Error::other(format!("Native write task failed: {error}"))
}

fn record_written_output(
    written: &mut Option<HashMap<String, [u8; 16]>>,
    path: &str,
    bytes: &[u8],
) {
    if let Some(written) = written {
        written.insert(path.to_owned(), output_hash(bytes));
    }
}

async fn write_output_file(path: String, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    if let Some(parent) = Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(path, contents).await
}

/// Content hash used to skip rewriting unchanged CSS/HTML companion files.
fn output_hash(bytes: &[u8]) -> [u8; 16] {
    md5::compute(bytes).0
}

/// Synchronous counterpart to [`write_generate_webfonts_result`] used by [`GenerateWebfontsResult::regenerate`].
///
/// `regenerate` is sync (it borrows `&mut self`, which can't be held across an `.await` in a NAPI
/// async method), and the rebuild it follows is CPU-bound and already runs on the caller's thread,
/// so a handful of blocking `std::fs` writes here is simpler than introducing an async write path.
/// Font outputs are written directly after a real rebuild; only CSS/HTML are hash-checked because
/// they often remain byte-identical after a geometry-only edit.
pub(crate) fn write_generate_webfonts_result_sync(
    result: &GenerateWebfontsResult,
    written: &mut HashMap<String, [u8; 16]>,
) -> std::io::Result<()> {
    let outputs = collect_write_outputs(result)?;

    // Write everything, updating `written_outputs` in place. Updating in place (rather than
    // taking the map and restoring it at the end) means a mid-write failure keeps the hashes of the
    // outputs already written, so a retry doesn't needlessly rewrite them.
    for output in outputs {
        if output.skip_unchanged {
            write_output_file_if_changed(written, output.path, output.contents.as_bytes())?;
        } else {
            write_output_file_sync(output.path, output.contents.as_bytes())?;
        }
    }

    Ok(())
}

fn write_output_file_sync(path: String, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

/// Write `bytes` to `path` unless an identical payload was written there before (tracked in `written`).
fn write_output_file_if_changed(
    written: &mut HashMap<String, [u8; 16]>,
    path: String,
    bytes: &[u8],
) -> std::io::Result<()> {
    let hash = output_hash(bytes);

    if written.get(&path) == Some(&hash) {
        return Ok(());
    }
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, bytes)?;
    written.insert(path, hash);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    use super::*;
    use crate::test_helpers::{resolve_options, webfont_fixture};
    use crate::types::FontOutputs;
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
}
