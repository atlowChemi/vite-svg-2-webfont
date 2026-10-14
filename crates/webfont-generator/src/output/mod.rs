pub(crate) mod collect;
#[cfg(all(test, feature = "fs"))]
mod tests;
#[cfg(feature = "fs")]
mod write;

#[cfg(feature = "fs")]
use std::collections::HashMap;

#[cfg(feature = "fs")]
use tokio::task::JoinSet;

#[cfg(feature = "fs")]
use crate::result::GenerateWebfontsResult;

#[cfg(feature = "fs")]
use collect::{CollectedOutput, OutputContents, OutputKind, collect_outputs};
#[cfg(feature = "fs")]
use write::{
    native_write_task_error, record_written_output, write_output_file,
    write_output_file_if_changed, write_output_file_sync,
};

#[cfg(feature = "fs")]
impl GenerateWebfontsResult {
    pub(crate) fn collect_outputs(&self) -> std::io::Result<Vec<CollectedOutput>> {
        let css = self
            .options
            .css
            .then(|| self.generate_css_shared(None))
            .transpose()?;
        let html = self
            .options
            .html
            .then(|| self.generate_html_shared(None))
            .transpose()?;
        Ok(collect_outputs(&self.fonts, css, html))
    }
}

#[cfg(feature = "fs")]
struct OutputFile {
    path: String,
    contents: OutputContents,
    skip_unchanged: bool,
}

#[cfg(feature = "fs")]
fn collect_write_outputs(result: &GenerateWebfontsResult) -> std::io::Result<Vec<OutputFile>> {
    Ok(result
        .collect_outputs()?
        .into_iter()
        .map(|output| {
            let (path, skip_unchanged) = match output.kind {
                OutputKind::Font(kind) => (
                    crate::input::default_output_dest(
                        &result.options.dest,
                        &result.options.font_name,
                        kind.as_extension(),
                    ),
                    false,
                ),
                OutputKind::Css => (result.options.css_dest.clone(), true),
                OutputKind::Html => (result.options.html_dest.clone(), true),
            };
            OutputFile {
                path,
                contents: output.contents,
                skip_unchanged,
            }
        })
        .collect())
}

/// Write every output to disk concurrently. For incremental results, also return hashes for
/// CSS/HTML outputs so a later `regenerate` can skip companion files unchanged from this initial
/// write.
#[cfg(feature = "fs")]
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

/// Synchronous counterpart to [`write_generate_webfonts_result`] used by [`GenerateWebfontsResult::regenerate`].
///
/// `regenerate` is sync (it borrows `&mut self`, which can't be held across an `.await` in a NAPI
/// async method), and the rebuild it follows is CPU-bound and already runs on the caller's thread,
/// so a handful of blocking `std::fs` writes here is simpler than introducing an async write path.
/// Font outputs are written directly after a real rebuild; only CSS/HTML are hash-checked because
/// they often remain byte-identical after a geometry-only edit.
#[cfg(feature = "fs")]
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
