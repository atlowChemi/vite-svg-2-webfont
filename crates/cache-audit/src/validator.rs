use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};
use wax::{
    Glob, Program,
    walk::{Entry, FileIterator},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Read,
    Write,
    ReadWrite,
    ReadDir,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Access {
    pub kind: Kind,
    pub path: PathBuf,
}

#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Task {
    inputs: Option<Vec<String>>,
    outputs: Option<Vec<String>>,
    depends_on: Option<Vec<String>>,
    #[serde(rename = "cache")]
    _cache: Option<bool>,
    #[serde(rename = "env")]
    _env: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct Workspace {
    tasks: BTreeMap<String, Task>,
    #[serde(default, rename = "packageConfigs")]
    package_configs: serde_yaml_ng::Value,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Policies {
    version: u32,
    tasks: BTreeMap<String, Policy>,
}

pub fn policy_tasks(path: &Path) -> Result<Vec<String>> {
    let policies: Policies = serde_yaml_ng::from_slice(&fs::read(path)?)?;
    if policies.version != 1 {
        return Err("unsupported audit policy version".into());
    }
    Ok(policies.tasks.into_keys().collect())
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Policy {
    #[serde(default)]
    ignore: Vec<Rule>,
    #[serde(default)]
    never_cache: Vec<Rule>,
    #[serde(default)]
    missing_inputs: Vec<Rule>,
    #[serde(default)]
    cleanup_outputs: Vec<Rule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    pattern: String,
    reason: String,
    kinds: Option<Vec<Kind>>,
    contents: Option<String>,
    environment: Option<String>,
    #[serde(default)]
    directory: bool,
}

struct CompiledRule {
    glob: Glob<'static>,
    absolute: bool,
    kinds: Option<Vec<Kind>>,
    directory: bool,
}

fn rules(rules: Vec<Rule>) -> Result<Vec<CompiledRule>> {
    rules
        .into_iter()
        .map(|r| {
            if r.reason.trim().is_empty() {
                return Err("policy rules require a reason".into());
            }
            Ok(CompiledRule {
                glob: Glob::new(&r.pattern)
                    .map_err(|error| format!("invalid policy pattern {:?}: {error}", r.pattern))?
                    .into_owned(),
                absolute: Path::new(&r.pattern).is_absolute(),
                kinds: r.kinds,
                directory: r.directory,
            })
        })
        .collect()
}

fn policy_matches(rule: &CompiledRule, path: &Path) -> bool {
    // wax matching alone does not distinguish rooted and relative candidates.
    // An external-system rule must never exempt workspace-relative source reads.
    rule.absolute == path.is_absolute() && rule.glob.is_match(path)
}

fn globs(patterns: &[String]) -> Result<Vec<Glob<'static>>> {
    patterns
        .iter()
        .map(|p| Ok(Glob::new(p)?.into_owned()))
        .collect()
}
fn matches(globs: &[Glob<'_>], path: &Path) -> bool {
    globs.iter().any(|g| g.is_match(path))
}

fn git(project: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git input enumeration failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output.stdout)
}

fn no_symlink_parents(root: &Path, relative: &Path) -> Result<()> {
    let mut path = root.to_path_buf();
    for part in relative.components() {
        if !matches!(part, Component::Normal(_)) {
            return Err("non-local cache path".into());
        }
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(format!("symlink in cache path: {}", path.display()).into());
        }
    }
    Ok(())
}

fn files(project: &Path) -> Result<BTreeSet<PathBuf>> {
    if !git(project, &["rev-parse", "--show-superproject-working-tree"])?
        .iter()
        .all(u8::is_ascii_whitespace)
        || git(project, &["ls-files", "--stage", "-z"])?
            .split(|b| *b == 0)
            .any(|p| p.starts_with(b"160000 "))
    {
        return Err("pnpm bypasses caching for submodules; coverage is unavailable".into());
    }
    let mut files = BTreeSet::new();
    for bytes in git(
        project,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?
    .split(|b| *b == 0)
    .filter(|p| !p.is_empty())
    {
        let path = PathBuf::from(std::str::from_utf8(bytes)?);
        if path.starts_with("node_modules") {
            continue;
        }
        match fs::symlink_metadata(project.join(&path)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
            Ok(_) => (),
        }
        no_symlink_parents(project, path.parent().unwrap_or(Path::new("")))?;
        files.insert(path);
    }
    Ok(files)
}

// Matches pnpm 12.13.0 cache/inputs.rs, including inputs: [] meaning defaults.
fn selected(task: &Task, files: &BTreeSet<PathBuf>) -> Result<BTreeSet<PathBuf>> {
    let patterns = task.inputs.as_deref().unwrap_or_default();
    let replace = globs(
        &patterns
            .iter()
            .filter(|p| !p.starts_with('+'))
            .cloned()
            .collect::<Vec<_>>(),
    )?;
    let add = globs(
        &patterns
            .iter()
            .filter_map(|p| p.strip_prefix('+').map(str::to_owned))
            .collect::<Vec<_>>(),
    )?;
    let output = globs(task.outputs.as_deref().unwrap_or_default())?;
    Ok(files
        .iter()
        .filter(|p| {
            !matches(&output, p) && (replace.is_empty() || matches(&replace, p) || matches(&add, p))
        })
        .cloned()
        .collect())
}

fn outputs(project: &Path, patterns: &[Glob<'_>], snapshot: bool) -> Result<BTreeSet<PathBuf>> {
    let mut result = BTreeSet::new();
    for glob in patterns {
        for entry in glob
            .walk(project)
            .not(wax::any(["**/node_modules/**", "**/.git/**"]))?
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    let error = std::io::Error::from(error);
                    if snapshot && error.kind() == std::io::ErrorKind::NotFound {
                        continue;
                    }
                    return Err(error.into());
                }
            };
            if entry.file_type().is_symlink() {
                return Err("symlink output is unsupported".into());
            }
            if !snapshot && !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(project)?;
            no_symlink_parents(project, relative)?;
            result.insert(entry.path().to_path_buf());
        }
    }
    Ok(result)
}

