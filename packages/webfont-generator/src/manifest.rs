//! CLI-only JSON configuration and manifest-relative input expansion.

use std::collections::HashSet;
use std::path::Path;

use webfont_generator::GenerateWebfontsOptions;

pub(crate) fn load(path: &Path) -> Result<GenerateWebfontsOptions, String> {
    let json = std::fs::read_to_string(path).map_err(|error| format!("config: {error}"))?;
    let mut deserializer = serde_json::Deserializer::from_str(&json);
    let mut options: GenerateWebfontsOptions =
        serde_path_to_error::deserialize(&mut deserializer).map_err(|error| error.to_string())?;
    deserializer
        .end()
        .map_err(|error| format!("config: {error}"))?;
    let base = path.parent().unwrap_or(Path::new("."));
    if options.dest.is_empty() {
        return Err("dest: must not be empty".to_owned());
    }
    options.dest = resolve(base, &options.dest);
    for (field, value) in [
        ("cssDest", &mut options.css_dest),
        ("htmlDest", &mut options.html_dest),
        ("cssTemplate", &mut options.css_template),
        ("htmlTemplate", &mut options.html_template),
    ] {
        if let Some(value) = value {
            if value.is_empty() {
                return Err(format!("{field}: must not be empty"));
            }
            *value = resolve(base, value);
        }
    }
    options.files = expand(base, &options.files, "files")?;
    if let Some(variants) = &mut options.variants {
        for (index, variant) in variants.iter_mut().enumerate() {
            variant.files = expand(base, &variant.files, &format!("variants[{index}].files"))?;
        }
    }
    Ok(options)
}

fn resolve(base: &Path, path: &str) -> String {
    base.join(path).to_string_lossy().into_owned()
}

fn expand(base: &Path, entries: &[String], field: &str) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        let location = format!("{field}[{index}]");
        if entry.is_empty() {
            return Err(format!("{location}: input path must not be empty"));
        }
        let path = base.join(entry);
        let metadata =
            std::fs::metadata(&path).map_err(|error| format!("{location}: {entry}: {error}"))?;
        let candidates = if metadata.is_dir() {
            let mut files = Vec::new();
            for item in std::fs::read_dir(&path).map_err(|error| format!("{location}: {error}"))? {
                let item = item.map_err(|error| format!("{location}: {error}"))?;
                let path = item.path();
                if path.extension().and_then(|ext| ext.to_str()) == Some("svg") && path.is_file() {
                    files.push(path);
                }
            }
            files.sort();
            files
        } else if metadata.is_file() {
            vec![path]
        } else {
            return Err(format!("{location}: {entry}: expected a file or directory"));
        };
        for path in candidates {
            let normalized = path
                .canonicalize()
                .map_err(|error| format!("{location}: {error}"))?;
            if !seen.insert(normalized) {
                return Err(format!(
                    "{location}: duplicate input path: {}",
                    path.display()
                ));
            }
            // Keep the supplied filename for glyph naming, including symlink aliases.
            paths.push(path.to_string_lossy().into_owned());
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expansion_preserves_entry_order_and_sorts_only_local_svg_files() {
        let root = std::env::temp_dir().join(format!("manifest-expansion-{}", std::process::id()));
        std::fs::create_dir_all(root.join("icons/nested")).unwrap();
        for name in [
            "first.svg",
            "last.svg",
            "icons/z.svg",
            "icons/a.svg",
            "icons/ignored.SVG",
            "icons/nested/hidden.svg",
        ] {
            std::fs::write(root.join(name), "").unwrap();
        }
        let paths = expand(
            &root,
            &["last.svg".into(), "icons".into(), "first.svg".into()],
            "files",
        )
        .unwrap();
        let names: Vec<_> = paths
            .iter()
            .map(|path| Path::new(path).file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, ["last.svg", "a.svg", "z.svg", "first.svg"]);
        assert!(
            expand(&root, &["*.svg".into()], "files")
                .unwrap_err()
                .contains("files[0]")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
