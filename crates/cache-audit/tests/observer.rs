use std::{fs, process::Command};

#[test]
fn observes_subprocess_accesses_and_preserves_failure() {
    let dir = std::env::temp_dir().join(format!("cache-audit-fixture-{}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("input"), "fixture").unwrap();
    let dir = fs::canonicalize(dir).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cache-audit"))
        .current_dir(&dir)
        .env("AUDIT_FIXTURE", "inherited")
        .args([
            "--",
            "sh",
            "-c",
            "test \"$AUDIT_FIXTURE\" = inherited || exit 99; sh -c 'cat input > output'; ls .; cat missing; exit 23",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(23), "{output:?}");
    let manifest: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(manifest["exit_code"], 23);
    let accesses = manifest["accesses"].as_array().unwrap();
    for (name, kinds) in [
        ("input", &["read", "read_write"][..]),
        ("output", &["write", "read_write"][..]),
        ("missing", &["read", "read_write"][..]),
        ("", &["read_dir"][..]),
    ] {
        let path = if name.is_empty() {
            dir.clone()
        } else {
            dir.join(name)
        };
        assert!(
            accesses.iter().any(|record| {
                record["path"].as_str() == path.to_str()
                    && kinds.contains(&record["kind"].as_str().unwrap())
            }),
            "missing {kinds:?} for {path:?}: {manifest}"
        );
    }
    fs::remove_dir_all(dir).unwrap();
}