fn visit<'a>(
    name: &'a str,
    tasks: &'a BTreeMap<String, Task>,
    scripts: &serde_json::Value,
    visiting: &mut BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    result: &mut Vec<&'a str>,
) -> Result<()> {
    if seen.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.into()) {
        return Err("cyclic task dependencies".into());
    }
    let task = tasks
        .get(name)
        .ok_or("task has no explicit configuration (implicit upstream ordering unsupported)")?;
    if scripts
        .get(name)
        .and_then(serde_json::Value::as_str)
        .is_none()
    {
        return Err(format!("missing script: {name}").into());
    }
    for dependency in task.depends_on.as_deref().unwrap_or_default() {
        if dependency.starts_with('^') {
            return Err(
                "upstream (^task) dependencies are not supported yet; refusing partial coverage"
                    .into(),
            );
        }
        visit(dependency, tasks, scripts, visiting, seen, result)?;
    }
    visiting.remove(name);
    seen.insert(name.into());
    result.push(name);
    Ok(())
}

pub struct Audit {
    root: PathBuf,
    project: PathBuf,
    task: String,
    inputs: BTreeSet<PathBuf>,
    readable_inputs: BTreeSet<PathBuf>,
    dependency_outputs: BTreeSet<PathBuf>,
    dependency_output_globs: Vec<Glob<'static>>,
    before_outputs: BTreeSet<PathBuf>,
    output_globs: Vec<Glob<'static>>,
    ignore: Vec<CompiledRule>,
    never: Vec<CompiledRule>,
    input_tasks: Vec<Task>,
    before_paths: BTreeSet<PathBuf>,
    before_symlinks: BTreeSet<PathBuf>,
    content_assertions: Vec<(PathBuf, String)>,
    environment_assertions: Vec<(PathBuf, String)>,
    missing_inputs: Vec<CompiledRule>,
    cleanup_outputs: Vec<CompiledRule>,
}

