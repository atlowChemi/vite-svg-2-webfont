use std::{fs, process::Command};

#[test]
fn tracks_atomic_publication_and_deletion() {
    let temp = tempfile::tempdir().unwrap();
    let dir = fs::canonicalize(temp.path()).unwrap();
    fs::write(dir.join("input"), "value").unwrap();
    // Python exercises libc wrappers, including relative directory descriptors.
    let output = Command::new(env!("CARGO_BIN_EXE_cache-audit"))
        .current_dir(&dir)
        .args(["--", "python3", "-c", "import os\nf=os.open('.', os.O_RDONLY)\nos.link('input','linked',src_dir_fd=f,dst_dir_fd=f)\nos.rename('linked','final',src_dir_fd=f,dst_dir_fd=f)\nos.unlink('input',dir_fd=f)\nos.close(f)"])
        .output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let manifest: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let accesses = manifest["accesses"].as_array().unwrap();
    for name in ["linked", "final", "input"] {
        assert!(
            accesses
                .iter()
                .any(|a| a["path"].as_str() == dir.join(name).to_str()
                    && ["write", "read_write"].contains(&a["kind"].as_str().unwrap())),
            "missing mutation for {name}: {manifest}"
        );
    }
    assert_eq!(fs::read_to_string(dir.join("final")).unwrap(), "value");
    assert!(!dir.join("input").exists());
}
