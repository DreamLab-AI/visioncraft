//! Native filesystem and command-line adapter for the deterministic core.
//!
//! Evaluation only reads source and evidence plus fixed Git metadata commands.
//! Commands in imported receipts are never replayed. All source coverage and test
//! observations remain unknown unless supplied by an actual observer/adapter.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::process::{self, Termination};
use crate::{
    Artifact, AuditReceipt, CheckSpec, Evaluator, ExecutionReceipt, GateOutcome, GatePolicy,
    Invocation, ManualObservation, ProcessStatus, QualityPlan, Subject, VerifiedArtifacts,
};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

const MAX_JSON: u64 = 64 * 1024 * 1024;
const MAX_FILE: u64 = 1024 * 1024 * 1024;

/// Explicit quality operations; only `run` and `deepsec` execute supplied checks.
#[derive(Debug, Parser)]
#[command(
    name = "visioncraft",
    version,
    about = "Explicit quality checks and evidence-bound gates"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Operation,
}

#[derive(Debug, Subcommand)]
enum Operation {
    /// Capture or validate a quality plan without running its check commands.
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    /// Execute selected exit-code checks and retain private receipts.
    Run(RunOptions),
    /// Evaluate existing receipts; never execute their recorded commands.
    Evaluate(EvaluateOptions),
    /// Evaluate the whole required-check gate.
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
    /// Check expectation coverage and all other required checks together.
    Expectations {
        #[command(subcommand)]
        command: ExpectationCommand,
    },
    /// Run the versioned Deepsec security adapter.
    Deepsec(crate::deepsec::DeepsecOptions),
}

