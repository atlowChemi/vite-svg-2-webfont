use std::{
    error::Error,
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    process::Command,
};

use indexmap::IndexMap;
use pnpm_config::{Config, Host, LinkWorkspacePackages};
use pnpm_workspace::{FindWorkspaceProjectsOpts, GraphPkg, find_workspace_projects};
use pnpm_workspace_projects_graph::{
    CreateProjectsGraphOptions, WorkspaceCatalogs, create_projects_graph,
};
use pnpm_workspace_task_scheduler::{
    BuildPipelineTaskGraphOptions, SequenceTasksOptions, TaskGraph, build_pipeline_task_graph,
    sequence_tasks, task_graph_to_json,
};
use serde_json::Value;

/// Resolve full pipeline graphs with the pinned pnpm implementation, without
/// installing dependencies or running tasks. Selection is deliberately explicit.
pub fn plans(
    root: &Path,
    pipeline: Option<&str>,
    task: Option<&str>,
) -> Result<IndexMap<String, TaskGraph>, Box<dyn Error>> {
    let root = root.canonicalize()?;
    let config = Config {
        skip_store_dir_resolution: true,
        ..Config::default()
    }
    .current::<Host>(&root)?;
    // Config::catalogs contains hook overrides, not the YAML catalog tables.
    let catalogs = match &config.catalogs {
        Some(catalogs) => catalogs.clone(),
        None => pnpm_catalogs_config::get_catalogs_from_workspace_manifest(
            pnpm_workspace::read_workspace_manifest(&root)?.as_ref(),
        )?,
    };
    let projects = find_workspace_projects(
        &root,
        &FindWorkspaceProjectsOpts {
            patterns: config.workspace_package_patterns.clone(),
            ..Default::default()
        },
    )?;
    let graph = create_projects_graph(
        projects
            .iter()
            .map(|project| GraphPkg { project })
            .collect(),
        &CreateProjectsGraphOptions {
            link_workspace_packages: Some(
                config.link_workspace_packages != LinkWorkspacePackages::Off,
            ),
            catalogs: Some(WorkspaceCatalogs {
                catalogs: &catalogs,
                workspace_dir: &root,
            }),
            ..Default::default()
        },
    );
    // pnpm's CLI keeps_workspace_root helper is private. The parity check guards
    // this full-selection rule as well as the public graph implementation.
    let keep_root = config.include_workspace_root
        || config
            .workspace_package_patterns
            .as_ref()
            .is_none_or(|patterns| patterns.len() == 1 && patterns[0] == ".");
    let dependencies: IndexMap<_, _> = graph
        .graph
        .iter()
        .filter(|(path, _)| keep_root || **path != root)
        .map(|(path, node)| {
            (
                path.clone(),
                node.dependencies
                    .iter()
                    .filter(|path| keep_root || **path != root)
                    .cloned()
                    .collect(),
            )
        })
        .collect();
    let scripts = |path: &Path, name: &str| {
        projects
            .iter()
            .find(|project| project.root_dir == path)
            .and_then(|project| project.manifest.value().get("scripts"))
            .and_then(|scripts| scripts.get(name))
            .filter(|script| script.is_string())
            .map(|_| vec![name.to_owned()])
            .unwrap_or_default()
    };
    let selected = if let Some(task) = task {
        if !projects
            .iter()
            .any(|project| !scripts(&project.root_dir, task).is_empty())
        {
            return Err(format!("unknown task: {task}").into());
        }
        IndexMap::from([(task.to_owned(), vec![task.to_owned()])])
    } else if let Some(name) = pipeline {
        IndexMap::from([(
            name.to_owned(),
            config
                .pipelines
                .get(name)
                .ok_or_else(|| format!("unknown pipeline: {name}"))?
                .clone(),
        )])
    } else {
        config.pipelines.clone()
    };
    let mut result = IndexMap::new();
    for (name, seeds) in selected {
        let names: Vec<_> = seeds.iter().map(String::as_str).collect();
        let mut graph = build_pipeline_task_graph(&BuildPipelineTaskGraphOptions {
            project_dependencies: &dependencies,
            select_scripts: &scripts,
            task_names: &names,
            requested_projects: None,
            tasks: Some(&config.tasks),
        });
        sequence_tasks(
            &mut graph,
            &SequenceTasksOptions {
                workspace_dir: &root,
                ignore_cycles: config.ignore_workspace_cycles,
                emit: |_| {},
            },
        )?;
        result.insert(name, graph);
    }
    Ok(result)
}

pub fn run(mut args: impl Iterator<Item = OsString>) -> Result<i32, Box<dyn Error>> {
    let mut root = PathBuf::from(".");
    let mut pipeline = None;
    let mut task = None;
    let mut all = false;
    let mut verify = false;
    let mut seen = std::collections::HashSet::new();
    while let Some(option) = args.next() {
        if !seen.insert(option.clone()) {
            return Err("duplicate option".into());
        }
        match option.to_str() {
            Some("--workspace") => root = args.next().ok_or("missing workspace")?.into(),
            Some("--pipeline" | "--task") => {
                let value = args
                    .next()
                    .ok_or("missing selection")?
                    .into_string()
                    .map_err(|_| "non-UTF-8 selection")?;
                if option == "--pipeline" {
                    pipeline = Some(value);
                } else {
                    task = Some(value);
                }
            }
            Some("--all") => all = true,
            Some("--verify-pnpm") => verify = true,
            _ => return Err(format!("unknown graph option: {}", option.to_string_lossy()).into()),
        }
    }
    if usize::from(all) + usize::from(pipeline.is_some()) + usize::from(task.is_some()) != 1 {
        return Err("graph requires exactly one of --pipeline NAME, --task NAME, or --all".into());
    }
    if verify && task.is_some() {
        return Err("--verify-pnpm requires --pipeline or --all".into());
    }
    let root = root.canonicalize()?;
    let plans = plans(&root, pipeline.as_deref(), task.as_deref())?;
    let graphs = Value::Object(
        plans
            .iter()
            .map(|(name, graph)| {
                Ok((
                    name.clone(),
                    serde_json::to_value(task_graph_to_json(graph, &root))?,
                ))
            })
            .collect::<Result<_, serde_json::Error>>()?,
    );
    if verify {
        for (name, graph) in graphs.as_object().ok_or("invalid graph document")? {
            let output = Command::new("pnpm")
                .current_dir(&root)
                .args(["pipeline", name, "--full", "--dry-run", "--json"])
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "pnpm graph {name} failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            let expected: Value = serde_json::from_slice(&output.stdout)?;
            if graph != &expected {
                return Err(format!(
                    "pnpm graph drift for {name}\nRust: {graph}\npnpm: {expected}"
                )
                .into());
            }
        }
    }
    serde_json::to_writer(io::stdout().lock(), &graphs)?;
    Ok(0)
}
