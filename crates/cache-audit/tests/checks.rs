use serde_json::{Value, json};
use std::{fs, process::Command};

#[test]
fn isolated_cache_compatibility_and_negative_checks_preserve_source_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let reports = temp.path().join("reports");
    fs::create_dir_all(root.join("crates/cache-audit")).unwrap();
    fs::create_dir_all(root.join("crates/webfont-generator/src")).unwrap();
    fs::create_dir_all(root.join("packages/webfont-generator")).unwrap();
    fs::write(root.join("package.json"), json!({
        "name":"negative-fixture", "private":true, "packageManager":"pnpm@12.15.0",
        "scripts":{"next:native":"cat crates/webfont-generator/src/lib.rs > packages/webfont-generator/binding.d.ts; printf 'export {};' > packages/webfont-generator/binding.js"}
    }).to_string()).unwrap();
    fs::write(
        root.join("packages/webfont-generator/package.json"),
        "{\"type\":\"module\"}",
    )
    .unwrap();
    fs::write(root.join("crates/webfont-generator/src/lib.rs"), "source").unwrap();
    fs::write(
        root.join("packages/webfont-generator/binding.d.ts"),
        "original output",
    )
    .unwrap();
    fs::write(
        root.join(".gitignore"),
        "packages/webfont-generator/binding.*\nnode_modules/\n",
    )
    .unwrap();
    let yaml = "packages: ['.']\ninclude:\n  ./tasks.yaml: [tasks]\n";
    let tasks = "tasks:\n  next:native:\n    cache: false\n    dependsOn: []\n    inputs: [package.json, pnpm-workspace.yaml, tasks.yaml, 'packages/*/package.json', 'crates/webfont-generator/**']\n    outputs: [packages/webfont-generator/binding.js, packages/webfont-generator/binding.d.ts]\n";
    fs::write(root.join("tasks.yaml"), tasks).unwrap();
    fs::write(root.join("pnpm-workspace.yaml"), yaml).unwrap();
    fs::write(root.join("crates/cache-audit/policy.native.yaml"), "version: 1\ntasks:\n  '.#next:native':\n    ignore:\n      - {pattern: '/**', reason: 'External fixture paths'}\n").unwrap();
    let install = Command::new("pnpm")
        .args(["install", "--ignore-scripts"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    // A copied manifest may retain its mtime; 12.15 must still reject stale deps.
    let manifest_path = root.join("package.json");
    let original_manifest = fs::read(&manifest_path).unwrap();
    let modified = fs::metadata(&manifest_path).unwrap().modified().unwrap();
    let mut changed: Value = serde_json::from_slice(&original_manifest).unwrap();
    changed["dependencies"] = json!({"missing-fixture-dependency": "1.0.0"});
    fs::write(&manifest_path, changed.to_string()).unwrap();
    fs::File::options()
        .write(true)
        .open(&manifest_path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let stale = Command::new("pnpm")
        .args(["run", "next:native"])
        .env("PNPM_CONFIG_VERIFY_DEPS_BEFORE_RUN", "error")
        .current_dir(&root)
        .output()
        .unwrap();
    fs::write(&manifest_path, original_manifest).unwrap();
    assert!(!stale.status.success());
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&stale.stdout),
        String::from_utf8_lossy(&stale.stderr)
    );
    assert!(
        diagnostic.contains("ERR_PNPM_VERIFY_DEPS_BEFORE_RUN"),
        "{diagnostic}"
    );
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["add", "."])
            .current_dir(&root)
            .status()
            .unwrap()
            .success()
    );
    for mode in ["compat", "negative"] {
        let output = Command::new(env!("CARGO_BIN_EXE_cache-audit"))
            .arg(mode)
            .arg("--workspace")
            .arg(&root)
            .arg("--reports")
            .arg(&reports)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{mode}: {}\n{}",
            String::from_utf8_lossy(&output.stderr),
            fs::read_to_string(reports.join("compat.log")).unwrap_or_default()
        );
        let status: Value =
            serde_json::from_slice(&fs::read(reports.join(format!("{mode}.status.json"))).unwrap())
                .unwrap();
        assert_eq!(status["status"], "passed");
    }
    assert_eq!(
        fs::read_to_string(root.join("pnpm-workspace.yaml")).unwrap(),
        yaml
    );
    assert_eq!(fs::read_to_string(root.join("tasks.yaml")).unwrap(), tasks);
    assert_eq!(
        fs::read_to_string(root.join("packages/webfont-generator/binding.d.ts")).unwrap(),
        "original output"
    );
    assert!(!root.join("packages/webfont-generator/binding.js").exists());
    let output = Command::new("git")
        .args(["diff", "--exit-code"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success(), "source workspace changed");
}
