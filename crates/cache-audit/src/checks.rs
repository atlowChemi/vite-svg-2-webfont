use crate::{runner, validator::Result};
use serde_json::{Value, json};
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

async fn checked(command: &mut Command, cancel: &CancellationToken) -> Result<()> {
    let code = runner::execute(command, cancel).await?;
    if code != 0 {
        return Err(format!("validation command failed ({code})").into());
    }
    Ok(())
}

pub async fn run(
    mut args: impl Iterator<Item = OsString>,
    cancel: CancellationToken,
) -> Result<i32> {
    let mode = args.next().ok_or("missing check mode")?;
    let mut root = PathBuf::from(".");
    let mut reports = env::var_os("CACHE_AUDIT_REPORTS").map(PathBuf::from);
    let mut seen = std::collections::HashSet::new();
    while let Some(option) = args.next() {
        if !seen.insert(option.clone()) {
            return Err("duplicate option".into());
        }
        match option.to_str() {
            Some("--workspace") => root = args.next().ok_or("missing workspace")?.into(),
            Some("--reports") => {
                reports = Some(args.next().ok_or("missing reports directory")?.into())
            }
            _ => return Err(format!("unknown check option: {}", option.to_string_lossy()).into()),
        }
    }
    let root = root.canonicalize()?;
    let reports = reports.unwrap_or_else(|| root.join("artifacts/cache-audit"));
    fs::create_dir_all(&reports)?;
    let reports = reports.canonicalize()?;
    let result = if mode == "compat" {
        compatibility(&root, &reports, &cancel).await
    } else {
        negative(&root, &reports, &cancel).await
    };
    fs::write(
        reports.join(format!("{}.status.json", mode.to_string_lossy())),
        serde_json::to_vec_pretty(&json!({
            "status": if cancel.is_cancelled() { "cancelled" } else if result.is_ok() { "passed" } else { "failed" },
            "error": result.as_ref().err().map(ToString::to_string),
        }))?,
    )?;
    if cancel.is_cancelled() {
        return Ok(130);
    }
    result?;
    Ok(0)
}

async fn cache_run(
    root: &Path,
    log: &fs::File,
    hit: bool,
    value: &str,
    cancel: &CancellationToken,
) -> Result<()> {
    let runs = || -> Result<usize> {
        match fs::read_to_string(root.join("trace")) {
            Ok(trace) => Ok(trace.lines().count()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error.into()),
        }
    };
    let before = runs()?;
    checked(
        Command::new("pnpm")
            .args(["pipeline", "proof", "--full"])
            .current_dir(root)
            .env("CACHE_COMPAT_VALUE", value)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log.try_clone()?)),
        cancel,
    )
    .await?;
    if runs()? != before + usize::from(!hit) {
        return Err(format!("expected cache {}", if hit { "hit" } else { "miss" }).into());
    }
    let expected = fs::read_to_string(root.join("source.txt"))?
        + &fs::read_to_string(root.join("dependency.txt"))?;
    if fs::read_to_string(root.join("result"))? != expected {
        return Err("cached output contents differ".into());
    }
    Ok(())
}

