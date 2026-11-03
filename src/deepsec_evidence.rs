//! Private checks against the pinned reviewer's persisted protocol.
use super::{Attempt, DeepsecOptions, GateError, StrictJson, error, read_bounded, write_new};
use crate::{Artifact, sha256};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn object(path: &Path) -> Result<Value, GateError> {
    let bytes = read_bounded(path, 16 * 1024 * 1024)?;
    serde_json::from_slice::<StrictJson>(&bytes)
        .map(|v| v.0)
        .map_err(|_| error(70, "malformed reviewer state"))
}
fn confined(root: &Path, relative: &str) -> Result<PathBuf, GateError> {
    if !crate::relative_path(relative)
        || relative.trim() != relative
        || relative.contains(['\n', '\r', '\\'])
    {
        return Err(error(
            78,
            "source path cannot be represented by the review protocol",
        ));
    }
    let mut path = root.to_owned();
    for part in Path::new(relative).components() {
        path.push(part);
        if fs::symlink_metadata(&path)
            .map_err(|_| error(70, "review source disappeared"))?
            .file_type()
            .is_symlink()
        {
            return Err(error(78, "review source must not traverse symlinks"));
        }
    }
    Ok(path)
}
pub(super) fn snapshot(root: &Path, paths: &[String]) -> Result<Vec<Artifact>, GateError> {
    if paths.len() > 100_000 {
        return Err(error(78, "review file count exceeds 100000"));
    }
    let mut total = 0_usize;
    paths
        .iter()
        .map(|name| {
            let bytes = read_bounded(&confined(root, name)?, 32 * 1024 * 1024)?;
            total = total.saturating_add(bytes.len());
            if total > 256 * 1024 * 1024 {
                return Err(error(
                    78,
                    "review source exceeds 256 MiB; choose a narrower scope",
                ));
            }
            Ok(Artifact {
                path: name.clone(),
                sha256: sha256(&bytes),
            })
        })
        .collect()
}
fn git_bytes(
    attempt: &mut Attempt<'_>,
    root: &Path,
    stage: &str,
    args: Vec<String>,
) -> Result<Vec<u8>, GateError> {
    let (rc, bytes) = attempt.command(stage, "git", args, root, false, false)?;
    if rc != 0 {
        return Err(error(78, "could not inspect read-only Git metadata"));
    }
    Ok(bytes)
}

fn tree(
    attempt: &mut Attempt<'_>,
    root: &Path,
    reference: &str,
) -> Result<BTreeMap<String, crate::git_metadata::Entry>, GateError> {
    if reference.starts_with('-') {
        return Err(error(78, "diff reference must not be a Git option"));
    }
    let bytes = git_bytes(
        attempt,
        root,
        "reference-tree",
        crate::git_metadata::tree_arguments(reference, &[]),
    )?;
    crate::git_metadata::tree_entries(&bytes).map_err(|message| error(78, message))
}

fn index(
    attempt: &mut Attempt<'_>,
    root: &Path,
) -> Result<BTreeMap<String, crate::git_metadata::Entry>, GateError> {
    let bytes = git_bytes(
        attempt,
        root,
        "index-entries",
        vec![
            "ls-files".into(),
            "--stage".into(),
            "-z".into(),
            "--".into(),
        ],
    )?;
    crate::git_metadata::index_entries(&bytes).map_err(|message| error(78, message))
}

fn working_hashes(
    attempt: &mut Attempt<'_>,
    root: &Path,
    paths: &[String],
) -> Result<BTreeMap<String, String>, GateError> {
    if paths.len() > 100_000 {
        return Err(error(78, "review file count exceeds 100000"));
    }
    let mut hashes = BTreeMap::new();
    for chunk in paths.chunks(64) {
        let mut present = Vec::new();
        for name in chunk {
            if !root
                .join(name)
                .try_exists()
                .map_err(|_| error(70, "source existence is unavailable"))?
            {
                continue;
            }
            let path = confined(root, name)?;
            let meta = fs::metadata(&path).map_err(|_| error(70, "source metadata unavailable"))?;
            if !meta.is_file() || meta.len() > 32 * 1024 * 1024 {
                return Err(error(
                    78,
                    "raw review input must be an ordinary file at most 32 MiB",
                ));
            }
            present.push(name.clone());
        }
        if present.is_empty() {
            continue;
        }
        let bytes = git_bytes(
            attempt,
            root,
            "raw-working-hashes",
            crate::git_metadata::raw_hash_arguments(&present),
        )?;
        hashes.extend(
            crate::git_metadata::raw_hashes(&present, &bytes)
                .map_err(|message| error(78, message))?,
        );
    }
    Ok(hashes)
}

