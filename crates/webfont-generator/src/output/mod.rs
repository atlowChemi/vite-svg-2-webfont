mod collect;
#[cfg(test)]
mod tests;
mod write;

use std::collections::HashMap;

use tokio::task::JoinSet;

use crate::result::GenerateWebfontsResult;

use collect::collect_write_outputs;
use write::{
    native_write_task_error, record_written_output, write_output_file,
    write_output_file_if_changed, write_output_file_sync,
};

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