async fn compatibility(root: &Path, reports: &Path, cancel: &CancellationToken) -> Result<()> {
    let temp = tempfile::Builder::new()
        .prefix("pnpm-cache-compat-")
        .tempdir()?;
    let fixture = temp.path().join("workspace");
    fs::create_dir(&fixture)?;
    let manifest: Value = serde_json::from_slice(&fs::read(root.join("package.json"))?)?;
    fs::write(
        fixture.join("package.json"),
        serde_json::to_vec(&json!({
            "name":"cache-compat", "private":true, "packageManager": manifest["packageManager"],
            "scripts": {
                "build": "node -e \"const f=require('node:fs');f.appendFileSync('trace','build\\n');f.writeFileSync('result',f.readFileSync('source.txt','utf8')+f.readFileSync('dependency.out','utf8'))\"",
                "dependency": "node -e \"require('node:fs').copyFileSync('dependency.txt','dependency.out')\""
            }
        }))?,
    )?;
    fs::write(
        fixture.join(".gitignore"),
        "trace\nresult\ndependency.out\nignored\nnode_modules/\n",
    )?;
    for (name, contents) in [
        ("source.txt", "source"),
        ("dependency.txt", "dependency"),
        ("unrelated", "before"),
    ] {
        fs::write(fixture.join(name), contents)?;
    }
    let log = fs::File::create(reports.join("compat.log"))?;
    for args in [
        vec!["init", "--quiet"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Cache compatibility",
            "-c",
            "user.email=cache@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "--quiet",
            "-m",
            "test: initialize cache compatibility fixture",
        ],
    ] {
        checked(
            Command::new("git")
                .args(args)
                .current_dir(&fixture)
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log.try_clone()?)),
            cancel,
        )
        .await?;
    }
    for inputs in [vec!["source.*"], vec![], vec!["+source.*"]] {
        fs::write(
            fixture.join("pnpm-workspace.yaml"),
            serde_yaml_ng::to_string(&json!({
                "packages":["."], "cacheDir":temp.path().join("cache"), "stateDir":temp.path().join("state"), "storeDir":temp.path().join("store"),
                "include":{"./tasks.yaml":["tasks","pipelines"]}
            }))?,
        )?;
        let mut definitions = json!({
                "pipelines":{"proof":["build"]},
                "tasks": {
                    "build":{"cache":true,"inputs":inputs,"outputs":["result"],"env":["CACHE_COMPAT_VALUE"],"dependsOn":["dependency"]},
                    "dependency":{"cache":true,"inputs":["dependency.txt"],"outputs":["dependency.out"],"dependsOn":[]}
                }
        });
        fs::write(
            fixture.join("tasks.yaml"),
            serde_yaml_ng::to_string(&definitions)?,
        )?;
        cache_run(&fixture, &log, false, "a", cancel).await?;
        cache_run(&fixture, &log, true, "a", cancel).await?;
        if inputs == ["source.*"] {
            fs::remove_file(fixture.join("result"))?;
            cache_run(&fixture, &log, true, "a", cancel).await?;
            fs::write(fixture.join("ignored"), "ignored")?;
            fs::write(fixture.join("unrelated"), "replacement excludes this")?;
            cache_run(&fixture, &log, true, "a", cancel).await?;
            for (path, content) in [
                ("source.txt", "changed"),
                ("source.new", "untracked input"),
                ("dependency.txt", "changed dependency"),
            ] {
                fs::write(fixture.join(path), content)?;
                cache_run(&fixture, &log, false, "a", cancel).await?;
            }
            cache_run(&fixture, &log, false, "b", cancel).await?;
            cache_run(&fixture, &log, true, "b", cancel).await?;
            definitions["tasks"]["unrelated"] = json!({"cache":false,"dependsOn":[]});
            fs::write(
                fixture.join("tasks.yaml"),
                serde_yaml_ng::to_string(&definitions)?,
            )?;
            cache_run(&fixture, &log, true, "b", cancel).await?;
            definitions["tasks"]["build"]["env"] =
                json!(["CACHE_COMPAT_VALUE", "CACHE_COMPAT_ADDITIONAL"]);
            fs::write(
                fixture.join("tasks.yaml"),
                serde_yaml_ng::to_string(&definitions)?,
            )?;
            cache_run(&fixture, &log, false, "b", cancel).await?;
            cache_run(&fixture, &log, true, "b", cancel).await?;
        } else {
            fs::write(fixture.join("unrelated"), serde_json::to_vec(&inputs)?)?;
            cache_run(&fixture, &log, false, "a", cancel).await?;
            fs::write(fixture.join("ignored"), serde_json::to_vec(&inputs)?)?;
            cache_run(&fixture, &log, true, "a", cancel).await?;
        }
    }
    eprintln!("pnpm cache input/dependency/environment invalidation and output restoration passed");
    Ok(())
}

fn copy_workspace(
    source: &Path,
    destination: &Path,
    root: &Path,
    copy_root: &Path,
    reports: &Path,
    cancel: &CancellationToken,
) -> Result<()> {
    if cancel.is_cancelled() {
        return Err("workspace copy cancelled".into());
    }
    let relative = source.strip_prefix(root)?;
    if [".git", "target", "artifacts", "crates/cache-audit/target"]
        .iter()
        .any(|path| relative == Path::new(path))
        || source == reports
    {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        let link = fs::read_link(source)?;
        let link = match link.strip_prefix(root) {
            Ok(relative) => copy_root.join(relative),
            Err(_) => link,
        };
        std::os::unix::fs::symlink(link, destination)?;
    } else if metadata.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_workspace(
                &entry.path(),
                &destination.join(entry.file_name()),
                root,
                copy_root,
                reports,
                cancel,
            )?;
        }
    } else if metadata.is_file() {
        // Never hardlink mutable workspace files back into the developer checkout.
        fs::copy(source, destination)?;
    } else {
        return Err(format!("unsupported workspace entry: {}", source.display()).into());
    }
    Ok(())
}

