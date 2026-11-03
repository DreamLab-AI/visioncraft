//! Shared policy for read-only Git metadata inspection, not user build commands.
use std::collections::BTreeMap;

/// Unfiltered tracked-file identity. Modes participate in index/tree parity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub mode: String,
    pub object: String,
}

/// List raw tree entries, optionally confined to explicitly selected paths.
pub(crate) fn tree_arguments(reference: &str, paths: &[String]) -> Vec<String> {
    let mut args = vec![
        "ls-tree".into(),
        "-r".into(),
        "-z".into(),
        "--full-tree".into(),
        reference.into(),
        "--".into(),
    ];
    args.extend(paths.iter().cloned());
    args
}

/// Parse `ls-tree -z` without consulting attributes or invoking conversions.
pub(crate) fn tree_entries(bytes: &[u8]) -> Result<BTreeMap<String, Entry>, &'static str> {
    let mut entries = BTreeMap::new();
    for row in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let at = row
            .iter()
            .position(|b| *b == b'\t')
            .ok_or("invalid Git tree record")?;
        let meta = std::str::from_utf8(&row[..at]).map_err(|_| "invalid Git tree metadata")?;
        let fields: Vec<_> = meta.split_whitespace().collect();
        if fields.len() != 3 {
            return Err("invalid Git tree fields");
        }
        if fields[1] != "blob" {
            continue;
        }
        let path = std::str::from_utf8(&row[at + 1..]).map_err(|_| "Git path is not UTF-8")?;
        if !crate::hex(fields[2], &[40, 64])
            || entries
                .insert(
                    path.to_owned(),
                    Entry {
                        mode: fields[0].to_owned(),
                        object: fields[2].to_owned(),
                    },
                )
                .is_some()
        {
            return Err("invalid or duplicate Git tree object");
        }
    }
    Ok(entries)
}

/// Parse `ls-files --stage -z`; unresolved index stages cannot attest parity.
pub(crate) fn index_entries(bytes: &[u8]) -> Result<BTreeMap<String, Entry>, &'static str> {
    let mut entries = BTreeMap::new();
    for row in bytes.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        let at = row
            .iter()
            .position(|b| *b == b'\t')
            .ok_or("invalid Git index record")?;
        let meta = std::str::from_utf8(&row[..at]).map_err(|_| "invalid Git index metadata")?;
        let fields: Vec<_> = meta.split_whitespace().collect();
        if fields.len() != 3 || fields[2] != "0" || !crate::hex(fields[1], &[40, 64]) {
            return Err("invalid or unresolved Git index entry");
        }
        if fields[0] == "160000" {
            return Err("submodule index entries require an explicit file manifest");
        }
        let path = std::str::from_utf8(&row[at + 1..]).map_err(|_| "Git path is not UTF-8")?;
        if entries
            .insert(
                path.to_owned(),
                Entry {
                    mode: fields[0].to_owned(),
                    object: fields[1].to_owned(),
                },
            )
            .is_some()
        {
            return Err("duplicate Git index entry");
        }
    }
    Ok(entries)
}

/// Explicitly bypass all attribute filters; never write a Git object.
pub(crate) fn raw_hash_arguments(paths: &[String]) -> Vec<String> {
    let mut args = vec!["hash-object".into(), "--no-filters".into(), "--".into()];
    args.extend(paths.iter().cloned());
    args
}

/// Bind ordered raw hashes to the paths passed to `hash-object`.
pub(crate) fn raw_hashes(
    paths: &[String],
    bytes: &[u8],
) -> Result<BTreeMap<String, String>, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Git object hashes are not UTF-8")?;
    let hashes: Vec<_> = text.lines().collect();
    if hashes.len() != paths.len() || hashes.iter().any(|h| !crate::hex(h, &[40, 64])) {
        return Err("Git raw hash count or identity differs");
    }
    Ok(paths
        .iter()
        .cloned()
        .zip(hashes.into_iter().map(str::to_owned))
        .collect())
}

pub(crate) fn prefix() -> Vec<String> {
    [
        "--no-pager",
        "--literal-pathspecs",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.untrackedCache=false",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub(crate) fn fixed_environment() -> BTreeMap<String, String> {
    [
        ("GIT_OPTIONAL_LOCKS", "0"),
        ("GIT_NO_LAZY_FETCH", "1"),
        ("GIT_NO_REPLACE_OBJECTS", "1"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ]
    .into_iter()
    .map(|(name, value)| (name.into(), value.into()))
    .collect()
}
