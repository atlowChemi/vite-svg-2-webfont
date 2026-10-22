use std::{env, error::Error, io, os::unix::process::ExitStatusExt, process::Stdio};

use fspy::PathAccess;
use serde_json::json;
mod validator;
use validator::{Access, Audit, Kind};

#[tokio::main]
async fn main() {
    match observe().await {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("cache-audit: {error}");
            std::process::exit(1);
        }
    }
}

async fn observe() -> Result<i32, Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let mut options = std::collections::BTreeMap::new();
    loop {
        let option = args.next().ok_or("usage: cache-audit [--workspace PATH --project PATH --task NAME --policy FILE] -- command ...")?;
        if option == "--" {
            break;
        }
        let option = option.into_string().map_err(|_| "invalid option")?;
        if !["--workspace", "--project", "--task", "--policy"].contains(&option.as_str()) {
            return Err(format!("unknown option: {option}").into());
        }
        let value = args.next().ok_or("missing option value")?;
        if options.insert(option, value).is_some() {
            return Err("duplicate option".into());
        }
    }
    let audit = if let Some(task) = options.get("--task") {
        Some(Audit::prepare(
            options
                .get("--workspace")
                .map(std::path::Path::new)
                .unwrap_or(std::path::Path::new(".")),
            options
                .get("--project")
                .map(std::path::Path::new)
                .unwrap_or(std::path::Path::new(".")),
            task.to_str().ok_or("non-UTF-8 task name")?,
            options.get("--policy").map(std::path::Path::new),
        )?)
    } else {
        if !options.is_empty() {
            return Err("audit options require --task".into());
        }
        None
    };
    let program = args.next().ok_or("missing command after --")?;
    let mut command = fspy::Command::new(program);
    command
        .args(args)
        .envs(env::vars_os())
        .current_dir(env::current_dir()?)
        .stdin(Stdio::inherit())
        // Keep stdout exclusively for the access manifest, without buffering child logs.
        .stdout(Stdio::from(io::stderr()))
        .stderr(Stdio::inherit());
    let child = command
        .spawn(tokio_util::sync::CancellationToken::new())
        .await?;
    let termination = child.wait_handle.await?;
    // Never emit a success-shaped manifest when observation was incomplete.
    let accesses = termination.path_accesses?;
    let mut records = Vec::new();
    for access in accesses.iter() {
        let (kind, path) = match access {
            PathAccess::Read(path) => (Kind::Read, path),
            PathAccess::Write(path) => (Kind::Write, path),
            PathAccess::ReadWrite(path) => (Kind::ReadWrite, path),
            PathAccess::ReadDir(path) => (Kind::ReadDir, path),
        };
        let path = path.strip_path_prefix("/", |relative| {
            let relative = relative?;
            // Reject unrepresentable paths instead of silently merging lossy names.
            let text = relative.to_str().ok_or("non-UTF-8 access path")?;
            Ok::<_, Box<dyn Error>>(format!("/{text}"))
        })?;
        records.push(Access {
            kind,
            path: path.into(),
        });
    }
    let code = termination
        .status
        .code()
        .unwrap_or_else(|| 128 + termination.status.signal().unwrap_or(1));
    // Failed commands can leave outputs incomplete; preserve their status rather
    // than replacing it with an output-enumeration error.
    let report = if code == 0 {
        audit.map(|a| a.finish(&records)).transpose()?
    } else {
        None
    };
    let failed = report.as_ref().is_some_and(|r| !r.findings.is_empty());
    serde_json::to_writer(
        io::stdout().lock(),
        &json!({ "version": 1, "exit_code": code, "accesses": records, "audit": report }),
    )?;
    Ok(if code == 0 && failed { 2 } else { code })
}
