use std::collections::HashMap;
use std::path::Path;

pub(super) fn native_write_task_error(error: tokio::task::JoinError) -> std::io::Error {
    std::io::Error::other(format!("Native write task failed: {error}"))
}

pub(super) fn record_written_output(
    written: &mut Option<HashMap<String, [u8; 16]>>,
    path: &str,
    bytes: &[u8],
) {
    if let Some(written) = written {
        written.insert(path.to_owned(), output_hash(bytes));
    }
}

pub(super) async fn write_output_file(
    path: String,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    if let Some(parent) = Path::new(&path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(path, contents).await
}

pub(super) fn write_output_file_sync(path: String, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

/// Write `bytes` to `path` unless an identical payload was written there before (tracked in `written`).
pub(super) fn write_output_file_if_changed(
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

/// Content hash used to skip rewriting unchanged CSS/HTML companion files.
pub(super) fn output_hash(bytes: &[u8]) -> [u8; 16] {
    md5::compute(bytes).0
}