/// Refuse repository-controlled reviewer state before any external tool is started.
pub(super) fn refuse_tracked_workspace(
    attempt: &mut Attempt<'_>,
    root: &Path,
) -> Result<(), GateError> {
    let bytes = git_bytes(
        attempt,
        root,
        "tracked-review-workspace",
        vec![
            "ls-files".into(),
            "--cached".into(),
            "-z".into(),
            "--".into(),
            ".deepsec-gate".into(),
        ],
    )?;
    if !bytes.is_empty() {
        return Err(error(78, "tracked .deepsec-gate reviewer state is refused"));
    }
    Ok(())
}

/// Observe only the selected raw input bytes against their committed Git blobs.
pub(super) fn selected_dirty(
    attempt: &mut Attempt<'_>,
    root: &Path,
    paths: &[String],
) -> Result<bool, GateError> {
    let mut reference = BTreeMap::new();
    for chunk in paths.chunks(64) {
        let bytes = git_bytes(
            attempt,
            root,
            "selected-reference-tree",
            crate::git_metadata::tree_arguments("HEAD", chunk),
        )?;
        reference.extend(
            crate::git_metadata::tree_entries(&bytes).map_err(|message| error(78, message))?,
        );
    }
    let current = working_hashes(attempt, root, paths)?;
    Ok(paths
        .iter()
        .any(|p| reference.get(p).map(|e| &e.object) != current.get(p)))
}

