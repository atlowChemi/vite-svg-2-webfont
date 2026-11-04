use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn graph(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cache-audit"))
        .args(["graph", "--workspace"])
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn graphs_match_pnpm_and_reject_cycles_and_invalid_selections() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("package.json"),
        r#"{"name":"root","private":true}"#,
    )
    .unwrap();
    for (name, manifest) in [
        (
            "a",
            json!({"name":"a", "version":"1.0.0", "scripts":{"build":"echo a", "check":"echo check"}, "dependencies":{"b":"workspace:*"}}),
        ),
        (
            "b",
            json!({"name":"b", "version":"1.0.0", "dependencies":{"c":"catalog:"}}),
        ),
        (
            "c",
            json!({"name":"c", "version":"1.0.0", "scripts":{"build":"echo c", "check":"echo check"}}),
        ),
    ] {
        let dir = root.join("packages").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("package.json"), manifest.to_string()).unwrap();
    }
    let workspace = root.join("pnpm-workspace.yaml");
    for include_root in [false, true] {
        fs::write(&workspace, format!("packages: [packages/*]\nincludeWorkspaceRoot: {include_root}\ncatalog:\n  c: 'workspace:*'\npipelines:\n  default: [build]\n  explicit: [check]\ntasks:\n  check:\n    dependsOn: [build, ^check]\n")).unwrap();
        let output = graph(root, &["--all", "--verify-pnpm"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let graphs: Value = serde_json::from_slice(&output.stdout).unwrap();
        let tasks = graphs["default"]["tasks"].as_array().unwrap();
        assert_eq!(
            tasks.iter().any(|task| task["project"] == "."),
            include_root
        );
        let b = tasks
            .iter()
            .find(|task| task["project"] == "packages/b")
            .unwrap();
        assert_eq!(b["missingScript"], true);
        assert_eq!(
            b["dependsOn"],
            json!([{"project":"packages/c", "script":"build"}])
        );
        let single = graph(root, &["--pipeline", "explicit", "--verify-pnpm"]);
        assert!(
            single.status.success(),
            "{}",
            String::from_utf8_lossy(&single.stderr)
        );
        let task = graph(root, &["--task", "build"]);
        assert!(task.status.success());
        let task: Value = serde_json::from_slice(&task.stdout).unwrap();
        assert_eq!(task["build"], graphs["default"]);
    }
    for args in [
        vec![],
        vec!["--all", "--task", "build"],
        vec!["--pipeline", "absent"],
        vec!["--task", "absent"],
        vec!["--all", "--all"],
    ] {
        assert!(!graph(root, &args).status.success(), "{args:?}");
    }
    fs::write(workspace, "packages: [packages/*]\npipelines:\n  default: [build]\ntasks:\n  build:\n    dependsOn: [check]\n  check:\n    dependsOn: [build]\n").unwrap();
    let output = graph(root, &["--all"]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .to_lowercase()
            .contains("cycle")
    );
}