const SNAPSHOT_EXCLUSIONS: [&str; 4] = [
    "**/node_modules/**",
    "**/.git/**",
    "**/target/**",
    ".pnpm/**",
];

fn snapshot(project: &Path) -> Result<(BTreeSet<PathBuf>, BTreeSet<PathBuf>)> {
    let mut paths = BTreeSet::from([project.to_path_buf()]);
    let mut symlinks = BTreeSet::new();
    for entry in Glob::new("**")?
        .walk(project)
        .not(wax::any(SNAPSHOT_EXCLUSIONS))?
    {
        let entry = entry?;
        paths.insert(entry.path().to_path_buf());
        if entry.file_type().is_symlink() {
            symlinks.insert(entry.path().to_path_buf());
        }
    }
    Ok((paths, symlinks))
}

// Resolve an existing prefix, retaining the missing suffix. Used only for policy
// exemptions, never for input coverage or claims that a syscall succeeded.
fn policy_target(path: &Path) -> std::io::Result<PathBuf> {
    match fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or(error)?;
            let mut resolved = policy_target(parent)?;
            match path.components().next_back() {
                Some(Component::ParentDir) => {
                    resolved.pop();
                }
                Some(Component::Normal(name)) => resolved.push(name),
                _ => (),
            }
            Ok(resolved)
        }
        Err(error) => Err(error),
    }
}

#[derive(Serialize)]
pub struct Report {
    pub task: String,
    pub input_count: usize,
    pub output_count: usize,
    pub findings: BTreeSet<(PathBuf, String)>,
}

fn normalize_access(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err("relative observation".into());
    }
    if !path.components().any(|c| c == Component::ParentDir) {
        return Ok(path.to_path_buf());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                // Lexically collapsing '..' across a symlink or a missing directory
                // could turn an uncovered access into a different, covered input.
                if !fs::symlink_metadata(&normalized)?.is_dir() {
                    return Err("parent traversal is not a directory".into());
                }
                normalized.pop();
            }
            Component::CurDir => (),
            other => {
                normalized.push(other);
                if fs::symlink_metadata(&normalized).is_ok_and(|m| m.file_type().is_symlink()) {
                    return Err("parent traversal through symlink".into());
                }
            }
        }
    }
    Ok(normalized)
}

