use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn run(root: &Path, reports: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cache-audit"))
        .args(["audit", "--workspace"])
        .arg(root)
        .arg("--reports")
        .arg(reports)
        .arg("--policy")
        .arg(root.join("policy.yaml"))
        .args(args)
        .output()
        .unwrap()
}

fn put_manifest(root: &Path, prepare: &str) {
    fs::write(root.join("package.json"), json!({
        "name": "runner-fixture", "private": true, "packageManager": "pnpm@12.14.0",
        "scripts": {
                "dependency": prepare,
            "build": "printf 'build\\n' >> trace; cat dep.out > final.out; printf '%s' \"$CARGO_TARGET_DIR\" > target-path",
            "independent": "printf 'independent\\n' >> trace; printf independent > independent.out",
            "dev": "touch must-not-run"
        }
    }).to_string()).unwrap();
}

#[test]
fn runs_prerequisites_once_attributes_audits_and_blocks_failed_descendants() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("workspace");
    let reports = temp.path().join("reports");
    fs::create_dir(&root).unwrap();
    assert!(
        Command::new("git")
            .arg("init")
            .arg(&root)
            .output()
            .unwrap()
            .status
            .success()
    );
    put_manifest(&root, "printf 'prepare\\n' >> trace; printf dep > dep.out");
    fs::write(root.join(".gitignore"), "trace\n*.out\ntarget-path\n").unwrap();
    fs::write(root.join("pnpm-workspace.yaml"), "packages: ['.']\npipelines:\n  proof: [build, independent]\n  server: [dev]\ntasks:\n  dependency: {cache: false, inputs: ['**'], outputs: [trace, dep.out], dependsOn: []}\n  build: {cache: false, inputs: ['**'], outputs: [trace, final.out, target-path], dependsOn: [dependency]}\n  independent: {cache: false, inputs: ['**'], outputs: [trace, independent.out], dependsOn: [dependency]}\n  dev: {cache: false, dependsOn: []}\n").unwrap();
    fs::write(root.join("policy.yaml"), "version: 1\ntasks:\n  '.#build': &policy\n    missingInputs:\n      - {pattern: '**/{package.json,package.json5,package.yaml,binding.gyp,.npmrc,.pnp.cjs,.pnpmfile.cjs,.pnpmfile.mjs}', reason: 'Optional fixture package configuration'}\n    neverCache:\n      - {pattern: 'node_modules', reason: 'Fixture package manager probes'}\n      - {pattern: 'node_modules/**', reason: 'Fixture package manager probes'}\n      - {pattern: '_tmp_*', reason: 'pnpm hardlink capability probes in uninstalled fixture'}\n    ignore:\n      - {pattern: '/**', reason: 'External fixture toolchain'}\n      - {pattern: '.', reason: 'Working directory probes', kinds: [read, read_dir]}\n  '.#independent': *policy\n").unwrap();
    let output = run(&root, &reports, &["--all"]);
    assert!(
        output.status.success(),
        "{}\n{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        fs::read_to_string(reports.join("build.json")).unwrap_or_default(),
        fs::read_to_string(reports.join("build.log")).unwrap_or_default()
    );
    let trace = fs::read_to_string(root.join("trace")).unwrap();
    assert_eq!(trace.lines().filter(|line| *line == "prepare").count(), 1);
    assert_eq!(trace.lines().count(), 3);
    assert_eq!(trace.lines().next(), Some("prepare"));
    assert!(!root.join("must-not-run").exists());
    let target = fs::read_to_string(root.join("target-path")).unwrap();
    assert!(!target.is_empty());
    assert!(!Path::new(&target).exists(), "fresh Cargo target leaked");
    let report: Value =
        serde_json::from_slice(&fs::read(reports.join("build.json")).unwrap()).unwrap();
    assert_eq!(report["exit_code"], 0);
    assert_eq!(report["audit"]["findings"], json!([]));
    assert!(
        !reports.join("dependency.json").exists(),
        "prerequisite should run without auditing"
    );
    let status: Value =
        serde_json::from_slice(&fs::read(reports.join("task-audits.json")).unwrap()).unwrap();
    assert_eq!(status.as_array().unwrap().len(), 3);

    let workspace = root.join("pnpm-workspace.yaml");
    let config = fs::read_to_string(&workspace).unwrap();
    let policy = fs::read_to_string(root.join("policy.yaml")).unwrap();
    fs::write(
        root.join("policy.yaml"),
        format!("{policy}  '.#dependency': *policy\n"),
    )
    .unwrap();
    let output = run(&root, &reports, &["--pipeline", "proof"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Findings in a successful producer must not block its consumers.
    fs::write(
        &workspace,
        config.replace("outputs: [trace, dep.out]", "outputs: [trace]"),
    )
    .unwrap();
    let output = run(&root, &reports, &["--pipeline", "proof"]);
    assert_eq!(output.status.code(), Some(1));
    let status: Value =
        serde_json::from_slice(&fs::read(reports.join("task-audits.json")).unwrap()).unwrap();
    assert!(
        status
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["commandExitCode"] == 0)
    );
    fs::write(root.join("policy.yaml"), policy).unwrap();
    // A single target preserves the low-level findings exit code.
    fs::write(
        &workspace,
        config.replace(
            "outputs: [trace, final.out, target-path]",
            "outputs: [trace, target-path]",
        ),
    )
    .unwrap();
    let output = run(&root, &reports, &["--task", "build"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(&workspace, &config).unwrap();

    put_manifest(&root, "exit 7");
    fs::remove_file(root.join("trace")).unwrap();
    let output = run(&root, &reports, &["--all"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!root.join("trace").exists());
    assert!(
        !reports.join("build.json").exists(),
        "stale successful report survived a blocked run"
    );
    let status: Value =
        serde_json::from_slice(&fs::read(reports.join("task-audits.json")).unwrap()).unwrap();
    assert_eq!(status[0]["exitCode"], 7);
    assert!(
        status
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .all(|row| row["status"] == "blocked")
    );
    let independent_config = config.replace(
        "independent.out], dependsOn: [dependency]",
        "independent.out], dependsOn: []",
    );
    fs::write(&workspace, independent_config).unwrap();
    let output = run(&root, &reports, &["--all"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(root.join("trace")).unwrap(),
        "independent\n"
    );
    fs::write(&workspace, &config).unwrap();
    let output = run(&root, &reports, &["--task", "dependency"]);
    assert_eq!(output.status.code(), Some(7));

    // Cancellation must kill the whole prerequisite process group and write a
    // final summary instead of leaving a background compiler alive.
    for observed in [false, true] {
        let _ = fs::remove_file(root.join("child.pid"));
        put_manifest(
            &root,
            if observed {
                "printf dep > dep.out"
            } else {
                "sleep 60 & echo $! > child.pid; wait"
            },
        );
        if observed {
            let path = root.join("package.json");
            let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            manifest["scripts"]["build"] = json!(
                "printf '%s' \"$CARGO_TARGET_DIR\" > target-path; sleep 60 & echo $! > child.pid; wait"
            );
            fs::write(path, manifest.to_string()).unwrap();
        }
        let mut child = Command::new(env!("CARGO_BIN_EXE_cache-audit"))
            .args(["audit", "--workspace"])
            .arg(&root)
            .arg("--reports")
            .arg(&reports)
            .args(["--task", "build"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !fs::read_to_string(root.join("child.pid"))
            .is_ok_and(|pid| pid.trim().parse::<i32>().is_ok())
        {
            if Instant::now() > deadline {
                child.kill().unwrap();
                panic!("prerequisite did not start");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(child.id() as i32),
            nix::sys::signal::Signal::SIGTERM,
        )
        .unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(130));
        let pid = fs::read_to_string(root.join("child.pid")).unwrap();
        let process = Path::new("/proc").join(pid.trim()).join("stat");
        // A dead descendant can briefly remain a zombie until container PID 1 reaps it.
        if let Ok(stat) = fs::read_to_string(process) {
            assert!(
                stat.split(')')
                    .nth(1)
                    .unwrap()
                    .trim_start()
                    .starts_with('Z')
            );
        }
        if observed {
            let target = fs::read_to_string(root.join("target-path")).unwrap();
            assert!(
                !Path::new(&target).exists(),
                "cancelled audit leaked its Cargo target"
            );
        }
    }
}
