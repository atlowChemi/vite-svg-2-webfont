use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    ffi::OsString,
    fs::{self, File},
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use crate::{
    graph,
    validator::{self, Result},
};
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use pnpm_workspace_task_scheduler::{SequenceTasksOptions, TaskGraph, TaskKey, sequence_tasks};
use serde_json::{Value, json};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

// Also stop surviving descendants when a tool exits without waiting for them.
struct ProcessGroup(Pid);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        let _ = killpg(self.0, Signal::SIGKILL);
    }
}

pub async fn execute(command: &mut Command, cancel: &CancellationToken) -> Result<i32> {
    if cancel.is_cancelled() {
        return Ok(130);
    }
    let mut child = command.process_group(0).kill_on_drop(true).spawn()?;
    let group = ProcessGroup(Pid::from_raw(child.id().ok_or("missing child PID")? as i32));
    tokio::select! {
        status = child.wait() => {
            let status = status?;
            Ok(status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1)))
        }
        _ = cancel.cancelled() => {
            let _ = killpg(group.0, Signal::SIGTERM);
            if tokio::time::timeout(Duration::from_secs(5), child.wait()).await.is_err() {
                let _ = killpg(group.0, Signal::SIGKILL);
                child.wait().await?;
            }
            Ok(130)
        }
    }
}

fn report_name(key: &TaskKey, root: &Path) -> Result<String> {
    let project = key.project.strip_prefix(root)?.to_string_lossy();
    let label = if project.is_empty() {
        key.task_name.clone()
    } else {
        format!("{project}#{}", key.task_name)
    };
    // Preserve existing root-task report names. Reject collisions before running.
    Ok(label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect())
}

fn key_label(key: &TaskKey, root: &Path) -> Result<String> {
    let project = key.project.strip_prefix(root)?;
    Ok(format!(
        "{}#{}",
        if project.as_os_str().is_empty() {
            ".".into()
        } else {
            project.to_string_lossy()
        },
        key.task_name
    ))
}

