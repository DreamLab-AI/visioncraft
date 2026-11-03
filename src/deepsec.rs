//! Executed Deepsec 2.3.10 gates with explicit policy and retained error states.
//!
//! This adapter knows no Agentbox manifest, memory service or network address.
//! Hosts project their policy into [`crate::deepsec::DeepsecPolicy`]. Credentials remain named
//! environment variables and are passed only to the model-processing child.

use crate::process::{Termination, private_directory, private_file, run_command_with_env};
use crate::{Invocation, from_json, sha256};
use clap::{ArgGroup, Args};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[path = "deepsec_evidence.rs"]
mod recorded;

static PROTOCOL: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
    serde_json::from_str(include_str!("../contracts/deepsec.json"))
        .expect("checked-in Deepsec release contract must be valid JSON")
});
fn supported_version() -> &'static str {
    PROTOCOL["package"]["version"]
        .as_str()
        .expect("release contract version")
}
const SEVERITIES: [&str; 6] = ["CRITICAL", "HIGH", "MEDIUM", "HIGH_BUG", "BUG", "LOW"];
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// Host-projected policy. Values are configuration; credential values are absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeepsecPolicy {
    /// Whether paid/model processing is allowed at all.
    pub enabled: bool,
    /// Deepsec agent plugin: claude, codex or pi.
    pub agent: String,
    /// Authentication route: local, direct or custom.
    pub model_auth: String,
    /// Optional model override; empty leaves selection to the pinned tool.
    pub model: String,
    /// Deepsec thinking-level parameter.
    pub thinking_level: String,
    /// Inclusive severity threshold; lower findings remain visible.
    pub fail_on: String,
    /// Whole attempt deadline, including probes/export (for example `45m`).
    pub max_duration: String,
    /// Files per model batch.
    pub batch_size: u32,
    /// Maximum concurrent batches inside Deepsec.
    pub concurrency: u32,
    /// Provider identifier for direct/custom authentication.
    pub ai_provider: String,
    /// Name of a credential environment variable, never its value.
    pub ai_api_key_env: String,
    /// Optional custom/direct base URL, without credentials or query strings.
    pub ai_base_url: String,
    /// Non-secret policy origin for receipts.
    pub policy_source: String,
    /// Combined stdout/stderr bound for each child, at most one GiB.
    pub max_output_bytes: u64,
}

impl Default for DeepsecPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            agent: "claude".into(),
            model_auth: "local".into(),
            model: String::new(),
            thinking_level: "high".into(),
            fail_on: "HIGH".into(),
            max_duration: "45m".into(),
            batch_size: 5,
            concurrency: 2,
            ai_provider: String::new(),
            ai_api_key_env: String::new(),
            ai_base_url: String::new(),
            policy_source: "defaults".into(),
            max_output_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Adapter input/runtime error with the documented legacy-compatible exit code.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct GateError {
    /// 70 runtime, 78 configuration/unavailable, 124 timeout, or 130 interruption.
    pub code: i32,
    /// Message deliberately excludes raw configuration and credential values.
    pub message: String,
}

fn error(code: i32, message: impl Into<String>) -> GateError {
    GateError {
        code,
        message: message.into(),
    }
}
fn io_error(_: std::io::Error) -> GateError {
    error(70, "evidence filesystem operation failed")
}

fn duration_ms(text: &str) -> Option<u64> {
    let (number, scale) = if let Some(n) = text.strip_suffix("ms") {
        (n, 1_u64)
    } else if let Some(n) = text.strip_suffix('s') {
        (n, 1000)
    } else if let Some(n) = text.strip_suffix('m') {
        (n, 60_000)
    } else if let Some(n) = text.strip_suffix('h') {
        (n, 3_600_000)
    } else if let Some(n) = text.strip_suffix('d') {
        (n, 86_400_000)
    } else {
        (text, 1000)
    };
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || fraction.len() > 3
        || !fraction.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let base = whole.parse::<u64>().ok()?.checked_mul(scale)?;
    let part = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().ok()?.checked_mul(scale)? / 10_u64.pow(fraction.len() as u32)
    };
    base.checked_add(part)
        .filter(|ms| (1..=86_400_000).contains(ms))
}