impl Audit {
    pub fn prepare(root: &Path, project: &Path, name: &str, policy: Option<&Path>) -> Result<Self> {
        let root = fs::canonicalize(root)?;
        let project = fs::canonicalize(root.join(project))?;
        let relative_project = project.strip_prefix(&root)?;
        let root_manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("package.json"))?)?;
        let version = root_manifest["packageManager"]
            .as_str()
            .unwrap_or("")
            .split('+')
            .next();
        let supported = version
            .and_then(|version| version.strip_prefix("pnpm@12."))
            .and_then(|version| version.split_once('.'))
            .is_some_and(|(minor, patch)| {
                minor.parse::<u32>().is_ok_and(|minor| minor >= 13) && patch.parse::<u32>().is_ok()
            });
        if !supported {
            return Err("validator requires stable pnpm 12.13 or newer within major 12; validate cache semantics before adopting another major".into());
        }
        let text = fs::read_to_string(root.join("pnpm-workspace.yaml"))?;
        if text.contains("${") {
            return Err("environment-expanded YAML is unsupported".into());
        }
        let workspace: Workspace = serde_yaml_ng::from_str(&text)?;
        if !workspace.package_configs.is_null() {
            return Err("packageConfigs overrides are unsupported".into());
        }
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(project.join("package.json"))?)?;
        let mut names = Vec::new();
        visit(
            name,
            &workspace.tasks,
            &manifest["scripts"],
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut names,
        )?;
        let own = &workspace.tasks[name];
        if own.outputs.is_none() {
            return Err("audited task must declare outputs (use [] for no artifacts)".into());
        }
        let all = files(&project)?;
        let mut inputs = BTreeSet::new();
        let mut dependency_outputs = BTreeSet::new();
        let mut dependency_output_globs = Vec::new();
        let mut input_tasks = Vec::new();
        for task_name in names {
            let task = &workspace.tasks[task_name];
            input_tasks.push(task.clone());
            inputs.extend(selected(task, &all)?.into_iter().map(|p| project.join(p)));
            let patterns = task.outputs.as_deref().unwrap_or_default();
            for pattern in patterns {
                if Path::new(pattern).is_absolute()
                    || Path::new(pattern)
                        .components()
                        .any(|p| matches!(p, Component::ParentDir))
                {
                    return Err("output glob escapes the project".into());
                }
            }
            if task_name != name {
                let patterns = globs(patterns)?;
                dependency_outputs.extend(outputs(&project, &patterns, false)?);
                dependency_output_globs.extend(patterns);
            }
        }
        // pnpm hashes the workspace lockfile independently of the input globs.
        if root.join("pnpm-lock.yaml").is_file() {
            inputs.insert(root.join("pnpm-lock.yaml"));
        }
        // pnpm hashes a symlink's target text, not the referenced file contents.
        let readable_inputs = inputs
            .iter()
            .filter(|p| !fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()))
            .cloned()
            .collect();
        let key = format!(
            "{}#{name}",
            if relative_project.as_os_str().is_empty() {
                ".".into()
            } else {
                relative_project.to_string_lossy().into_owned()
            }
        );
        let policy = match policy {
            Some(path) => {
                let mut policies: Policies = serde_yaml_ng::from_slice(&fs::read(path)?)?;
                if policies.version != 1 {
                    return Err("unsupported audit policy version".into());
                }
                policies
                    .tasks
                    .remove(&key)
                    .ok_or("policy has no entry for this project/task")?
            }
            None => Policy::default(),
        };
        if policy
            .never_cache
            .iter()
            .any(|r| r.kinds.is_some() || r.directory)
        {
            return Err(
                "neverCache rules cannot be restricted to access kinds or directories".into(),
            );
        }
        if policy
            .missing_inputs
            .iter()
            .chain(&policy.cleanup_outputs)
            .any(|r| {
                r.kinds.is_some() || r.contents.is_some() || r.environment.is_some() || r.directory
            })
        {
            return Err("missingInputs and cleanupOutputs accept only pattern and reason".into());
        }
        let mut content_assertions = Vec::new();
        let mut environment_assertions = Vec::new();
        for rule in policy.ignore.iter().chain(&policy.never_cache) {
            if rule.contents.is_some() || rule.environment.is_some() {
                if rule
                    .pattern
                    .contains(['*', '?', '[', ']', '{', '}', '<', '>', '\\'])
                    || Path::new(&rule.pattern)
                        .components()
                        .any(|c| c == Component::ParentDir)
                {
                    return Err(
                        "file assertions require a literal path without parent traversal".into(),
                    );
                }
                if let Some(contents) = &rule.contents {
                    content_assertions.push((root.join(&rule.pattern), contents.clone()));
                }
                if let Some(variable) = &rule.environment {
                    if rule.kinds.as_deref() != Some(&[Kind::Read])
                        || rule.directory
                        || !input_tasks.iter().any(|task| {
                            task._env.as_ref().is_some_and(|env| env.contains(variable))
                        })
                    {
                        return Err("environment file assertions require read-only rules and an explicitly keyed task environment variable".into());
                    }
                    environment_assertions.push((root.join(&rule.pattern), variable.clone()));
                }
            }
        }
        check_contents(&content_assertions)?;
        check_environment(&environment_assertions)?;
        let output_globs = globs(own.outputs.as_deref().unwrap_or_default())?;
        let (before_paths, before_symlinks) = snapshot(&project)?;
        Ok(Self {
            cleanup_outputs: rules(policy.cleanup_outputs)?,
            missing_inputs: rules(policy.missing_inputs)?,
            content_assertions,
            environment_assertions,
            input_tasks,
            before_paths,
            before_symlinks,
            before_outputs: outputs(&project, &output_globs, true)?,
            root,
            project,
            task: key,
            inputs,
            readable_inputs,
            dependency_outputs,
            dependency_output_globs,
            output_globs,
            ignore: rules(policy.ignore)?,
            never: rules(policy.never_cache)?,
        })
    }

    fn policy_path<'a>(&self, path: &'a Path) -> &'a Path {
        match path.strip_prefix(&self.root) {
            Ok(p) if p.as_os_str().is_empty() => Path::new("."),
            Ok(p) => p,
            Err(_) => path,
        }
    }

    fn ignored(&self, path: &Path, access: Kind, kind: Kind) -> bool {
        self.ignore.iter().chain(&self.never).any(|r| {
            policy_matches(r, self.policy_path(path))
                && (!r.directory || fs::metadata(path).is_ok_and(|m| m.is_dir()))
                && r.kinds
                    .as_ref()
                    .is_none_or(|k| k.contains(&kind) || k.contains(&access))
        })
    }

    pub fn finish(&self, accesses: &[Access]) -> Result<Report> {
        check_contents(&self.content_assertions)?;
        check_environment(&self.environment_assertions)?;
        let after = outputs(&self.project, &self.output_globs, false)?;
        let mut findings = BTreeSet::new();
        let normalized: Vec<_> = accesses
            .iter()
            .filter_map(|access| match normalize_access(&access.path) {
                Ok(path) => Some(Access {
                    kind: access.kind,
                    path,
                }),
                Err(error) => {
                    // Resolve only for explicit exemptions, never to claim input
                    // coverage: hashing a symlink is not hashing its target.
                    let exempt = |path: &Path| match access.kind {
                        Kind::ReadWrite => {
                            self.ignored(path, access.kind, Kind::Read)
                                && self.ignored(path, access.kind, Kind::Write)
                        }
                        kind => self.ignored(path, kind, kind),
                    };
                    if exempt(&access.path)
                        && policy_target(&access.path).is_ok_and(|path| exempt(&path))
                    {
                        return None;
                    }
                    findings.insert((
                        access.path.clone(),
                        format!("unresolved observed path: {error}"),
                    ));
                    None
                }
            })
            .collect();
        for path in self
            .inputs
            .iter()
            .chain(&after)
            .chain(&self.dependency_outputs)
        {
            if self
                .never
                .iter()
                .any(|r| policy_matches(r, self.policy_path(path)))
            {
                findings.insert((
                    path.clone(),
                    "must-not-cache path is selected as input/output".into(),
                ));
            }
        }
        let writes: BTreeSet<_> = normalized
            .iter()
            .filter(|a| matches!(a.kind, Kind::Write | Kind::ReadWrite))
            .map(|a| &a.path)
            .collect();
        for access in &normalized {
            let path = &access.path;
            let ignored = |kind| self.ignored(path, access.kind, kind);
            if matches!(access.kind, Kind::Write | Kind::ReadWrite)
                && !ignored(Kind::Write)
                && !after.contains(path)
                && !(self.before_outputs.contains(path)
                    && !path.try_exists()?
                    && self
                        .cleanup_outputs
                        .iter()
                        .any(|r| policy_matches(r, self.policy_path(path))))
            {
                findings.insert((
                    path.clone(),
                    "undeclared write/deletion/intermediate (requires output or policy)".into(),
                ));
            }
            if matches!(access.kind, Kind::Read | Kind::ReadWrite) && !ignored(Kind::Read) {
                let generated = after.contains(path)
                    && !self.before_outputs.contains(path)
                    && writes.contains(path);
                if !self.readable_inputs.contains(path)
                    && !self.dependency_outputs.contains(path)
                    && !generated
                    && !self.covered_absence(path, &writes)?
                {
                    findings.insert((
                        path.clone(),
                        "read not covered by effective inputs or dependency outputs".into(),
                    ));
                }
            }
            if access.kind == Kind::ReadDir && !ignored(Kind::ReadDir) {
                findings.insert((
                    path.clone(),
                    "directory enumeration requires an explicit policy".into(),
                ));
            }
        }
        Ok(Report {
            task: self.task.clone(),
            input_count: self.inputs.len(),
            output_count: after.len(),
            findings,
        })
    }

    fn covered_absence(&self, path: &Path, writes: &BTreeSet<&PathBuf>) -> Result<bool> {
        // fspy does not distinguish open/stat. Only reviewed regular-file probes
        // may use this rule; arbitrary existence/empty-directory checks may not.
        if !self
            .missing_inputs
            .iter()
            .any(|r| policy_matches(r, self.policy_path(path)))
        {
            return Ok(false);
        }
        let Ok(relative) = path.strip_prefix(&self.project) else {
            return Ok(false);
        };
        if self.before_paths.contains(path)
            || path.ancestors().any(|p| self.before_symlinks.contains(p))
            || wax::any(SNAPSHOT_EXCLUSIONS)?.is_match(relative)
            || !fs::symlink_metadata(path).is_err_and(|e| {
                matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                )
            })
            || writes.iter().any(|p| path.starts_with(p))
        {
            return Ok(false);
        }
        // Node also probes foo.js/package.json. Replacing the keyed file ancestor
        // with a directory changes that input (or its separately audited producer).
        if let Some(file) = path
            .ancestors()
            .skip(1)
            .find(|p| fs::metadata(p).is_ok_and(|m| m.is_file()))
        {
            return Ok(self.before_paths.contains(file)
                && (self.readable_inputs.contains(file)
                    || self.dependency_outputs.contains(file)));
        }
        if !policy_target(path).is_ok_and(|p| p == path) {
            return Ok(false);
        }
        // Dependency output namespaces are owned by separately audited producers,
        // including the absence of optional manifests alongside their artifacts.
        if matches(&self.dependency_output_globs, relative) {
            return Ok(true);
        }
        // A future file must enter pnpm's actual key, not merely match a glob.
        let candidate = BTreeSet::from([relative.to_path_buf()]);
        let mut selected_by_task = false;
        for task in &self.input_tasks {
            selected_by_task |= !selected(task, &candidate)?.is_empty();
        }
        if !selected_by_task {
            return Ok(false);
        }
        let result = Command::new("git")
            .args(["check-ignore", "--no-index", "-q", "--"])
            .arg(relative)
            .current_dir(&self.project)
            .status()?;
        match result.code() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err("git absence coverage check failed".into()),
        }
    }
}

