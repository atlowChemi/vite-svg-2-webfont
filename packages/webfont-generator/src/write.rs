use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::task::JoinSet;

use crate::default_output_dest;
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
        result.map_err(|error| {
            std::io::Error::other(format!("Native write task failed: {error}"))
        })??;
    }

    Ok(written)
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
    result: &mut GenerateWebfontsResult,
) -> std::io::Result<()> {
    let outputs = collect_write_outputs(result)?;

    // Write everything, updating `written_outputs` in place. Updating in place (rather than
    // taking the map and restoring it at the end) means a mid-write failure keeps the hashes of the
    // outputs already written, so a retry doesn't needlessly rewrite them.
    let written = &mut result.written_outputs;
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
