#![cfg(all(target_os = "linux", target_env = "gnu"))]
use std::{fs, process::Command};

#[test]
fn tracks_dynamic_and_static_syscall_mutations() {
    let temp = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(temp.path()).unwrap();
    for mode in ["dynamic", "static"] {
        let dir = base.join(mode);
        fs::create_dir(&dir).unwrap();
        let binary = dir.join("fixture");
        let mut cc = Command::new("cc");
        cc.arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mutations.c"
        ))
        .arg("-o")
        .arg(&binary);
        if mode == "static" {
            cc.arg("-static");
        }
        let built = cc
            .output()
            .expect("Linux mutation checks require a C compiler and static libc");
        assert!(built.status.success(), "{built:?}");
        let output = Command::new(env!("CARGO_BIN_EXE_cache-audit"))
            .current_dir(&dir)
            .arg("--")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(output.status.success(), "{mode}: {output:?}");
        let manifest: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let accesses = manifest["accesses"].as_array().unwrap();
        for name in [
            "source",
            "linked",
            "direct-link",
            "fd-link",
            "renamed",
            "direct-rename",
        ] {
            assert!(
                accesses
                    .iter()
                    .any(|a| a["path"].as_str() == dir.join(name).to_str()
                        && ["write", "read_write"].contains(&a["kind"].as_str().unwrap())),
                "{mode} missing {name}: {manifest}"
            );
        }
        assert_eq!(fs::read(dir.join("direct-rename")).unwrap(), b"data");
        assert!(!dir.join("source").exists());
        assert!(!dir.join("direct-link").exists());
    }
}