pub async fn run(
    mut args: impl Iterator<Item = OsString>,
    cancel: CancellationToken,
) -> Result<i32> {
    let mut root = PathBuf::from(".");
    let mut task = None;
    let mut pipeline = None;
    let mut all = false;
    let mut policy = None;
    let mut reports = env::var_os("CACHE_AUDIT_REPORTS").map(PathBuf::from);
    let mut seen = HashSet::new();
    while let Some(option) = args.next() {
        if !seen.insert(option.clone()) {
            return Err("duplicate option".into());
        }
        match option.to_str() {
            Some("--all") => all = true,
            Some("--workspace" | "--reports" | "--policy" | "--task" | "--pipeline") => {
                let value = args.next().ok_or("missing audit option value")?;
                match option.to_str().unwrap() {
                    "--workspace" => root = value.into(),
                    "--reports" => reports = Some(value.into()),
                    "--policy" => policy = Some(PathBuf::from(value)),
                    "--task" => task = Some(value.into_string().map_err(|_| "non-UTF-8 task")?),
                    _ => pipeline = Some(value.into_string().map_err(|_| "non-UTF-8 pipeline")?),
                }
            }
            _ => return Err(format!("unknown audit option: {}", option.to_string_lossy()).into()),
        }
    }
    if usize::from(all) + usize::from(task.is_some()) + usize::from(pipeline.is_some()) != 1 {
        return Err("audit requires exactly one of --task NAME, --pipeline NAME, or --all".into());
    }
    let root = root.canonicalize()?;
    let reports = reports.unwrap_or_else(|| root.join("artifacts/cache-audit"));
    let policy_files = match policy {
        Some(path) => vec![path.canonicalize()?],
        None => ["native", "build"]
            .iter()
            .map(|name| root.join(format!("crates/cache-audit/policy.{name}.yaml")))
            .filter(|path| path.exists())
            .collect(),
    };
    let mut policies = BTreeMap::new();
    for file in policy_files {
        for key in validator::policy_tasks(&file)? {
            if policies.insert(key, file.clone()).is_some() {
                return Err("duplicate task across policies".into());
            }
        }
    }
    let mut plan = TaskGraph::new();
    if all {
        if policies.is_empty() {
            return Err("--all requires reviewed policy tasks".into());
        }
        // Only reviewed policy targets seed --all; servers and publishing are
        // never selected merely because a pipeline exists.
        for label in policies.keys() {
            let (_, task) = label.rsplit_once('#').ok_or("invalid policy task key")?;
            for graph in graph::plans(&root, None, Some(task))?.into_values() {
                plan.extend(graph);
            }
        }
    } else {
        for graph in graph::plans(&root, pipeline.as_deref(), task.as_deref())?.into_values() {
            plan.extend(graph);
        }
    }
    let targets: HashSet<_> = plan
        .iter()
        .filter_map(|(key, node)| {
            if node.scripts.is_empty() {
                return None;
            }
            let selected = if all {
                policies.contains_key(&key_label(key, &root).ok()?)
            } else if let Some(task) = &task {
                &key.task_name == task
            } else {
                true
            };
            selected.then_some(key.clone())
        })
        .collect();
    if targets.is_empty() {
        return Err("selection has no executable audit tasks".into());
    }
    if all {
        for label in policies.keys() {
            if !targets
                .iter()
                .any(|key| key_label(key, &root).is_ok_and(|key| &key == label))
            {
                return Err(format!("policy task is not executable: {label}").into());
            }
        }
    }
    let mut needed = HashSet::new();
    let mut pending: Vec<_> = targets.iter().cloned().collect();
    while let Some(key) = pending.pop() {
        if needed.insert(key.clone()) {
            pending.extend(plan[&key].dependencies.iter().cloned());
        }
    }
    plan.retain(|key, _| needed.contains(key));
    let order = sequence_tasks(
        &mut plan,
        &SequenceTasksOptions {
            workspace_dir: &root,
            ignore_cycles: false,
            emit: |_| {},
        },
    )?;
    let mut names = HashSet::new();
    for key in &order {
        if !plan[key].scripts.is_empty() && !names.insert(report_name(key, &root)?) {
            return Err("task report filename collision".into());
        }
    }
    fs::create_dir_all(&reports)?;
    let reports = reports.canonicalize()?;
    for key in &order {
        if plan[key].scripts.is_empty() {
            continue;
        }
        for suffix in ["json", "log", "status.json", "addon.log"] {
            match fs::remove_file(reports.join(format!("{}.{suffix}", report_name(key, &root)?))) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    fs::write(reports.join("task-audits.json"), b"[]")?;
    let mut environment = BTreeMap::<String, String>::new();
    // Resolve placement once, outside observation. Raw and nested pnpm scripts
    // must not install or mutate the lockfile while auditing a build.
    let config = pnpm_config::Config::default().current::<pnpm_config::Host>(&root)?;
    environment.insert(
        "PNPM_CONFIG_STORE_DIR".into(),
        config
            .store_dir
            .root()
            .to_str()
            .ok_or("non-UTF-8 pnpm store path")?
            .into(),
    );
    environment.insert("PNPM_CONFIG_VERIFY_DEPS_BEFORE_RUN".into(), "error".into());
    let fingerprint = root.join("scripts/next-pipeline.mjs");
    if fingerprint.exists() {
        let output = tempfile::NamedTempFile::new()?;
        let code = execute(
            Command::new("node")
                .arg(fingerprint)
                .arg("--fingerprint")
                .current_dir(&root)
                .stdout(Stdio::from(output.reopen()?)),
            &cancel,
        )
        .await?;
        if code != 0 {
            return Ok(code);
        }
        let value: Value = serde_json::from_reader(output.reopen()?)?;
        environment.insert(
            "CACHE_TOOLCHAIN_FINGERPRINT".into(),
            value["fingerprint"]
                .as_str()
                .ok_or("missing fingerprint")?
                .into(),
        );
        for (key, value) in value["fileInputs"]
            .as_object()
            .ok_or("missing file fingerprints")?
        {
            environment.insert(
                key.clone(),
                value.as_str().ok_or("invalid file fingerprint")?.into(),
            );
        }
    }
    let mut outcomes = HashMap::<TaskKey, bool>::new();
    let mut results = Vec::new();
    let mut failed = false;
    // ponytail: serial execution keeps filesystem attribution deterministic;
    // use pnpm's concurrent scheduler only after auditing overlapping outputs.
    for key in order {
        let node = &plan[&key];
        let blocked = node
            .dependencies
            .iter()
            .any(|dep| outcomes.get(dep) != Some(&true));
        if blocked || cancel.is_cancelled() {
            outcomes.insert(key.clone(), false);
            if !node.scripts.is_empty() {
                results.push(json!({"project": if key.project == root { Path::new(".") } else { key.project.strip_prefix(&root)? }, "task": key.task_name, "status": if cancel.is_cancelled() { "cancelled" } else { "blocked" }, "exitCode": null}));
                failed = true;
            }
            continue;
        }
        if node.scripts.is_empty() {
            outcomes.insert(key, true);
            continue;
        }
        let name = report_name(&key, &root)?;
        let started = Instant::now();
        let audited = targets.contains(&key);
        let result = execute_task(
            &root,
            &reports,
            &key,
            &name,
            audited,
            policies.get(&key_label(&key, &root)?),
            &environment,
            &cancel,
        )
        .await;
        let (code, command_code, error) = match result {
            Ok((code, command_code)) => (code, command_code, None),
            Err(error) => (1, None, Some(error.to_string())),
        };
        let success = command_code == Some(0);
        outcomes.insert(key.clone(), success);
        failed |= code != 0;
        let status = json!({"project": if key.project == root { Path::new(".") } else { key.project.strip_prefix(&root)? }, "task": key.task_name, "audited": audited,
            "status": if cancel.is_cancelled() { "cancelled" } else if success { if code == 0 { "passed" } else { "findings" } } else { "failed" },
            "exitCode": code, "commandExitCode": command_code, "seconds": started.elapsed().as_secs_f64(), "error": error});
        fs::write(
            reports.join(format!("{name}.status.json")),
            serde_json::to_vec_pretty(&status)?,
        )?;
        results.push(status);
        fs::write(
            reports.join("task-audits.json"),
            serde_json::to_vec_pretty(&results)?,
        )?;
        eprintln!(
            "{}: exit {code}; evidence {}",
            key_label(&key, &root)?,
            reports.display()
        );
    }
    fs::write(
        reports.join("task-audits.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    if cancel.is_cancelled() {
        return Ok(130);
    }
    // Preserve both findings and command failures for an explicitly selected task.
    if task.is_some()
        && targets.len() == 1
        && let Some(code) = results
            .iter()
            .find(|r| r["audited"] == true)
            .and_then(|r| r["exitCode"].as_i64())
    {
        return Ok(code as i32);
    }
    Ok(i32::from(failed))
}

#[allow(clippy::too_many_arguments)]
async fn execute_task(
    root: &Path,
    reports: &Path,
    key: &TaskKey,
    name: &str,
    audited: bool,
    policy: Option<&PathBuf>,
    environment: &BTreeMap<String, String>,
    cancel: &CancellationToken,
) -> Result<(i32, Option<i32>)> {
    let log = File::create(reports.join(format!("{name}.log")))?;
    // Tool installation belongs outside the observed WASM compilation.
    if audited && key.project == root && key.task_name == "next:wasm" {
        let code = execute(
            Command::new("node")
                .args([
                    "--input-type=module",
                    "-e",
                    "import {setup} from './packages/webfont-browser/scripts/setup.mjs'; setup();",
                ])
                .current_dir(root)
                .envs(environment)
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log.try_clone()?)),
            cancel,
        )
        .await?;
        if code != 0 {
            return Ok((code, Some(code)));
        }
    }
    let target = if audited {
        Some(
            tempfile::Builder::new()
                .prefix("cache-audit-cargo-")
                .tempdir()?,
        )
    } else {
        None
    };
    let mut command = if audited {
        let mut command = Command::new(env::current_exe()?);
        command
            .arg("--workspace")
            .arg(root)
            .arg("--project")
            .arg(&key.project)
            .arg("--task")
            .arg(&key.task_name);
        if let Some(policy) = policy {
            command.arg("--policy").arg(policy);
        }
        command.args(["--", "pnpm", "run"]).arg(&key.task_name);
        command.stdout(Stdio::from(File::create(
            reports.join(format!("{name}.json")),
        )?));
        command
    } else {
        let mut command = Command::new("pnpm");
        command
            .arg("run")
            .arg(&key.task_name)
            .stdout(Stdio::from(log.try_clone()?));
        command
    };
    command
        .current_dir(&key.project)
        .envs(environment)
        .stderr(Stdio::from(log.try_clone()?));
    if let Some(target) = &target {
        command.env("CARGO_TARGET_DIR", target.path());
    }
    let mut code = execute(&mut command, cancel).await?;
    let command_code = if audited && [0, 2].contains(&code) {
        let report: Value =
            serde_json::from_slice(&fs::read(reports.join(format!("{name}.json")))?)?;
        Some(
            report["exit_code"]
                .as_i64()
                .ok_or("missing command exit code")? as i32,
        )
    } else {
        Some(code)
    };
    // Published addons must load without depending on compiler scratch files.
    drop(target);
    if audited && key.project == root && key.task_name == "next:native" && command_code == Some(0) {
        let addon_log = File::create(reports.join(format!("{name}.addon.log")))?;
        let addon = execute(
            Command::new("node")
                .args([
                    "--input-type=module",
                    "-e",
                    "await import('./packages/webfont-generator/binding.js')",
                ])
                .current_dir(root)
                .envs(environment)
                .stdout(Stdio::from(addon_log.try_clone()?))
                .stderr(Stdio::from(addon_log)),
            cancel,
        )
        .await?;
        if addon != 0 {
            code = addon;
            return Ok((code, Some(addon)));
        }
    }
    Ok((code, command_code))
}