pub(super) fn select(
    options: &DeepsecOptions,
    files: Option<&Path>,
    attempt: &mut Attempt<'_>,
    root: &Path,
) -> Result<Vec<String>, GateError> {
    let raw = if let Some(files) = files {
        if files == Path::new("-") {
            let (rc, bytes) = attempt.command("input-files", "cat", vec![], root, false, true)?;
            if rc != 0 {
                return Err(error(78, "could not read file list"));
            }
            bytes
        } else {
            read_bounded(files, 8 * 1024 * 1024)?
        }
    } else {
        let args: Vec<String> = if options.full || options.scan_only {
            vec![
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        } else {
            Vec::new()
        };
        let mut bytes = if !args.is_empty() {
            let mut args = args;
            args.push("--".into());
            git_bytes(attempt, root, "source-selection", args)?
        } else {
            let base = tree(attempt, root, options.diff.as_deref().unwrap_or("HEAD"))?;
            let indexed = index(attempt, root)?;
            let selected: Vec<String> = if options.diff_staged {
                indexed
                    .iter()
                    .filter(|(p, e)| base.get(*p) != Some(*e))
                    .map(|(p, _)| p.clone())
                    .collect()
            } else {
                let paths: Vec<_> = indexed.keys().cloned().collect();
                working_hashes(attempt, root, &paths)?
                    .into_iter()
                    .filter(|(p, hash)| base.get(p).map(|e| &e.object) != Some(hash))
                    .map(|(p, _)| p)
                    .collect()
            };
            let mut out = Vec::new();
            for path in selected {
                out.extend(path.as_bytes());
                out.push(0);
            }
            out
        };
        if options.diff_working {
            let (rc, more) = attempt.command(
                "untracked-selection",
                "git",
                vec!["ls-files", "-z", "--others", "--exclude-standard", "--"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                root,
                false,
                false,
            )?;
            if rc != 0 {
                return Err(error(78, "could not resolve untracked selection"));
            }
            bytes.extend(more);
        }
        bytes
    };
    let text = String::from_utf8(raw).map_err(|_| error(78, "file list must be UTF-8"))?;
    let separator = if files.is_some() { '\n' } else { '\0' };
    let paths: BTreeSet<String> = text
        .split(separator)
        .filter(|s| !s.is_empty() && *s != ".deepsec-gate" && !s.starts_with(".deepsec-gate/"))
        .map(str::to_owned)
        .collect();
    if paths.is_empty() {
        return Err(error(
            78,
            "no review inputs; this is not a completed review",
        ));
    }
    for path in &paths {
        confined(root, path)?;
    }
    if options.diff_staged {
        let indexed = index(attempt, root)?;
        let selected: Vec<_> = paths.iter().cloned().collect();
        let working = working_hashes(attempt, root, &selected)?;
        if paths
            .iter()
            .any(|p| indexed.get(p).map(|e| &e.object) != working.get(p))
        {
            return Err(error(
                78,
                "staged inputs differ from working-tree bytes; review an exact index checkout",
            ));
        }
    }
    Ok(paths.into_iter().collect())
}
pub(super) fn run_files(data: &Path) -> Result<BTreeSet<String>, GateError> {
    let dir = data.join("runs");
    if !dir.exists() {
        return Ok(BTreeSet::new());
    }
    let meta =
        fs::symlink_metadata(&dir).map_err(|_| error(70, "review run directory unavailable"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(error(70, "invalid review run directory"));
    }
    let mut set = BTreeSet::new();
    for item in fs::read_dir(dir).map_err(|_| error(70, "review runs cannot be read"))? {
        let item = item.map_err(|_| error(70, "review run entry unavailable"))?;
        if set.len() >= 100_000 {
            return Err(error(70, "review history exceeds limit"));
        }
        if let Some(name) = item.file_name().to_str()
            && name.ends_with(".json")
        {
            set.insert(name.to_owned());
        }
    }
    Ok(set)
}
fn file_record(data: &Path, name: &str) -> Result<Value, GateError> {
    object(&confined(data, &format!("files/{name}.json"))?)
}
pub(super) fn pending(data: &Path, selected: &[Artifact]) -> Result<Vec<Artifact>, GateError> {
    let mut out = Vec::new();
    for artifact in selected {
        let path = data.join("files").join(format!("{}.json", artifact.path));
        if !path.exists() {
            continue;
        }
        let value = file_record(data, &artifact.path)?;
        if matches!(
            value["status"].as_str(),
            Some("pending" | "error" | "processing")
        ) {
            out.push(artifact.clone());
        }
    }
    if out.is_empty() {
        return Err(error(
            78,
            "scan has no pending candidates; no new review evidence produced",
        ));
    }
    Ok(out)
}
pub(super) fn validate_review(
    data: &Path,
    prior: &BTreeSet<String>,
    expected: &[Artifact],
    root: &Path,
    project: &str,
    report: &Path,
) -> Result<Value, GateError> {
    let mut runs = Vec::new();
    for name in run_files(data)?.difference(prior) {
        let value = object(&data.join("runs").join(name))?;
        if value["type"] == "process" {
            runs.push(value);
        }
    }
    let records: BTreeMap<_, _> = expected
        .iter()
        .map(|artifact| {
            (
                artifact.path.clone(),
                file_record(data, &artifact.path).unwrap_or(Value::Null),
            )
        })
        .collect();
    // Preserve rejected state as well as successful observations. The decoder's
    // raw state remains private; a null entry means its file could not be read.
    let evidence = json!({"runs":runs,"files":records});
    let bytes = serde_json::to_vec_pretty(&evidence)
        .map_err(|_| error(70, "review evidence encoding failed"))?;
    write_new(&report.join("review-evidence.json"), &bytes)?;
    if runs.len() != 1 {
        return Err(error(70, "missing or ambiguous fresh processing run"));
    }
    let run = &runs[0];
    let run_id = run["runId"]
        .as_str()
        .filter(|s| crate::identifier(s))
        .ok_or_else(|| error(70, "invalid processing run identity"))?;
    if run["phase"] != "done"
        || run["projectId"] != project
        || run["rootPath"].as_str() != root.to_str()
        || !run["completedAt"].is_string()
        || run["stats"]["filesProcessed"].as_u64() != Some(expected.len() as u64)
    {
        return Err(error(
            70,
            "processing run is incomplete or covers another scope",
        ));
    }
    for artifact in expected {
        let record = &records[&artifact.path];
        if record["filePath"] != artifact.path
            || record["projectId"] != project
            || record["fileHash"] != artifact.sha256
            || record["status"] != "analyzed"
        {
            return Err(error(
                70,
                "review file identity, content or completion mismatch",
            ));
        }
        let entries = record["analysisHistory"]
            .as_array()
            .ok_or_else(|| error(70, "missing per-file analysis history"))?;
        let entries: Vec<_> = entries.iter().filter(|e| e["runId"] == run_id).collect();
        if entries.len() != 1 {
            return Err(error(
                70,
                "missing or ambiguous analysis for the current run",
            ));
        }
        let entry = entries[0];
        if entry["refusal"]["refused"].as_bool() != Some(false)
            || entry["refusal"]["skipped"]
                .as_array()
                .is_none_or(|s| !s.is_empty())
        {
            return Err(error(
                70,
                "review was refused, partly skipped, or refusal status is unknown",
            ));
        }
        if entry["usage"]["outputTokens"]
            .as_u64()
            .is_none_or(|n| n == 0)
            || entry["model"].as_str().is_none_or(|s| s.is_empty())
            || entry["agentType"].as_str().is_none_or(|s| s.is_empty())
        {
            return Err(error(
                70,
                "review lacks actual model identity or output evidence",
            ));
        }
    }
    Ok(
        json!({"run_id":run_id,"observed_files":expected.iter().map(|a|&a.path).collect::<Vec<_>>(),"evidence_sha256":sha256(&bytes),"coverage_boundary":"Recorded per-file investigations of exact selected bytes; dependencies and runtime behaviour remain outside this observation."}),
    )
}
