use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn audit(root: &Path, project: &str, command: &str) -> (Output, Value) {
    audit_env(root, project, command, &[])
}

fn audit_env(
    root: &Path,
    project: &str,
    command: &str,
    environment: &[(&str, &str)],
) -> (Output, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_cache-audit"))
        .current_dir(root)
        .envs(environment.iter().copied())
        .args([
            "--workspace",
            root.to_str().unwrap(),
            "--project",
            project,
            "--task",
            "build",
            "--policy",
            "policy.yaml",
            "--",
            "sh",
            "-c",
            command,
        ])
        .output()
        .unwrap();
    let value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    (output, value)
}

fn workspace(root: &Path, inputs: &str, outputs: &str) {
    fs::write(root.join("pnpm-workspace.yaml"), format!("tasks:\n  build:\n    inputs: {inputs}\n    outputs: {outputs}\n    dependsOn: [dependency]\n  dependency:\n    inputs: [dependency.txt]\n    outputs: [dependency.out]\n")).unwrap();
}

#[test]
fn validates_real_accesses_and_rejects_missing_declarations() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    assert!(
        Command::new("git")
            .arg("init")
            .arg(&root)
            .output()
            .unwrap()
            .status
            .success()
    );
    let manifest = json!({"packageManager":"pnpm@12.14.0", "scripts":{"build":"fixture", "dependency":"fixture"}});
    fs::write(root.join("package.json"), manifest.to_string()).unwrap();
    fs::write(root.join(".gitignore"), "output\ndependency.out\nignored\n").unwrap();
    for file in [
        "input",
        "dependency.txt",
        "dependency.out",
        "ignored",
        "unlisted",
    ] {
        fs::write(root.join(file), "fixture").unwrap();
    }
    let policy = "version: 1\ntasks:\n  '.#build':\n    missingInputs:\n      - {pattern: missing, reason: 'Optional regular-file fixture'}\n      - {pattern: ignored, reason: 'Ignored paths must still fail'}\n      - {pattern: '**/package.json', reason: 'Node package scope probes'}\n    ignore:\n      - {pattern: '/**', reason: 'System paths outside fixture'}\n      - {pattern: '.', reason: 'Working directory probes', kinds: [read, read_dir]}\n";
    fs::write(root.join("policy.yaml"), policy).unwrap();
    workspace(&root, "[input]", "[output]");
    for version in ["pnpm@12.13.0", "pnpm@12.14.0", "pnpm@12.99.1"] {
        let mut candidate = manifest.clone();
        candidate["packageManager"] = json!(version);
        fs::write(root.join("package.json"), candidate.to_string()).unwrap();
        assert!(audit(&root, ".", "cat input > output").0.status.success());
    }
    for version in [
        "pnpm@12.12.0",
        "pnpm@13.0.0",
        "pnpm@12.14.0-rc.1",
        "npm@12.14.0",
    ] {
        let mut candidate = manifest.clone();
        candidate["packageManager"] = json!(version);
        fs::write(root.join("package.json"), candidate.to_string()).unwrap();
        assert!(!audit(&root, ".", "touch should-not-run").0.status.success());
        assert!(!root.join("should-not-run").exists());
    }
    fs::write(root.join("package.json"), manifest.to_string()).unwrap();
    let (run, report) = audit(
        &root,
        ".",
        "cat input dependency.txt dependency.out > output",
    );
    assert!(run.status.success(), "{run:?}\n{report}");
    assert_eq!(report["audit"]["findings"], json!([]));
    fs::create_dir(root.join("sub")).unwrap();
    assert!(
        audit(&root, ".", "cat sub/../input > output")
            .0
            .status
            .success()
    );
    std::os::unix::fs::symlink("sub", root.join("jump")).unwrap();
    let (run, report) = audit(&root, ".", "cat jump/../input > output");
    assert_eq!(run.status.code(), Some(2));
    assert!(
        report["audit"]["findings"]
            .to_string()
            .contains("unresolved observed path")
    );

    let link_policy =
        format!("{policy}      - {{pattern: 'jump/**', kinds: [read], reason: 'Known alias'}}\n");
    fs::write(root.join("policy.yaml"), &link_policy).unwrap();
    // An exempt spelling must not hide a nonexempt symlink target.
    assert_eq!(
        audit(&root, ".", "cat jump/../input > output")
            .0
            .status
            .code(),
        Some(2)
    );
    fs::write(
        root.join("policy.yaml"),
        format!(
            "{link_policy}      - {{pattern: input, kinds: [read], reason: 'Known alias target'}}\n"
        ),
    )
    .unwrap();
    let (run, report) = audit(&root, ".", "cat jump/../input > output");
    assert!(run.status.success(), "{run:?}\n{report}");
    fs::write(root.join("policy.yaml"), policy).unwrap();

    workspace(&root, "[dependency.txt]", "[output]");
    let (run, report) = audit(&root, ".", "cat input > output");
    assert_eq!(run.status.code(), Some(2), "{run:?}\n{report}");
    assert!(report["audit"]["findings"].to_string().contains("/input"));

    workspace(&root, "[input]", "[]");
    let (run, report) = audit(&root, ".", "cat input > output");
    assert_eq!(run.status.code(), Some(2), "{run:?}");
    assert!(
        report["audit"]["findings"]
            .to_string()
            .contains("undeclared write")
    );

    // Read-only exemptions cover the read half of O_RDWR, never its write half.
    fs::write(root.join("policy.yaml"), format!("{policy}      - {{pattern: output, kinds: [read], reason: 'Generated output replacement'}}\n")).unwrap();
    workspace(&root, "[input]", "[output]");
    let (run, report) = audit(&root, ".", "exec 3<>output");
    assert!(run.status.success(), "{run:?}\n{report}");
    assert!(
        report["accesses"].as_array().unwrap().iter().any(
            |a| a["kind"] == "read_write" && a["path"].as_str() == root.join("output").to_str()
        )
    );
    workspace(&root, "[input]", "[]");
    let (run, report) = audit(&root, ".", "exec 3<>output");
    assert_eq!(run.status.code(), Some(2));
    assert!(
        report["audit"]["findings"]
            .to_string()
            .contains("undeclared write")
    );
    assert!(
        !report["audit"]["findings"]
            .to_string()
            .contains("read not covered")
    );
    fs::write(root.join("policy.yaml"), policy).unwrap();

    // Absent inputs are covered only when creating them changes the effective key.
    workspace(&root, "[input, unreviewed]", "[output]");
    assert_eq!(
        audit(&root, ".", "cat unreviewed >/dev/null 2>&1 || true")
            .0
            .status
            .code(),
        Some(2)
    );
    workspace(&root, "[input, missing, ignored]", "[output]");
    let (run, report) = audit(&root, ".", "cat missing >/dev/null 2>&1 || true");
    assert!(run.status.success(), "{run:?}\n{report}");
    let absent_input_count = report["audit"]["input_count"].as_u64().unwrap();
    workspace(&root, "[input]", "[output]");
    assert_eq!(
        audit(&root, ".", "cat missing >/dev/null 2>&1 || true")
            .0
            .status
            .code(),
        Some(2)
    );
    fs::remove_file(root.join("ignored")).unwrap();
    workspace(&root, "[input, missing, ignored]", "[output]");
    assert_eq!(
        audit(&root, ".", "cat ignored >/dev/null 2>&1 || true")
            .0
            .status
            .code(),
        Some(2)
    );
    // A create/read/delete sequence must not become a covered absence.
    let (run, report) = audit(
        &root,
        ".",
        "touch missing; cat missing >/dev/null; rm missing",
    );
    assert_eq!(run.status.code(), Some(2));
    assert!(
        report["audit"]["findings"]
            .to_string()
            .contains("read not covered")
    );
    fs::write(root.join("ignored"), "fixture").unwrap();
    // File-probe policy is not coverage of existing directory metadata.
    fs::create_dir(root.join("missing")).unwrap();
    assert_eq!(
        audit(&root, ".", "test -d missing").0.status.code(),
        Some(2)
    );
    fs::remove_dir(root.join("missing")).unwrap();
    // Once the optional input exists, its content is ordinary input coverage.
    fs::write(root.join("missing"), "present").unwrap();
    let (run, report) = audit(&root, ".", "cat missing >/dev/null");
    assert!(run.status.success(), "{run:?}\n{report}");
    assert_eq!(
        report["audit"]["input_count"].as_u64().unwrap(),
        absent_input_count + 1
    );
    workspace(&root, "[input]", "[output]");
    assert_eq!(
        audit(&root, ".", "cat missing >/dev/null").0.status.code(),
        Some(2)
    );

    // Pinned installation-file exemptions fail closed when contents change.
    let pinned_policy = format!(
        "{policy}      - {{pattern: ignored, kinds: [read], contents: fixture, reason: 'Pinned installation fixture'}}\n"
    );
    fs::write(root.join("policy.yaml"), &pinned_policy).unwrap();
    assert!(
        audit(&root, ".", "cat ignored >/dev/null")
            .0
            .status
            .success()
    );
    fs::write(root.join("ignored"), "changed").unwrap();
    let (run, _) = audit(&root, ".", "touch should-not-run");
    assert!(!run.status.success());
    assert!(!root.join("should-not-run").exists());
    fs::write(root.join("ignored"), "fixture").unwrap();
    assert!(
        !audit(&root, ".", "printf changed >ignored")
            .0
            .status
            .success()
    );
    fs::write(root.join("ignored"), "fixture").unwrap();
    fs::write(root.join("policy.yaml"), policy).unwrap();

    // Cleanup only covers prior declared outputs, never an omitted output glob.
    let cleanup = format!(
        "{policy}      - {{pattern: output-old, kinds: [read], reason: 'Cleanup metadata'}}\n    cleanupOutputs:\n      - {{pattern: 'output-*', reason: 'Retired artifact cleanup'}}\n"
    );
    fs::write(root.join("policy.yaml"), cleanup).unwrap();
    workspace(&root, "[input]", "['output*']");
    fs::write(root.join("output-old"), "old").unwrap();
    let (run, report) = audit(&root, ".", "rm output-old");
    assert!(run.status.success(), "{run:?}\n{report}");
    workspace(&root, "[input]", "[output]");
    fs::write(root.join("output-old"), "old").unwrap();
    let (run, report) = audit(&root, ".", "rm output-old");
    assert_eq!(run.status.code(), Some(2));
    assert!(
        report["audit"]["findings"]
            .to_string()
            .contains("undeclared write")
    );

    // Directory metadata exceptions cannot exempt a regular file at that path.
    fs::write(root.join("policy.yaml"), format!("{policy}      - {{pattern: metadata, directory: true, kinds: [read], reason: 'Directory lookup'}}\n")).unwrap();
    fs::create_dir(root.join("metadata")).unwrap();
    assert!(audit(&root, ".", "test -d metadata").0.status.success());
    fs::remove_dir(root.join("metadata")).unwrap();
    fs::write(root.join("metadata"), "file").unwrap();
    assert_eq!(
        audit(&root, ".", "cat metadata >/dev/null").0.status.code(),
        Some(2)
    );
    fs::write(root.join("policy.yaml"), policy).unwrap();

    // ENOTDIR lookups are covered only when their file ancestor is keyed.
    workspace(&root, "[input]", "[output]");
    assert!(
        audit(&root, ".", "cat input/package.json >/dev/null 2>&1 || true")
            .0
            .status
            .success()
    );
    workspace(&root, "[unlisted]", "[output]");
    assert_eq!(
        audit(&root, ".", "cat input/package.json >/dev/null 2>&1 || true")
            .0
            .status
            .code(),
        Some(2)
    );
    // Optional manifests within a producer-owned output tree require that edge.
    fs::create_dir(root.join("dependency-artifacts")).unwrap();
    fs::write(root.join("dependency-artifacts/result"), "prepared").unwrap();
    fs::write(
        root.join(".gitignore"),
        "output\ndependency.out\nignored\ndependency-artifacts/\n",
    )
    .unwrap();
    workspace(&root, "[input]", "[output]");
    let config = fs::read_to_string(root.join("pnpm-workspace.yaml"))
        .unwrap()
        .replace(
            "outputs: [dependency.out]",
            "outputs: ['dependency-artifacts/**']",
        );
    fs::write(root.join("pnpm-workspace.yaml"), &config).unwrap();
    assert!(
        audit(
            &root,
            ".",
            "cat dependency-artifacts/package.json >/dev/null 2>&1 || true"
        )
        .0
        .status
        .success()
    );
    fs::write(
        root.join("pnpm-workspace.yaml"),
        config.replace("dependsOn: [dependency]", "dependsOn: []"),
    )
    .unwrap();
    assert_eq!(
        audit(
            &root,
            ".",
            "cat dependency-artifacts/package.json >/dev/null 2>&1 || true"
        )
        .0
        .status
        .code(),
        Some(2)
    );

    // Uncacheable local configuration is permitted only with explicit cache: false.
    workspace(&root, "[input]", "[output]");
    let uncached_policy = format!(
        "{policy}      - {{pattern: ignored, kinds: [read], reason: 'Uncached local configuration'}}\n    requiresUncached: true\n"
    );
    fs::write(root.join("policy.yaml"), &uncached_policy).unwrap();
    let config = fs::read_to_string(root.join("pnpm-workspace.yaml")).unwrap();
    for cache in ["", "\n    cache: true", "\n    cache: false"] {
        fs::write(
            root.join("pnpm-workspace.yaml"),
            config.replace("outputs: [output]", &format!("outputs: [output]{cache}")),
        )
        .unwrap();
        let (result, _) = audit(&root, ".", "cat ignored >/dev/null");
        assert_eq!(result.status.success(), cache.ends_with("false"));
    }

    // Ignored local configuration needs a verified, explicitly keyed environment hash.
    workspace(&root, "[input]", "[output]");
    let config = fs::read_to_string(root.join("pnpm-workspace.yaml"))
        .unwrap()
        .replace(
            "outputs: [output]",
            "outputs: [output]\n    env: [TEST_FILE_HASH]",
        );
    fs::write(root.join("pnpm-workspace.yaml"), &config).unwrap();
    fs::write(root.join("policy.yaml"), format!("{policy}      - {{pattern: ignored, kinds: [read], environment: TEST_FILE_HASH, reason: 'Local configuration fingerprint'}}\n")).unwrap();
    fs::remove_file(root.join("ignored")).unwrap();
    assert!(
        audit_env(
            &root,
            ".",
            "cat ignored >/dev/null 2>&1 || true",
            &[("TEST_FILE_HASH", "missing")]
        )
        .0
        .status
        .success()
    );
    fs::write(root.join("ignored"), "abc").unwrap();
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert!(
        audit_env(
            &root,
            ".",
            "cat ignored >/dev/null",
            &[("TEST_FILE_HASH", hash)]
        )
        .0
        .status
        .success()
    );
    assert!(
        !audit_env(
            &root,
            ".",
            "touch should-not-run",
            &[("TEST_FILE_HASH", "missing")]
        )
        .0
        .status
        .success()
    );
    assert!(!root.join("should-not-run").exists());
    assert!(
        !audit_env(
            &root,
            ".",
            "printf xyz >ignored",
            &[("TEST_FILE_HASH", hash)]
        )
        .0
        .status
        .success()
    );
    fs::write(root.join("ignored"), "abc").unwrap();
    workspace(&root, "[input]", "[output]");
    assert!(
        !audit_env(
            &root,
            ".",
            "touch should-not-run",
            &[("TEST_FILE_HASH", hash)]
        )
        .0
        .status
        .success()
    );
    assert!(!root.join("should-not-run").exists());
    fs::write(root.join("ignored"), "fixture").unwrap();
    fs::write(root.join("policy.yaml"), policy).unwrap();

    // A + pattern cannot reintroduce Git-ignored inputs.
    workspace(&root, "[input, '+ignored']", "[output]");
    let (run, report) = audit(&root, ".", "cat ignored > output");
    assert_eq!(run.status.code(), Some(2));
    assert!(report["audit"]["findings"].to_string().contains("/ignored"));

    // An all-additive or empty input list retains pnpm's default input set.
    for inputs in ["['+input']", "[]"] {
        workspace(&root, inputs, "[output]");
        let (run, report) = audit(&root, ".", "cat unlisted > output");
        assert!(run.status.success(), "{run:?}\n{report}");
    }
    workspace(&root, "[input, '+unlisted']", "[output]");
    assert!(
        audit(&root, ".", "cat unlisted > output")
            .0
            .status
            .success()
    );

    // Must-not-cache evaluates effective selections even without an access.
    fs::write(root.join("policy.yaml"), format!("{policy}    neverCache:\n      - {{pattern: input, reason: 'Deliberate forbidden input'}}\n")).unwrap();
    let (run, report) = audit(&root, ".", "true");
    assert_eq!(run.status.code(), Some(2));
    assert!(
        report["audit"]["findings"]
            .to_string()
            .contains("must-not-cache")
    );
    fs::write(root.join("policy.yaml"), format!("{policy}    neverCache:\n      - {{pattern: output, reason: 'Deliberate forbidden output'}}\n")).unwrap();
    let (run, report) = audit(&root, ".", "true");
    assert_eq!(run.status.code(), Some(2));
    assert!(
        report["audit"]["findings"]
            .to_string()
            .contains("must-not-cache")
    );
    fs::write(root.join("policy.yaml"), policy).unwrap();
    assert_eq!(audit(&root, ".", "exit 23").0.status.code(), Some(23));

    // Parent input globs do not extend the owning project's Git enumeration.
    fs::create_dir(root.join("pkg")).unwrap();
    fs::write(root.join("pkg/dependency.out"), "prepared").unwrap();
    fs::write(root.join("pkg/package.json"), manifest.to_string()).unwrap();
    fs::write(
        root.join("policy.yaml"),
        policy.replace(".#build", "pkg#build"),
    )
    .unwrap();
    workspace(&root, "['../input']", "[]");
    let (run, report) = audit(&root, "pkg", "cat input >/dev/null");
    assert_eq!(run.status.code(), Some(2));
    assert!(report["audit"]["findings"].to_string().contains("/input"));

    // Do not silently claim support for dependency semantics not implemented yet.
    fs::write(
        root.join("pnpm-workspace.yaml"),
        "tasks:\n  build: {outputs: [], dependsOn: ['^build']}\n",
    )
    .unwrap();
    let (run, _) = audit(&root, ".", "touch should-not-run");
    assert!(!run.status.success());
    assert!(!root.join("should-not-run").exists());
}