impl DeepsecPolicy {
    /// Validate policy and return the whole-attempt deadline in milliseconds.
    /// No environment is read and no process is started.
    pub fn validate(&self) -> Result<u64, GateError> {
        if !["claude", "codex", "pi"].contains(&self.agent.as_str()) {
            return Err(error(78, "unsupported Deepsec agent"));
        }
        if !["local", "direct", "custom"].contains(&self.model_auth.as_str()) {
            return Err(error(78, "unsupported model authentication route"));
        }
        if !["minimal", "low", "medium", "high", "xhigh"].contains(&self.thinking_level.as_str()) {
            return Err(error(78, "unsupported thinking level"));
        }
        if !SEVERITIES.contains(&self.fail_on.as_str()) {
            return Err(error(78, "unsupported severity threshold"));
        }
        if !(1..=100).contains(&self.batch_size)
            || !(1..=32).contains(&self.concurrency)
            || !(1..=1_073_741_824).contains(&self.max_output_bytes)
        {
            return Err(error(78, "invalid batch, concurrency or output bound"));
        }
        if [&self.model, &self.policy_source, &self.ai_provider]
            .iter()
            .any(|x| x.contains('\0'))
        {
            return Err(error(78, "invalid policy string"));
        }
        if !self.ai_api_key_env.is_empty()
            && !(self
                .ai_api_key_env
                .as_bytes()
                .first()
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
                && self
                    .ai_api_key_env
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_'))
        {
            return Err(error(78, "invalid credential environment name"));
        }
        if !self.ai_base_url.is_empty() {
            let url = url::Url::parse(&self.ai_base_url)
                .map_err(|_| error(78, "invalid model base URL"))?;
            if !["http", "https"].contains(&url.scheme())
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(error(
                    78,
                    "base URL must be HTTP(S), without credentials, query or fragment",
                ));
            }
        }
        match self.model_auth.as_str() {
            "local" if self.agent == "pi" => {
                return Err(error(78, "local authentication requires claude or codex"));
            }
            "direct"
                if !["anthropic", "openai"].contains(&self.ai_provider.as_str())
                    || self.ai_api_key_env.is_empty() =>
            {
                return Err(error(
                    78,
                    "direct authentication requires a supported provider and credential name",
                ));
            }
            "custom"
                if self.agent != "pi"
                    || self.ai_api_key_env.is_empty()
                    || self.ai_base_url.is_empty() =>
            {
                return Err(error(
                    78,
                    "custom authentication requires pi, a credential name and a base URL",
                ));
            }
            _ => (),
        }
        duration_ms(&self.max_duration).ok_or_else(|| {
            error(
                78,
                "invalid duration; use positive seconds/ms/m/h/d up to one day",
            )
        })
    }
}

/// Deepsec gate flags. Mutually exclusive source modes are checked by Clap.
#[derive(Debug, Clone, Args)]
#[command(group(ArgGroup::new("source").required(true).multiple(false).args(["diff", "diff_working", "diff_staged", "files_from", "full", "scan_only"])))]
pub struct DeepsecOptions {
    /// Compare against this Git reference.
    #[arg(long)]
    pub diff: Option<String>,
    /// Review working and untracked changes.
    #[arg(long)]
    pub diff_working: bool,
    /// Review staged changes.
    #[arg(long)]
    pub diff_staged: bool,
    /// Read newline-delimited file paths, or `-` for stdin.
    #[arg(long)]
    pub files_from: Option<PathBuf>,
    /// Scan the repository and process its pending candidates.
    #[arg(long)]
    pub full: bool,
    /// Scan without model processing; result is SCANNED, not review PASS.
    #[arg(long)]
    pub scan_only: bool,
    /// Source root, otherwise the nearest containing Git worktree or cwd.
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Fresh output directory; existing directories are refused.
    #[arg(long)]
    pub report_dir: Option<PathBuf>,
    /// Typed JSON policy from a host adapter; contains credential names only.
    #[arg(long)]
    pub policy_json: Option<String>,
    /// Override the severity threshold.
    #[arg(long)]
    pub fail_on: Option<String>,
    /// Override the thinking level.
    #[arg(long)]
    pub thinking_level: Option<String>,
    /// Override the Deepsec agent.
    #[arg(long)]
    pub agent: Option<String>,
    /// Override the model identifier.
    #[arg(long)]
    pub model: Option<String>,
    /// Override the authentication route.
    #[arg(long)]
    pub model_auth: Option<String>,
    /// Print the resolved plan without running any command or writing evidence.
    #[arg(long)]
    pub dry_run: bool,
}