async fn negative(root: &Path, reports: &Path, cancel: &CancellationToken) -> Result<()> {
    let document = crate::validator::workspace_document(root)?;
    let owner = match document
        .includes
        .as_ref()
        .map(|includes| includes.owner(&pnpm_workspace::SettingSlot::Setting("tasks".into())))
    {
        Some(pnpm_workspace::SettingOwner::Include(pnpm_workspace::IncludeSource::Local {
            path,
            ..
        })) => path.clone(),
        _ => document.path,
    };
    let owner = owner.strip_prefix(root)?;
    // Mutate the owning source, not a flattened copy that loses delegation.
    let original: Value = serde_yaml_ng::from_slice(&fs::read(root.join(owner))?)?;
    let temporary = tempfile::Builder::new()
        .prefix("cache-audit-negative-")
        .tempdir()?;
    let copy = temporary.path().join("workspace");
    if copy.starts_with(root) {
        return Err("negative-check temporary directory must be outside the workspace".into());
    }
    copy_workspace(root, &copy, root, &copy, reports, cancel)?;
    // pnpm records absolute workspace paths. Refresh relocated installation
    // metadata before observation, without changing the lockfile or running hooks.
    let install_log = fs::File::create(reports.join("negative-install.log"))?;
    checked(
        Command::new("pnpm")
            .args(["install", "--frozen-lockfile", "--ignore-scripts"])
            .current_dir(&copy)
            .stdout(Stdio::from(install_log.try_clone()?))
            .stderr(Stdio::from(install_log)),
        cancel,
    )
    .await?;
    checked(
        Command::new("git").args(["init", "--quiet"]).arg(&copy),
        cancel,
    )
    .await?;
    // Preserve tracked-but-ignored inputs without copying the source .git database.
    let tracked = tempfile::NamedTempFile::new()?;
    checked(
        Command::new("git")
            .args(["ls-files", "-z"])
            .current_dir(root)
            .stdout(Stdio::from(tracked.reopen()?)),
        cancel,
    )
    .await?;
    checked(
        Command::new("git")
            .args(["add", "-f", "--pathspec-file-nul", "--pathspec-from-file"])
            .arg(tracked.path())
            .current_dir(&copy),
        cancel,
    )
    .await?;
    for (name, field, declaration, suffix, diagnostic) in [
        (
            "source",
            "inputs",
            "crates/webfont-generator/**",
            "/crates/webfont-generator/src/lib.rs",
            "read not covered",
        ),
        (
            "output",
            "outputs",
            "packages/webfont-generator/binding.d.ts",
            "/packages/webfont-generator/binding.d.ts",
            "undeclared write",
        ),
    ] {
        let mut config = original.clone();
        let values = config["tasks"]["next:native"][field]
            .as_array_mut()
            .ok_or("missing baseline declarations")?;
        let before = values.len();
        values.retain(|value| value != declaration);
        if before == values.len() {
            return Err(format!("missing baseline declaration: {declaration}").into());
        }
        fs::write(copy.join(owner), serde_yaml_ng::to_string(&config)?)?;
        let directory = reports.join(format!("negative-{name}"));
        let code = runner::run(
            [
                OsString::from("--workspace"),
                copy.clone().into_os_string(),
                "--reports".into(),
                directory.clone().into_os_string(),
                "--task".into(),
                "next:native".into(),
            ]
            .into_iter(),
            cancel.clone(),
        )
        .await?;
        if code != 2 {
            return Err(format!("{name}: expected findings (2), got {code}").into());
        }
        let report: Value = serde_json::from_slice(&fs::read(directory.join("next-native.json"))?)?;
        let found = report["audit"]["findings"]
            .as_array()
            .is_some_and(|findings| {
                findings.iter().any(|finding| {
                    finding[0]
                        .as_str()
                        .is_some_and(|path| path.ends_with(suffix))
                        && finding[1]
                            .as_str()
                            .is_some_and(|reason| reason.contains(diagnostic))
                })
            });
        if report["exit_code"] != 0 || !found {
            return Err(format!("{name}: omission was not detected").into());
        }
        eprintln!("Verified deliberate {name} omission in isolated workspace");
    }
    Ok(())
}