#[derive(Debug, Subcommand)]
enum PlanCommand {
    /// Capture selected source bytes into a plan for one explicit argv check.
    Init(InitOptions),
    /// Validate a plan schema and policy; no recorded commands are run.
    Check {
        plan: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum GateCommand {
    Evaluate(EvaluateOptions),
}

#[derive(Debug, Subcommand)]
enum ExpectationCommand {
    Check(EvaluateOptions),
}

#[derive(Debug, Args)]
struct InitOptions {
    /// Explicit Git worktree root; defaults to the current directory.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// New plan file; existing outputs are refused.
    #[arg(long)]
    out: PathBuf,
    /// Selected source paths relative to root; repeat for locks/configuration.
    #[arg(long, required = true)]
    input: Vec<String>,
    #[arg(long, default_value = "quality")]
    id: String,
    #[arg(long, default_value = "check")]
    check: String,
    #[arg(long)]
    repository: Option<String>,
    /// Features, target and fixture relevant to this check.
    #[arg(long)]
    configuration: String,
    #[arg(long, default_value = ".")]
    cwd: String,
    /// Inherited environment names only; never values.
    #[arg(long, default_value = "PATH")]
    env: Vec<String>,
    #[arg(long, default_value_t = 60_000)]
    timeout_ms: u64,
    #[arg(long, default_value_t = 8 * 1024 * 1024)]
    max_output_bytes: u64,
    /// Literal program and arguments after `--`; no shell is added.
    #[arg(last = true, required = true, num_args = 1..)]
    command: Vec<String>,
}

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("selection").required(true).multiple(false).args(["check", "all"])))]
struct RunOptions {
    plan: PathBuf,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Select a single declared check.
    #[arg(long)]
    check: Option<String>,
    /// Run every declared check serially.
    #[arg(long)]
    all: bool,
    /// Fresh private run directory; never reuse an earlier attempt.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct EvaluateOptions {
    plan: PathBuf,
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Run root containing check-*/receipt.json and their relative artifacts.
    #[arg(long, required = true)]
    receipts: Vec<PathBuf>,
    /// Optional strict JSON audit/manual/family context; artifact paths are
    /// relative to this file's parent, which must be a private evidence root.
    #[arg(long)]
    context: Option<PathBuf>,
    /// Optional current-subject JSON, verified against the actual selected files.
    #[arg(long)]
    current_subject: Option<PathBuf>,
    /// Inject evaluation time in Unix milliseconds; defaults to the current clock.
    #[arg(long)]
    as_of_ms: Option<u64>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextFile {
    #[serde(default)]
    audits: Vec<AuditReceipt>,
    #[serde(default)]
    manual_observations: Vec<ManualObservation>,
    #[serde(default)]
    families: Vec<FamilyDeclaration>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilyDeclaration {
    route: String,
    family: String,
}

/// CLI failure with a stable process exit code and non-source diagnostic.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct CliError {
    /// Usage/schema (2), execution (70), unavailable (78), timeout (124), etc.
    pub code: i32,
    /// Diagnostic without raw source/configuration contents.
    pub message: String,
}

fn error(code: i32, message: impl Into<String>) -> CliError {
    CliError {
        code,
        message: message.into(),
    }
}
fn invalid(e: impl std::fmt::Display) -> CliError {
    error(2, e.to_string())
}
fn io_error(e: std::io::Error) -> CliError {
    error(
        70,
        format!("filesystem/process operation failed: {}", e.kind()),
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn root(path: &Path) -> Result<PathBuf, CliError> {
    let path = path.canonicalize().map_err(io_error)?;
    if !path.is_dir() {
        return Err(error(2, "root must be a directory"));
    }
    Ok(path)
}

fn confined(root: &Path, relative: &str, directory: bool) -> Result<PathBuf, CliError> {
    if relative == "." && directory {
        return Ok(root.to_owned());
    }
    if !crate::relative_path(relative) {
        return Err(error(2, "path must be confined and relative"));
    }
    let mut path = root.to_owned();
    for component in Path::new(relative).components() {
        let Component::Normal(name) = component else {
            return Err(error(2, "invalid relative path"));
        };
        path.push(name);
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(error(
                2,
                "symlinks inside source/evidence roots are refused",
            ));
        }
    }
    let metadata = fs::metadata(&path).map_err(io_error)?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(error(2, "expected an ordinary file or directory"));
    }
    Ok(path)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, CliError> {
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(error(2, "only ordinary files may be read"));
    }
    if metadata.len() > limit {
        return Err(error(2, "input exceeds the bounded file size"));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io_error)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > limit {
        return Err(error(2, "input exceeds the bounded file size"));
    }
    Ok(bytes)
}

fn read_explicit(path: &Path, limit: u64) -> Result<(PathBuf, Vec<u8>), CliError> {
    let parent = root(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    let name = path
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| error(2, "input filename must be UTF-8"))?;
    let file = confined(&parent, name, false)?;
    Ok((parent, read_bounded(&file, limit)?))
}

fn output_path(path: &Path) -> Result<PathBuf, CliError> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().map_err(io_error)?.join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => return Err(error(2, "output path cannot traverse parents")),
            Component::CurDir => continue,
            _ => current.push(component),
        }
        match fs::symlink_metadata(&current) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(error(2, "symlinked output paths are refused"));
            }
            Ok(_) if current == absolute => return Err(error(2, "output already exists")),
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(error(2, "output parent is not a directory")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && current == absolute => {}
            Err(e) => return Err(io_error(e)),
        }
    }
    Ok(absolute)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let path = output_path(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| error(2, "output lacks parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    temporary.write_all(bytes).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    temporary
        .persist_noclobber(&path)
        .map_err(|e| io_error(e.error))?;
    Ok(())
}

fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, CliError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(invalid)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn print_json(value: &impl Serialize) -> Result<(), CliError> {
    std::io::stdout()
        .write_all(&json_bytes(value)?)
        .map_err(io_error)
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, CliError> {
    Command::new("git")
        .args([
            "--no-pager",
            "--literal-pathspecs",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-C",
        ])
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(io_error)
}

fn revision(root: &Path) -> Result<String, CliError> {
    let output = git(root, &["rev-parse", "--verify", "HEAD"])?;
    let revision = String::from_utf8(output.stdout)
        .map_err(|_| error(2, "Git revision is not UTF-8"))?
        .trim()
        .to_owned();
    if !output.status.success() || !crate::hex(&revision, &[40, 64]) {
        return Err(error(2, "source root requires a committed Git revision"));
    }
    Ok(revision)
}

fn selected_dirty(root: &Path, paths: &[String]) -> Result<bool, CliError> {
    // Git status/diff may invoke configured clean filters. Compare unfiltered
    // committed blob bytes instead; worktree conversion differences are dirty.
    let mut args = vec!["ls-tree", "-z", "--full-tree", "HEAD", "--"];
    args.extend(paths.iter().map(String::as_str));
    let output = git(root, &args)?;
    if !output.status.success() {
        return Err(error(70, "could not inspect selected Git input status"));
    }
    let mut entries = std::collections::BTreeMap::new();
    for row in output
        .stdout
        .split(|b| *b == 0)
        .filter(|row| !row.is_empty())
    {
        let (meta, path) = row.split_at(
            row.iter()
                .position(|b| *b == b'\t')
                .ok_or_else(|| error(70, "invalid Git tree record"))?,
        );
        let meta = std::str::from_utf8(meta).map_err(|_| error(70, "invalid Git tree metadata"))?;
        let fields: Vec<_> = meta.split_whitespace().collect();
        if fields.len() != 3 || fields[1] != "blob" {
            continue;
        }
        let path = std::str::from_utf8(&path[1..])
            .map_err(|_| error(70, "selected Git path is not UTF-8"))?;
        entries.insert(path.to_owned(), fields[2].to_owned());
    }
    for path in paths {
        let Some(object) = entries.get(path) else {
            return Ok(true);
        };
        if !crate::hex(object, &[40, 64]) {
            return Err(error(70, "invalid Git object identifier"));
        }
        let blob = git(root, &["cat-file", "blob", object])?;
        if !blob.status.success() {
            return Err(error(70, "committed source blob is unavailable"));
        }
        let current = match confined(root, path, false) {
            Ok(file) => read_bounded(&file, MAX_FILE)?,
            Err(_) if !root.join(path).try_exists().map_err(io_error)? => return Ok(true),
            Err(e) => return Err(e),
        };
        if blob.stdout != current {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Serialize)]
struct SnapshotInput {
    path: String,
    sha256: String,
    bytes: Vec<u8>,
}

fn inputs(root: &Path, paths: &[String], missing_ok: bool) -> Result<Vec<Artifact>, CliError> {
    let mut result = Vec::new();
    for path in paths {
        match confined(root, path, false) {
            Ok(file) => result.push(Artifact {
                path: path.clone(),
                sha256: crate::sha256(&read_bounded(&file, MAX_FILE)?),
            }),
            Err(_) if missing_ok && !root.join(path).try_exists().map_err(io_error)? => {
                // Re-walk existing parents to distinguish deletion from a symlink
                // escape or an unreadable path. Only NotFound is an omission.
                let mut p = root.to_owned();
                for c in Path::new(path).components() {
                    p.push(c);
                    match fs::symlink_metadata(&p) {
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                        Err(e) => return Err(io_error(e)),
                        Ok(m) if m.file_type().is_symlink() => {
                            return Err(error(2, "symlinked input refused"));
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(result)
}

fn source_snapshot(
    root: &Path,
    paths: &[String],
    revision: &str,
    expected: &[Artifact],
) -> Result<Vec<u8>, CliError> {
    let mut files = Vec::new();
    let mut total = 0usize;
    for path in paths {
        let bytes = read_bounded(&confined(root, path, false)?, MAX_JSON)?;
        total = total.saturating_add(bytes.len());
        if total > (MAX_JSON / 5) as usize {
            return Err(error(
                2,
                "dirty selected inputs exceed private snapshot bound",
            ));
        }
        let digest = crate::sha256(&bytes);
        if !expected
            .iter()
            .any(|a| &a.path == path && a.sha256 == digest)
        {
            return Err(error(
                1,
                "selected inputs changed while capturing the dirty snapshot",
            ));
        }
        files.push(SnapshotInput {
            path: path.clone(),
            sha256: digest,
            bytes,
        });
    }
    json_bytes(&serde_json::json!({"revision":revision,"selected_inputs":files}))
}

fn read_plan(path: &Path) -> Result<(PathBuf, QualityPlan), CliError> {
    let (parent, bytes) = read_explicit(path, MAX_JSON)?;
    Ok((parent, QualityPlan::parse(&bytes).map_err(invalid)?))
}

fn init(options: InitOptions) -> Result<i32, CliError> {
    let root = root(&options.root)?;
    let out = output_path(&options.out)?;
    let mut paths = options.input;
    paths.sort();
    if paths.windows(2).any(|p| p[0] == p[1]) {
        return Err(error(2, "input paths must be unique"));
    }
    for path in &paths {
        if !crate::relative_path(path) {
            return Err(error(2, "input paths must be relative"));
        }
    }
    let revision = revision(&root)?;
    let selected = inputs(&root, &paths, false)?;
    let dirty = selected_dirty(&root, &paths)?;
    let snapshot = if dirty {
        Some(source_snapshot(&root, &paths, &revision, &selected)?)
    } else {
        None
    };
    let snapshot_directory = format!(
        ".{}.evidence",
        out.file_name()
            .and_then(|p| p.to_str())
            .ok_or_else(|| error(2, "plan filename must be UTF-8"))?
    );
    let snapshot_name = format!("{snapshot_directory}/selected-inputs.json");
    let working_tree = snapshot.as_ref().map(|bytes| Artifact {
        path: snapshot_name.clone(),
        sha256: crate::sha256(bytes),
    });
    let plan = QualityPlan {
        version: 1,
        id: options.id,
        subject: Subject {
            repository: options.repository.unwrap_or_else(|| {
                root.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            }),
            revision,
            configuration: options.configuration,
            dirty,
            working_tree,
            inputs: selected,
        },
        checks: vec![CheckSpec {
            id: options.check,
            required: true,
            invocation: Invocation {
                program: options.command[0].clone(),
                args: options.command[1..].to_vec(),
                cwd: options.cwd,
                env: options.env,
                timeout_ms: options.timeout_ms,
                max_output_bytes: options.max_output_bytes,
            },
            evaluator: Evaluator::ExitCode {
                success_codes: vec![0],
            },
            scope: paths.clone(),
        }],
        expectations: vec![],
        policy: GatePolicy {
            max_age_ms: Some(30 * 24 * 60 * 60 * 1000),
            require_observed_scope: false,
        },
    };
    plan.validate().map_err(invalid)?;
    confined(&root, &plan.checks[0].invocation.cwd, true)?;
    if inputs(&root, &paths, false)? != plan.subject.inputs {
        return Err(error(1, "selected inputs changed while preparing the plan"));
    }
    if let Some(bytes) = &snapshot {
        let parent = out.parent().ok_or_else(|| error(2, "plan needs parent"))?;
        let snapshot_root = parent.join(&snapshot_directory);
        process::private_directory(&snapshot_root).map_err(io_error)?;
        write_new(&snapshot_root.join(".gitignore"), b"*\n")?;
        let snapshot_path = parent.join(&snapshot_name);
        write_new(&snapshot_path, bytes)?;
    }
    write_new(&out, &json_bytes(&plan)?)?;
    print_json(
        &serde_json::json!({"operation":"plan-init","plan":out,"plan_sha256":plan.digest().map_err(invalid)?,"check_commands_executed":false,"selected_inputs":paths.len()}),
    )?;
    Ok(0)
}

fn add_artifact(
    verified: &mut VerifiedArtifacts,
    root: &Path,
    artifact: &Artifact,
) -> Result<(), CliError> {
    crate::artifact_shape(artifact).map_err(invalid)?;
    let bytes = read_bounded(&confined(root, &artifact.path, false)?, MAX_FILE)?;
    verified.insert(artifact, &bytes).map_err(invalid)
}

fn receipt_artifacts(receipt: &ExecutionReceipt) -> Vec<&Artifact> {
    receipt
        .stdout
        .iter()
        .chain(receipt.stderr.iter())
        .chain(receipt.subject.working_tree.iter())
        .chain(receipt.reported.iter().map(|r| &r.artifact))
        .collect()
}

fn evidence_parent(root: &Path, path: &str) -> Result<PathBuf, CliError> {
    if !crate::relative_path(path) {
        return Err(error(2, "artifact output path is not confined"));
    }
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            process::private_directory(parent).map_err(io_error)?;
        }
    }
    output_path(&path)
}

fn process_status(termination: Termination) -> ProcessStatus {
    match termination {
        Termination::Completed => ProcessStatus::Completed,
        Termination::Unavailable => ProcessStatus::Unavailable,
        Termination::FailedToStart => ProcessStatus::FailedToStart,
        Termination::TimedOut => ProcessStatus::TimedOut,
        Termination::Interrupted => ProcessStatus::Interrupted,
        Termination::CaptureFailed => ProcessStatus::CaptureFailed,
    }
}

fn execution_exit(receipt: &ExecutionReceipt, evaluator: &Evaluator) -> i32 {
    match receipt.process {
        ProcessStatus::Unavailable => 78,
        ProcessStatus::TimedOut => 124,
        ProcessStatus::Interrupted => 130,
        ProcessStatus::FailedToStart | ProcessStatus::CaptureFailed => 70,
        ProcessStatus::Completed => match evaluator {
            Evaluator::ExitCode { success_codes }
                if receipt
                    .exit_code
                    .is_some_and(|code| success_codes.contains(&code)) =>
            {
                0
            }
            _ => 1,
        },
    }
}

fn run(options: RunOptions, cancelled: &AtomicBool) -> Result<i32, CliError> {
    let source_root = root(&options.root)?;
    let (plan_root, plan) = read_plan(&options.plan)?;
    let selected: Vec<_> = plan
        .checks
        .iter()
        .filter(|c| options.all || options.check.as_deref() == Some(&c.id))
        .collect();
    if selected.is_empty() {
        return Err(error(2, "selected check does not exist"));
    }
    for check in &selected {
        if !matches!(check.evaluator, Evaluator::ExitCode { .. }) {
            return Err(error(
                78,
                "reported evaluators require their named adapter; generic run cannot invent an assessment",
            ));
        }
        confined(&source_root, &check.invocation.cwd, true)?;
    }
    let paths: Vec<_> = plan.subject.inputs.iter().map(|a| a.path.clone()).collect();
    if revision(&source_root)? != plan.subject.revision
        || inputs(&source_root, &paths, false)? != plan.subject.inputs
    {
        return Err(error(
            1,
            "current selected source does not match the plan; create a new plan",
        ));
    }
    let mut snapshot_bytes = None;
    if let Some(snapshot) = &plan.subject.working_tree {
        let bytes = read_bounded(&confined(&plan_root, &snapshot.path, false)?, MAX_JSON)?;
        if crate::sha256(&bytes) != snapshot.sha256 {
            return Err(error(2, "dirty snapshot hash does not match the plan"));
        }
        snapshot_bytes = Some(bytes);
    }
    process::private_directory(&options.out).map_err(io_error)?;
    let out = root(&options.out)?;
    write_new(&out.join(".gitignore"), b"*\n")?;
    write_new(&out.join("plan.json"), &json_bytes(&plan)?)?;
    if let (Some(snapshot), Some(bytes)) = (&plan.subject.working_tree, snapshot_bytes) {
        write_new(&evidence_parent(&out, &snapshot.path)?, &bytes)?;
    }
    let mut receipts = Vec::new();
    let mut exit = 0;
    for (index, check) in selected.into_iter().enumerate() {
        if inputs(&source_root, &paths, false)? != plan.subject.inputs
            || revision(&source_root)? != plan.subject.revision
        {
            return Err(error(
                1,
                "selected inputs changed before a later check; previous receipts retained",
            ));
        }
        confined(&source_root, &check.invocation.cwd, true)?;
        let directory = format!("check-{index:04}");
        let attempt = out.join(&directory);
        process::private_directory(&attempt).map_err(io_error)?;
        let captured =
            process::run_command(&check.invocation, &source_root, &attempt, cancelled, false)
                .map_err(io_error)?;
        let stdout = Artifact {
            path: format!("{directory}/stdout.log"),
            sha256: crate::sha256(&read_bounded(&captured.stdout, MAX_FILE)?),
        };
        let stderr = Artifact {
            path: format!("{directory}/stderr.log"),
            sha256: crate::sha256(&read_bounded(&captured.stderr, MAX_FILE)?),
        };
        let mut status = process_status(captured.termination);
        let mut diagnostic = captured.error;
        if status == ProcessStatus::Completed && captured.signal.is_some() {
            status = ProcessStatus::Interrupted;
            diagnostic = Some("process terminated by a signal".into());
        }
        let after = match inputs(&source_root, &paths, true) {
            Ok(after) => after,
            Err(_) => {
                status = ProcessStatus::CaptureFailed;
                diagnostic = Some("could not capture selected inputs after execution".into());
                vec![]
            }
        };
        if status != ProcessStatus::Completed && diagnostic.is_none() {
            diagnostic = Some(format!("execution ended as {status:?}"));
        }
        let receipt = ExecutionReceipt {
            version: 1,
            attempt_id: format!(
                "{}-{}-{}",
                check.id,
                captured.started_at_ms,
                std::process::id()
            ),
            plan_sha256: plan.digest().map_err(invalid)?,
            check_id: check.id.clone(),
            subject: plan.subject.clone(),
            invocation: check.invocation.clone(),
            executable: check.invocation.program.clone(),
            executable_sha256: None,
            tool_version: None,
            started_at_ms: captured.started_at_ms,
            finished_at_ms: captured.finished_at_ms,
            elapsed_ms: captured.elapsed_ms,
            process: status,
            exit_code: captured.exit_code,
            signal: captured.signal,
            capture_complete: status == ProcessStatus::Completed,
            stdout: Some(stdout),
            stderr: Some(stderr),
            inputs_after: after,
            observed_scope: None,
            observed_tests: vec![],
            reported: None,
            diagnostic,
        };
        receipt.validate().map_err(invalid)?;
        let path = attempt.join("receipt.json");
        write_new(&path, &json_bytes(&receipt)?)?;
        let code = execution_exit(&receipt, &check.evaluator);
        if exit == 0 && code != 0 {
            exit = code;
        }
        if receipt.inputs_after != plan.subject.inputs && exit == 0 {
            exit = 1;
        }
        receipts.push(path);
        if receipt.process != ProcessStatus::Completed {
            break;
        }
    }
    print_json(
        &serde_json::json!({"operation":"run","gate_evaluated":false,"receipts":receipts,"exit":exit}),
    )?;
    Ok(exit)
}

fn load_receipts(
    directory: &Path,
    verified: &mut VerifiedArtifacts,
) -> Result<Vec<ExecutionReceipt>, CliError> {
    let root = root(directory)?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(&root).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let metadata = fs::symlink_metadata(entry.path()).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(error(2, "symlinked entries in run evidence are refused"));
        }
        if name == "receipt.json" && metadata.is_file() {
            paths.push(entry.path());
        } else if name.starts_with("check-") && metadata.is_dir() {
            let file = confined(&root, &format!("{name}/receipt.json"), false)?;
            paths.push(file);
        }
    }
    paths.sort();
    let mut receipts = Vec::new();
    for path in paths {
        let receipt = ExecutionReceipt::parse(&read_bounded(&path, MAX_JSON)?).map_err(invalid)?;
        for artifact in receipt_artifacts(&receipt) {
            add_artifact(verified, &root, artifact)?;
        }
        receipts.push(receipt);
    }
    Ok(receipts)
}

fn evaluate_options(options: EvaluateOptions) -> Result<i32, CliError> {
    let source_root = root(&options.root)?;
    let (_, plan) = read_plan(&options.plan)?;
    let paths: Vec<_> = plan.subject.inputs.iter().map(|a| a.path.clone()).collect();
    let mut current = plan.subject.clone();
    current.revision = revision(&source_root)?;
    current.inputs = inputs(&source_root, &paths, true)?;
    // An absent selected file is stale evidence, not a synthetic empty subject.
    if current.inputs.is_empty() {
        return Err(error(
            1,
            "all selected source inputs are missing; evidence is stale",
        ));
    }
    current.dirty = selected_dirty(&source_root, &paths)?;
    current.working_tree = if current.dirty {
        Some(Artifact {
            path: "current-selected-inputs.json".into(),
            sha256: crate::sha256(&json_bytes(&current.inputs)?),
        })
    } else {
        None
    };
    if let Some(path) = &options.current_subject {
        let (_, bytes) = read_explicit(path, MAX_JSON)?;
        let supplied: Subject = crate::from_json(&bytes).map_err(invalid)?;
        supplied.validate().map_err(invalid)?;
        if supplied.repository != current.repository
            || supplied.configuration != current.configuration
            || supplied.revision != current.revision
            || supplied.inputs != current.inputs
            || supplied.dirty != current.dirty
        {
            return Err(error(
                2,
                "supplied current subject contradicts the observed source snapshot",
            ));
        }
    }
    let mut verified = VerifiedArtifacts::default();
    let mut receipts = Vec::new();
    for directory in &options.receipts {
        receipts.extend(load_receipts(directory, &mut verified)?);
    }
    let mut context_data = ContextFile::default();
    if let Some(path) = &options.context {
        let (context_root, bytes) = read_explicit(path, MAX_JSON)?;
        context_data = crate::from_json(&bytes).map_err(invalid)?;
        for artifact in context_data
            .audits
            .iter()
            .map(|a| &a.artifact)
            .chain(context_data.manual_observations.iter().map(|m| &m.artifact))
        {
            add_artifact(&mut verified, &context_root, artifact)?;
        }
    }
    let mut ancestors = Vec::new();
    if current.revision != plan.subject.revision
        && git(
            &source_root,
            &[
                "merge-base",
                "--is-ancestor",
                &plan.subject.revision,
                &current.revision,
            ],
        )?
        .status
        .success()
    {
        ancestors.push(plan.subject.revision.clone());
    }
    let mut context =
        crate::EvaluationContext::new(options.as_of_ms.unwrap_or_else(now_ms), &current, &verified);
    context.ancestor_revisions = &ancestors;
    context.audits = &context_data.audits;
    context.manual_observations = &context_data.manual_observations;
    let mut aliases = BTreeSet::new();
    for declaration in context_data.families {
        if !aliases.insert(declaration.route.clone()) {
            return Err(error(2, "duplicate model-family declaration"));
        }
        context
            .families
            .declare(
                &declaration.route,
                sealmap_corpus::route::Family::named(&declaration.family).map_err(invalid)?,
            )
            .map_err(invalid)?;
    }
    let report = crate::evaluate(&plan, &receipts, &context);
    if options.json {
        print_json(&report)?;
    } else {
        println!(
            "{:?}: {}/{} required checks passed; {} expectation(s)",
            report.outcome,
            report.passed_required_checks,
            report.required_checks,
            report.expectations.len()
        );
        for check in &report.checks {
            println!(
                "{}: {:?} — {}",
                check.id,
                check.outcome,
                check.reasons.join("; ")
            );
        }
        for expectation in &report.expectations {
            println!(
                "{}: {:?} — {}",
                expectation.id,
                expectation.outcome,
                expectation.reasons.join("; ")
            );
        }
        for error in &report.errors {
            eprintln!("visioncraft: {error}");
        }
    }
    Ok(match report.outcome {
        GateOutcome::Passed => 0,
        GateOutcome::Blocked | GateOutcome::Incomplete => 1,
        GateOutcome::Error => 70,
    })
}

/// Dispatch an explicit CLI operation. Imported evidence is never executed.
///
/// # Errors
/// Returns typed usage, filesystem or execution failures; raw source bytes are
/// retained privately rather than interpolated into diagnostics.
pub fn execute(cli: Cli, cancelled: &AtomicBool) -> Result<i32, CliError> {
    match cli.command {
        Operation::Plan {
            command: PlanCommand::Init(options),
        } => init(options),
        Operation::Plan {
            command: PlanCommand::Check { plan, json },
        } => {
            let (_, plan) = read_plan(&plan)?;
            if json {
                print_json(
                    &serde_json::json!({"valid":true,"plan_sha256":plan.digest().map_err(invalid)?,"checks":plan.checks.len(),"expectations":plan.expectations.len(),"commands_executed":false}),
                )?;
            } else {
                println!(
                    "Valid plan: {} checks, {} expectations; no check commands executed",
                    plan.checks.len(),
                    plan.expectations.len()
                );
            }
            Ok(0)
        }
        Operation::Run(options) => run(options, cancelled),
        Operation::Evaluate(options)
        | Operation::Gate {
            command: GateCommand::Evaluate(options),
        }
        | Operation::Expectations {
            command: ExpectationCommand::Check(options),
        } => evaluate_options(options),
        Operation::Deepsec(options) => {
            crate::deepsec::execute(&options, cancelled).map_err(|e| error(e.code, e.message))
        }
    }
}
