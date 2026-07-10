use std::path::{Path, PathBuf};

use serde_json::Value;

fn iconify_json_path(icon_set: &str) -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let direct = root
        .join("node_modules")
        .join("@iconify-json")
        .join(icon_set)
        .join("icons.json");
    if direct.exists() {
        return Some(direct);
    }

    let pnpm = root.join("node_modules/.pnpm");
    let prefix = format!("@iconify-json+{icon_set}@");
    for entry in std::fs::read_dir(pnpm).ok()? {
        let entry = entry.ok()?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.starts_with(&prefix) {
            continue;
        }
        let path = entry
            .path()
            .join("node_modules")
            .join("@iconify-json")
            .join(icon_set)
            .join("icons.json");
        if path.exists() {
            return Some(path);
        }
    }
    None
}

pub fn iconify_svgs(size: usize) -> Option<Vec<(String, String)>> {
    let icon_set = std::env::var("BENCH_ICON_SET").unwrap_or_else(|_| "simple-icons".to_owned());
    let path = iconify_json_path(&icon_set)?;
    let json: Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let default_width = json.get("width").and_then(Value::as_u64).unwrap_or(24);
    let default_height = json.get("height").and_then(Value::as_u64).unwrap_or(24);
    let icons = json.get("icons")?.as_object()?;
    let mut svgs = Vec::with_capacity(size);
    for (index, (name, icon)) in icons.iter().take(size).enumerate() {
        let width = icon
            .get("width")
            .and_then(Value::as_u64)
            .unwrap_or(default_width);
        let height = icon
            .get("height")
            .and_then(Value::as_u64)
            .unwrap_or(default_height);
        let body = icon.get("body")?.as_str()?;
        let view_width = width + (index as u64 % 5) * (width / 2).max(1);
        let view_height = height + ((index as u64 * 3) % 7) * (height / 3).max(1);
        svgs.push((
            name.replace(['/', ':'], "-"),
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {view_width} {view_height}\">{body}</svg>"
            ),
        ));
    }
    (svgs.len() == size).then_some(svgs)
}