fn check_contents(assertions: &[(PathBuf, String)]) -> Result<()> {
    for (path, expected) in assertions {
        if fs::read_to_string(path)? != *expected {
            return Err(format!("policy content assertion failed: {}", path.display()).into());
        }
    }
    Ok(())
}

fn check_environment(assertions: &[(PathBuf, String)]) -> Result<()> {
    for (path, variable) in assertions {
        // Node is already required by this workspace. Its stdlib hashes bytes
        // identically to the pre-pipeline launcher, without a new Rust dependency.
        let output = Command::new("node").args(["-e", "try { process.stdout.write(require('node:crypto').createHash('sha256').update(require('node:fs').readFileSync(process.argv[1])).digest('hex')); } catch (e) { if (e.code === 'ENOENT') process.stdout.write('missing'); else throw e; }"]).arg(path).output()?;
        if !output.status.success()
            || std::env::var(variable).ok().as_deref() != Some(std::str::from_utf8(&output.stdout)?)
        {
            return Err(format!(
                "environment file fingerprint mismatch: {} via {variable}",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_policy_patterns_compile() {
        for source in [
            include_str!("../policy.native.yaml"),
            include_str!("../policy.build.yaml"),
        ] {
            let policies: Policies = serde_yaml_ng::from_str(source).unwrap();
            for policy in policies.tasks.into_values() {
                rules(policy.ignore).unwrap();
                rules(policy.never_cache).unwrap();
                rules(policy.missing_inputs).unwrap();
                rules(policy.cleanup_outputs).unwrap();
            }
        }
    }
}