fn root_for(options: &DeepsecOptions) -> Result<PathBuf, GateError> {
    let path = if let Some(root) = &options.root {
        root.clone()
    } else {
        let cwd = std::env::current_dir().map_err(io_error)?;
        cwd.ancestors()
            .find(|p| p.join(".git").exists())
            .unwrap_or(&cwd)
            .to_owned()
    };
    let path = path
        .canonicalize()
        .map_err(|_| error(78, "source root is unavailable"))?;
    if !path.is_dir() {
        return Err(error(78, "source root must be a directory"));
    }
    Ok(path)
}

fn environment(policy: &DeepsecPolicy, model: bool) -> Vec<String> {
    let mut names: Vec<String> = [
        "PATH",
        "HOME",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "NODE_EXTRA_CA_CERTS",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if model {
        names.extend(
            ["CLAUDE_CODE_EXECUTABLE", "CLAUDE_CONFIG_DIR", "CODEX_HOME"]
                .into_iter()
                .map(str::to_owned),
        );
        if policy.model_auth == "local" {
            names.push("CLAUDE_CODE_OAUTH_TOKEN".into());
        } else {
            names.push(policy.ai_api_key_env.clone());
        }
    }
    names.sort();
    names.dedup();
    names
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), GateError> {
    let parent = path
        .parent()
        .ok_or_else(|| error(70, "evidence path has no parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    temporary.write_all(bytes).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    temporary
        .persist_noclobber(path)
        .map_err(|_| error(70, "evidence output already exists or cannot be finalised"))?;
    Ok(())
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, GateError> {
    let meta = fs::symlink_metadata(path).map_err(io_error)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > limit {
        return Err(error(70, "evidence must be a bounded regular file"));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io_error)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > limit {
        return Err(error(70, "evidence exceeded its byte bound"));
    }
    Ok(bytes)
}

fn findings(bytes: &[u8]) -> Result<Vec<Value>, GateError> {
    let value: StrictJson = serde_json::from_slice(bytes)
        .map_err(|_| error(70, "malformed or ambiguous findings JSON"))?;
    let value = value.0;
    let rows = value
        .as_array()
        .or_else(|| value.get("findings").and_then(Value::as_array))
        .ok_or_else(|| error(70, "unrecognised findings export shape"))?;
    let mut output = Vec::new();
    for row in rows {
        if row.get("severity").is_some() {
            output.push(row.clone());
        } else if let Some(nested) = row.get("findings").and_then(Value::as_array) {
            for finding in nested {
                let mut object = finding
                    .as_object()
                    .cloned()
                    .ok_or_else(|| error(70, "finding must be an object"))?;
                if let Some(path) = row.get("filePath") {
                    object.entry("filePath").or_insert_with(|| path.clone());
                }
                output.push(Value::Object(object));
            }
        } else {
            return Err(error(70, "unrecognised finding row"));
        }
    }
    for finding in &mut output {
        let severity = finding
            .get("severity")
            .and_then(Value::as_str)
            .ok_or_else(|| error(70, "finding lacks a string severity"))?
            .to_ascii_uppercase();
        if !SEVERITIES.contains(&severity.as_str()) {
            return Err(error(70, "finding has an unrecognised severity"));
        }
        finding["severity"] = Value::String(severity);
    }
    Ok(output)
}

// Findings come from an external protocol. Reject duplicate keys at every level,
// including severity and findings, rather than silently accepting the last key.
struct StrictJson(Value);
impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("unambiguous JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<StrictJson, E> {
                Ok(StrictJson(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<StrictJson, E> {
                Ok(StrictJson(Value::Number(v.into())))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<StrictJson, E> {
                Ok(StrictJson(Value::Number(v.into())))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<StrictJson, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| StrictJson(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<StrictJson, E> {
                Ok(StrictJson(Value::String(v.into())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<StrictJson, E> {
                Ok(StrictJson(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<StrictJson, E> {
                Ok(StrictJson(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<StrictJson, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<StrictJson>()? {
                    values.push(value.0);
                }
                Ok(StrictJson(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<StrictJson, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                    values.insert(key, map.next_value::<StrictJson>()?.0);
                }
                Ok(StrictJson(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

struct Attempt<'a> {
    policy: &'a DeepsecPolicy,
    report: &'a Path,
    cancelled: &'a AtomicBool,
    started: Instant,
    deadline: Duration,
    sequence: u32,
}

impl Attempt<'_> {
    fn command(
        &mut self,
        stage: &str,
        program: &str,
        args: Vec<String>,
        cwd: &Path,
        model: bool,
        stdin: bool,
    ) -> Result<(i32, Vec<u8>), GateError> {
        let remaining = self
            .deadline
            .checked_sub(self.started.elapsed())
            .filter(|v| !v.is_zero())
            .ok_or_else(|| error(124, "whole-attempt deadline reached"))?;
        let directory = self.report.join(format!("{:02}-{stage}", self.sequence));
        self.sequence += 1;
        private_directory(&directory).map_err(io_error)?;
        let (args, env, fixed) = if program == "git" {
            let mut hardened = crate::git_metadata::prefix();
            hardened.extend(args);
            (
                hardened,
                vec!["PATH".into()],
                crate::git_metadata::fixed_environment(),
            )
        } else {
            (args, environment(self.policy, model), BTreeMap::new())
        };
        let invocation = Invocation {
            program: program.into(),
            args,
            cwd: ".".into(),
            env,
            timeout_ms: remaining.as_millis().max(1).try_into().unwrap_or(u64::MAX),
            max_output_bytes: self.policy.max_output_bytes,
        };
        write_new(
            &directory.join("invocation.json"),
            &serde_json::to_vec_pretty(&invocation)
                .map_err(|_| error(70, "invocation serialisation failed"))?,
        )?;
        let run = run_command_with_env(&invocation, cwd, &directory, self.cancelled, stdin, &fixed)
            .map_err(io_error)?;
        let stdout = read_bounded(&run.stdout, self.policy.max_output_bytes)?;
        let stderr = read_bounded(&run.stderr, self.policy.max_output_bytes)?;
        let mut log = OpenOptions::new()
            .append(true)
            .open(self.report.join("deepsec.log"))
            .map_err(io_error)?;
        log.write_all(&stdout)
            .and_then(|_| log.write_all(&stderr))
            .map_err(io_error)?;
        let execution = json!({"stage":stage,"process":format!("{:?}",run.termination),"exit_code":run.exit_code,"signal":run.signal,"started_at_ms":run.started_at_ms,"finished_at_ms":run.finished_at_ms,"elapsed_ms":run.elapsed_ms,"stdout_sha256":sha256(&stdout),"stderr_sha256":sha256(&stderr),"error":run.error});
        write_new(
            &directory.join("execution.json"),
            &serde_json::to_vec_pretty(&execution)
                .map_err(|_| error(70, "execution serialisation failed"))?,
        )?;
        match run.termination {
            Termination::Completed if run.signal.is_none() => Ok((
                run.exit_code
                    .ok_or_else(|| error(70, "missing exit status"))?,
                stdout,
            )),
            Termination::Unavailable => Err(error(78, "required executable is unavailable")),
            Termination::TimedOut => Err(error(
                124,
                "whole-attempt deadline reached; checkpoint evidence retained",
            )),
            Termination::Interrupted => Err(error(
                130,
                "review interrupted; checkpoint evidence retained",
            )),
            _ => Err(error(70, "review process or capture did not complete")),
        }
    }
}

#[cfg(unix)]
fn workspace_lock(workspace: &Path) -> Result<nix::fcntl::Flock<File>, GateError> {
    use nix::fcntl::{Flock, FlockArg};
    use std::os::unix::fs::OpenOptionsExt;
    let path = workspace.join(".visioncraft.lock");
    if fs::symlink_metadata(&path).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err(error(78, "invalid review workspace lock"));
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(io_error)?;
    Flock::lock(file, FlockArg::LockExclusiveNonblock)
        .map_err(|_| error(78, "another review owns this workspace"))
}

#[cfg(unix)]
fn validate_workspace_tree(workspace: &Path) -> Result<(), GateError> {
    use std::os::unix::fs::MetadataExt;
    // Deepsec owns durable checkpoint state here. Reject redirected/special
    // entries before passing that tree to an external tool with credentials.
    // This does not isolate concurrent writers running as the same Unix user.
    let mut pending = vec![(workspace.to_owned(), 0_usize)];
    let mut count = 0_usize;
    while let Some((directory, depth)) = pending.pop() {
        if depth > 64 {
            return Err(error(78, "review workspace exceeds traversal bounds"));
        }
        for entry in fs::read_dir(directory).map_err(io_error)? {
            count += 1;
            if count > 100_000 {
                return Err(error(78, "review workspace exceeds traversal bounds"));
            }
            let entry = entry.map_err(io_error)?;
            let meta = fs::symlink_metadata(entry.path()).map_err(io_error)?;
            if meta.file_type().is_symlink()
                || !(meta.is_dir() || meta.is_file())
                || (meta.is_file() && meta.nlink() != 1)
            {
                return Err(error(
                    78,
                    "review workspace contains redirected or special entries",
                ));
            }
            if meta.is_dir() {
                pending.push((entry.path(), depth + 1));
            }
        }
    }
    Ok(())
}

/// Execute an explicitly requested security gate, returning its legacy exit code.
///
/// Dry-run prints only the resolved plan. Completed, failed and interrupted
/// attempts write fresh receipts; unknown exports and incomplete execution can
/// never be interpreted as an empty successful review.
pub fn execute(options: &DeepsecOptions, cancelled: &AtomicBool) -> Result<i32, GateError> {
    let modes = [
        options.diff.is_some(),
        options.diff_working,
        options.diff_staged,
        options.files_from.is_some(),
        options.full,
        options.scan_only,
    ];
    if modes.into_iter().filter(|x| *x).count() != 1
        || options.diff.as_ref().is_some_and(|v| v.trim().is_empty())
    {
        return Err(error(78, "choose exactly one nonempty source mode"));
    }
    #[cfg(not(unix))]
    {
        let _ = (options, cancelled);
        Err(error(78, "Deepsec process adapter requires Unix"))
    }
    #[cfg(unix)]
    {
        let mut policy = match &options.policy_json {
            Some(text) => from_json::<DeepsecPolicy>(text.as_bytes())
                .map_err(|_| error(78, "invalid policy JSON"))?,
            None => DeepsecPolicy::default(),
        };
        if let Some(v) = &options.agent {
            policy.agent = v.clone();
        }
        if let Some(v) = &options.model {
            policy.model = v.clone();
        }
        if let Some(v) = &options.model_auth {
            policy.model_auth = v.clone();
        }
        if let Some(v) = &options.thinking_level {
            policy.thinking_level = v.clone();
        }
        if let Some(v) = &options.fail_on {
            policy.fail_on = v.to_ascii_uppercase();
        }
        let deadline_ms = policy.validate()?;
        if !policy.enabled {
            return Err(error(
                78,
                "Deepsec gate disabled by policy; record SKIPPED, not passed",
            ));
        }
        let root = root_for(options)?;
        let cwd = std::env::current_dir().map_err(io_error)?;
        let files = match &options.files_from {
            Some(path) if path == Path::new("-") => Some(path.clone()),
            Some(path) => {
                let path = cwd
                    .join(path)
                    .canonicalize()
                    .map_err(|_| error(78, "file manifest is unavailable"))?;
                read_bounded(&path, 8 * 1024 * 1024)
                    .map_err(|_| error(78, "file manifest must be a readable bounded file"))?;
                Some(path)
            }
            None => None,
        };
        let mode = if options.scan_only {
            "scan-only"
        } else if options.full {
            "full"
        } else if files.is_some() {
            "files-from"
        } else if options.diff_working {
            "diff-working"
        } else if options.diff_staged {
            "diff-staged"
        } else {
            "diff"
        };
        let workspace = root.join(".deepsec-gate");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let attempt_id = format!(
            "{now}-{}-{}",
            std::process::id(),
            ATTEMPT.fetch_add(1, Ordering::SeqCst)
        );
        let report = options
            .report_dir
            .as_ref()
            .map(|p| cwd.join(p))
            .unwrap_or_else(|| workspace.join("reports").join(&attempt_id));
        let project_id: String = root
            .file_name()
            .map_or_else(|| "project".into(), |v| v.to_string_lossy())
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || "_.-".contains(c) {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let mut route = match policy.model_auth.as_str() {
            "local" => json!({"mode":"local","provider":"local"}),
            "direct" => {
                json!({"mode":"direct","provider":policy.ai_provider,"apiKeyEnv":policy.ai_api_key_env})
            }
            _ => {
                json!({"mode":"custom","provider":if policy.ai_provider.is_empty(){"custom"}else{&policy.ai_provider},"apiKeyEnv":policy.ai_api_key_env,"credentialHeader":{"name":"authorization","scheme":"bearer"}})
            }
        };
        if !policy.ai_base_url.is_empty() {
            route["baseUrl"] = json!(policy.ai_base_url);
        }
        let plan = json!({"version":1,"attempt_id":attempt_id,"mode":mode,"diff_ref":options.diff,"files_from":files,"root":root,"project_id":project_id,"workspace":workspace,"report_dir":report,"deepsec_version_required":supported_version(),"deepsec_protocol_version":PROTOCOL["adapter_protocol_version"],"policy":policy,"route":route});
        if options.dry_run {
            println!(
                "{}",
                serde_json::to_string_pretty(&plan)
                    .map_err(|_| error(70, "plan serialisation failed"))?
            );
            return Ok(0);
        }
        if !options.scan_only
            && policy.model_auth != "local"
            && std::env::var_os(&policy.ai_api_key_env).is_none_or(|v| v.is_empty())
        {
            return Err(error(
                78,
                "configured credential environment variable is unset",
            ));
        }
        match fs::symlink_metadata(&workspace) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => (),
            Ok(_) => return Err(error(78, "review workspace must be an ordinary directory")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                private_directory(&workspace).map_err(io_error)?
            }
            Err(_) => return Err(error(70, "review workspace unavailable")),
        }
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&workspace, fs::Permissions::from_mode(0o700)).map_err(io_error)?;
        let _lock = workspace_lock(&workspace)?;
        validate_workspace_tree(&workspace)?;
        if fs::symlink_metadata(&report).is_ok() {
            return Err(error(78, "report directory must be new"));
        }
        private_directory(&report).map_err(io_error)?;
        let _log = private_file(&report.join("deepsec.log")).map_err(io_error)?;
        write_new(
            &report.join("plan.json"),
            &serde_json::to_vec_pretty(&plan)
                .map_err(|_| error(70, "plan serialisation failed"))?,
        )?;
        let mut ignore = tempfile::NamedTempFile::new_in(&workspace).map_err(io_error)?;
        ignore.write_all(b"*\n").map_err(io_error)?;
        ignore
            .persist(workspace.join(".gitignore"))
            .map_err(|_| error(70, "cannot finalise review ignore file"))?;
        let config = workspace.join("deepsec.config.mjs");
        if fs::symlink_metadata(&config).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
            return Err(error(78, "configuration path is not an ordinary file"));
        }
        let config_value =
            json!({"ai":route,"projects":[{"id":project_id,"root":root}],"dataDir":"./data"});
        let config_text = format!(
            "// Generated by VisionCraft; credential names only.\nexport default {};\n",
            serde_json::to_string_pretty(&config_value)
                .map_err(|_| error(70, "configuration serialisation failed"))?
        );
        let mut staged = tempfile::NamedTempFile::new_in(&workspace).map_err(io_error)?;
        staged.write_all(config_text.as_bytes()).map_err(io_error)?;
        staged.as_file().sync_all().map_err(io_error)?;
        staged
            .persist(&config)
            .map_err(|_| error(70, "cannot finalise review configuration"))?;
        let mut attempt = Attempt {
            policy: &policy,
            report: &report,
            cancelled,
            started: Instant::now(),
            deadline: Duration::from_millis(deadline_ms),
            sequence: 0,
        };
        let mut revision = "unknown".to_owned();
        let mut dirty: Option<bool> = None;
        let mut observed_version: Option<String> = None;
        let outcome = (|| -> Result<Value, GateError> {
            let (rc, version) = attempt.command(
                "version",
                "deepsec",
                vec!["--version".into()],
                &workspace,
                false,
                false,
            )?;
            let version_text = String::from_utf8_lossy(&version)
                .trim()
                .trim_start_matches("deepsec ")
                .to_owned();
            // Store only the bounded version token, never arbitrary subprocess output.
            if version_text.len() <= 64
                && version_text
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".+-_".contains(&b))
            {
                observed_version = Some(version_text.clone());
            }
            if rc != 0 || version_text != supported_version() {
                return Err(error(
                    78,
                    "Deepsec version does not match the verified 2.3.10 protocol",
                ));
            }
            let (rc, bytes) = attempt.command(
                "revision",
                "git",
                vec!["rev-parse".into(), "HEAD".into()],
                &root,
                false,
                false,
            )?;
            if rc == 0 {
                revision = String::from_utf8_lossy(&bytes).trim().to_owned();
            }
            let (rc, bytes) = attempt.command(
                "dirty",
                "git",
                vec!["status".into(), "--porcelain".into()],
                &root,
                false,
                false,
            )?;
            if rc == 0 {
                dirty = Some(!bytes.is_empty());
            }
            let selected = recorded::select(options, files.as_deref(), &mut attempt, &root)?;
            let before = recorded::snapshot(&root, &selected)?;
            write_new(
                &report.join("inputs-before.json"),
                &serde_json::to_vec_pretty(&before)
                    .map_err(|_| error(70, "source encoding failed"))?,
            )?;
            let source_list = report.join("selected-files.txt");
            write_new(
                &source_list,
                format!("{}\n", selected.join("\n")).as_bytes(),
            )?;
            let data = workspace.join("data").join(&project_id);
            let common = vec![
                "--project-id".into(),
                project_id.clone(),
                "--root".into(),
                root.to_string_lossy().into_owned(),
            ];
            let mut model_args = vec![
                "--agent".into(),
                policy.agent.clone(),
                "--thinking-level".into(),
                policy.thinking_level.clone(),
                "--batch-size".into(),
                policy.batch_size.to_string(),
                "--concurrency".into(),
                policy.concurrency.to_string(),
            ];
            if !policy.model.is_empty() {
                model_args.extend(["--model".into(), policy.model.clone()]);
            }
            if options.scan_only || options.full {
                let mut args = vec!["scan".into()];
                args.extend(common.clone());
                let (rc, _) = attempt.command("scan", "deepsec", args, &workspace, false, false)?;
                if rc != 0 {
                    return Err(error(70, "Deepsec scan failed"));
                }
                if options.scan_only {
                    return Ok(
                        json!({"result":"SCANNED","deepsec_exit":0,"findings_by_severity":{},"blocking":[],"boundary":"Scan-only is not a completed model review."}),
                    );
                }
            }
            let expected = if options.full {
                recorded::pending(&data, &before)?
            } else {
                before.clone()
            };
            let prior_runs = recorded::run_files(&data)?;
            let mut args = vec!["process".into()];
            if !options.full {
                args.extend([
                    "--files-from".into(),
                    source_list.to_string_lossy().into_owned(),
                    "--no-ignore".into(),
                ]);
            }
            args.extend(common);
            args.extend(model_args);
            if !options.full {
                args.extend([
                    "--comment-out".into(),
                    report.join("comment.md").to_string_lossy().into_owned(),
                ]);
            }
            let (rc, _) = attempt.command("process", "deepsec", args, &workspace, true, false)?;
            if (options.full && rc != 0) || (!options.full && ![0, 1].contains(&rc)) {
                return Err(error(70, "Deepsec processing failed"));
            }
            if rc == 1
                && read_bounded(&report.join("comment.md"), policy.max_output_bytes)
                    .map_or(true, |b| b.is_empty())
            {
                return Err(error(
                    70,
                    "exit 1 without fresh completed findings evidence",
                ));
            }
            let after = recorded::snapshot(&root, &selected)?;
            write_new(
                &report.join("inputs-after.json"),
                &serde_json::to_vec_pretty(&after)
                    .map_err(|_| error(70, "source encoding failed"))?,
            )?;
            if before != after {
                return Err(error(70, "review inputs changed while the reviewer ran"));
            }
            let observed = recorded::validate_review(
                &data,
                &prior_runs,
                &expected,
                &root,
                &project_id,
                &report,
            )?;
            let args = vec![
                "export".into(),
                "--project-id".into(),
                project_id.clone(),
                "--format".into(),
                "json".into(),
                "--min-severity".into(),
                "LOW".into(),
                "--out".into(),
                report.join("findings.json").to_string_lossy().into_owned(),
            ];
            let (export_rc, _) =
                attempt.command("export", "deepsec", args, &workspace, false, false)?;
            if export_rc != 0 {
                return Err(error(70, "Deepsec findings export failed"));
            }
            let raw = read_bounded(&report.join("findings.json"), policy.max_output_bytes)?;
            let findings = findings(&raw)?;
            if rc == 1 && findings.is_empty() {
                return Err(error(70, "findings exit with an empty export"));
            }
            let mut counts: BTreeMap<&str, usize> =
                SEVERITIES.into_iter().map(|s| (s, 0)).collect();
            let threshold = SEVERITIES
                .iter()
                .position(|s| *s == policy.fail_on)
                .expect("validated threshold");
            let mut blocking = Vec::new();
            for finding in &findings {
                let severity = finding["severity"].as_str().expect("validated severity");
                *counts.get_mut(severity).expect("known severity") += 1;
                if SEVERITIES
                    .iter()
                    .position(|s| *s == severity)
                    .expect("known severity")
                    <= threshold
                {
                    blocking.push(finding.clone());
                }
            }
            Ok(
                json!({"result":if blocking.is_empty(){"PASS"}else{"BLOCK"},"deepsec_exit":rc,"findings_by_severity":counts,"blocking":blocking,"findings_total":findings.len(),"observed_review":observed,"raw_export_sha256":sha256(&raw),"net_new_reported_by_deepsec":if options.full{None}else{Some(rc==1)},"boundary":"Threshold result over the raw export; historical findings may be included. Observed coverage is listed separately; semantic truth is not inferred."}),
            )
        })();
        let (mut receipt, exit) = match outcome {
            Ok(value) => {
                let code = if value["result"] == "BLOCK" { 1 } else { 0 };
                (value, code)
            }
            Err(err) => (
                json!({"result":match err.code {124=>"TIMEOUT",130=>"INTERRUPTED",78=>"UNAVAILABLE",_=>"ERROR"},"error":err.message}),
                err.code,
            ),
        };
        let object = receipt.as_object_mut().expect("receipt object");
        object.extend(json!({"schema_version":1,"gate":"deepsec","attempt_id":attempt_id,"mode":mode,"revision":revision,"dirty":dirty,"deepsec_version":observed_version,"deepsec_version_required":supported_version(),"deepsec_git_head":PROTOCOL["package"]["git_head"],"deepsec_npm_integrity":PROTOCOL["package"]["integrity"],"deepsec_protocol_version":PROTOCOL["adapter_protocol_version"],"release_identity_boundary":"Expected upstream identity; installed package bytes are verified by the host. CLI version was observed separately.","policy_source":policy.policy_source,"fail_on":policy.fail_on,"report_dir":report,"state_dir":workspace,"elapsed_ms":attempt.started.elapsed().as_millis(),"plan_sha256":sha256(&serde_json::to_vec(&plan).map_err(|_|error(70,"plan encoding failed"))?)}).as_object().expect("object").clone());
        write_new(
            &report.join("receipt.json"),
            &serde_json::to_vec_pretty(&receipt)
                .map_err(|_| error(70, "receipt encoding failed"))?,
        )?;
        println!(
            "{}",
            serde_json::to_string_pretty(&receipt)
                .map_err(|_| error(70, "receipt encoding failed"))?
        );
        Ok(exit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duration_and_policy_refuse_unbounded_or_secret_urls() {
        assert_eq!(duration_ms("0.25s"), Some(250));
        assert_eq!(duration_ms("45m"), Some(2_700_000));
        for bad in ["NaNs", "-1s", "0s", "100d", "1.2.3s"] {
            assert_eq!(duration_ms(bad), None);
        }
        let policy = DeepsecPolicy {
            ai_base_url: "https://user:password@example.com".into(),
            ..DeepsecPolicy::default()
        };
        assert!(policy.validate().is_err());
        assert!(DeepsecPolicy::default().validate().is_ok());
    }
    #[test]
    fn export_shape_has_positive_and_negative_controls() {
        assert!(findings(b"[]").unwrap().is_empty());
        assert_eq!(
            findings(br#"[{"filePath":"a","findings":[{"severity":"high"}]}]"#).unwrap()[0]["severity"],
            "HIGH"
        );
        for bad in [
            b"[null]".as_slice(),
            b"{}",
            b"[{\"severity\":\"SUPERHIGH\"}]",
            b"[{\"findings\":[{}]}]",
            b"[",
            br#"[{"severity":"HIGH","severity":"LOW"}]"#,
            br#"{"findings":[{"severity":"HIGH"}],"findings":[]}"#,
        ] {
            assert!(findings(bad).is_err());
        }
    }
}
