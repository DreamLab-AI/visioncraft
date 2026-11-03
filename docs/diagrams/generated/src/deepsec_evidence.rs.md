---
sealmap: 2
source: src/deepsec_evidence.rs
module: "sym:cargo visioncraft . deepsec_evidence/"
language: rust
source_hash: blake3:7bdca6b97bbbee14a261123c7f4f1a363cf86abefbaf059141d6e3f8985a36d2
lines: 317
fragments: 6
---
# `sym:cargo visioncraft . deepsec_evidence/` · src/deepsec_evidence.rs
> Private checks against the pinned reviewer's persisted protocol.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__deepsec_evidence["visioncraft::deepsec_evidence"] {
    <<module>>
    #pending(super) Result#lt;Vec#lt;Artifact#gt;, GateError#gt;
    #run_files(super) Result#lt;BTreeSet#lt;String#gt;, GateError#gt;
    #select(super) Result#lt;Vec#lt;String#gt;, GateError#gt;
    #snapshot(super) Result#lt;Vec#lt;Artifact#gt;, GateError#gt;
    #validate_review(super) Result#lt;Value, GateError#gt;
  }
  class _sealmap_corpus__review__Artifact["sealmap_corpus::review::Artifact"] {
    <<external>>
  }
  class _serde_json__Value["serde_json::Value"] {
    <<external>>
  }
  class _visioncraft__Attempt["visioncraft::Attempt"] {
    <<external>>
  }
  class _visioncraft__DeepsecOptions["visioncraft::DeepsecOptions"] {
    <<external>>
  }
  class _visioncraft__GateError["visioncraft::GateError"] {
    <<external>>
  }
  visioncraft__deepsec_evidence ..> _sealmap_corpus__review__Artifact
  visioncraft__deepsec_evidence ..> _serde_json__Value
  visioncraft__deepsec_evidence ..> _visioncraft__Attempt
  visioncraft__deepsec_evidence ..> _visioncraft__DeepsecOptions
  visioncraft__deepsec_evidence ..> _visioncraft__GateError
```

## `sym:cargo visioncraft . deepsec_evidence/confined().`
`fn confined(root: &Path, relative: &str) -> Result<PathBuf, GateError>` · L15-L37
```mermaid
sequenceDiagram
  participant visioncraft__deepsec_evidence as deepsec_evidence mod
  participant visioncraft as visioncraft mod
  participant _visioncraft as visioncraft ext
  visioncraft__deepsec_evidence->>visioncraft: crate::relative_path(relative)
  opt !crate::relative_path(relative) || relative.trim() !…
    visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;source path …)
    Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;source path cannot be represented…
  end
  loop for part in Path::new(relative).components()
    opt via map_err
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review sourc…)
    end
    opt fs::symlink_metadata(&path).map_err(| _ | error(70, …
      visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;review sourc…)
      Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;review source must not traverse s…
    end
  end
```

## `sym:cargo visioncraft . deepsec_evidence/snapshot().`
`pub(super) fn snapshot(root: &Path, paths: &[String]) -> Result<Vec<Artifact>, GateError>` · L38-L60
```mermaid
sequenceDiagram
  participant visioncraft__deepsec_evidence as deepsec_evidence mod
  participant _visioncraft as visioncraft ext
  participant visioncraft as visioncraft mod
  opt paths.len()> 100_000
    visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;review file …)
    Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;review file count exceeds 100000#quot;…
  end
  opt via map
    visioncraft__deepsec_evidence->>visioncraft__deepsec_evidence: confined(root, name)?
    visioncraft__deepsec_evidence->>_visioncraft: ~read_bounded(&_, _)?
    opt total> 256 * 1024 * 1024
      visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;review sourc…)
      Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;review source exceeds 256 MiB#59; ch…
    end
    visioncraft__deepsec_evidence->>visioncraft: sha256(&bytes)
  end
```

## `sym:cargo visioncraft . deepsec_evidence/select().`
`pub(super) fn select(options: &DeepsecOptions, files: Option<&Path>, attempt: &mut Attempt<'_>, root: &Path,) -> Result<Vec<String>, GateError>` · L61-L169
```mermaid
sequenceDiagram
  participant visioncraft__deepsec_evidence as deepsec_evidence mod
  participant _visioncraft as visioncraft ext
  alt let Some(files) = files
    alt files == Path::new(#quot;-#quot;)
      opt rc != 0
        visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;could not re…)
        Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;could not read file list#quot;))
      end
    else
      visioncraft__deepsec_evidence->>_visioncraft: ~read_bounded(files, _)?
    end
  else
    opt let Some(reference) = &options.diff
      opt reference.starts_with('-')
        visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;diff referen…)
        Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;diff reference must not be a Git …
      end
    end
    opt rc != 0
      visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;could not re…)
      Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;could not resolve review source s…
    end
    opt options.diff_working
      opt rc != 0
        visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;could not re…)
        Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;could not resolve untracked selec…
      end
    end
  end
  opt via map_err
    visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;file list mu…)
  end
  opt paths.is_empty()
    visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;no review in…)
    Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;no review inputs#59; this is not a c…
  end
  loop for path in &paths
    visioncraft__deepsec_evidence->>visioncraft__deepsec_evidence: confined(root, path)?
  end
  opt options.diff_staged
    opt via map_err
      visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;Git paths mu…)
    end
    opt rc != 0 || changed.split('\0').any(| p | paths.conta…
      visioncraft__deepsec_evidence->>_visioncraft: ~error(78, #quot;staged input…)
      Note over visioncraft__deepsec_evidence: return Err(error(78, #quot;staged inputs differ from working…
    end
  end
```

## `sym:cargo visioncraft . deepsec_evidence/run_files().`
`pub(super) fn run_files(data: &Path) -> Result<BTreeSet<String>, GateError>` · L170-L193
```mermaid
sequenceDiagram
  participant visioncraft__deepsec_evidence as deepsec_evidence mod
  participant _visioncraft as visioncraft ext
  opt !dir.exists()
    Note over visioncraft__deepsec_evidence: return Ok(BTreeSet::new())
  end
  opt via map_err
    visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review run d…)
  end
  opt !meta.is_dir() || meta.file_type().is_symlink()
    visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;invalid revi…)
    Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;invalid review run directory#quot;))
  end
  opt via map_err
    visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review runs …)
  end
  loop for item in fs::read_dir(dir).map_err(| _ | error(70, #quot;…
    opt via map_err
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review run e…)
    end
    opt set.len()>= 100_000
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review histo…)
      Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;review history exceeds limit#quot;))
    end
  end
```

## `sym:cargo visioncraft . deepsec_evidence/validate_review().`
`pub(super) fn validate_review(data: &Path, prior: &BTreeSet<String>, expected: &[Artifact], root: &Path, project: &str, report: &Path,) -> Result<Value, GateError>` · L220-L317
```mermaid
sequenceDiagram
  participant visioncraft__deepsec_evidence as deepsec_evidence mod
  participant _sealmap_corpus as sealmap_corpus ext
  participant _serde_json as serde_json ext
  participant _visioncraft as visioncraft ext
  participant visioncraft as visioncraft mod
  visioncraft__deepsec_evidence->>visioncraft__deepsec_evidence: run_files(data)?
  loop for name in run_files(data)?.difference(prior)
    visioncraft__deepsec_evidence->>visioncraft__deepsec_evidence: object(&join())?
  end
  visioncraft__deepsec_evidence->>_sealmap_corpus: Artifact::iter()
  opt via map
    visioncraft__deepsec_evidence->>visioncraft__deepsec_evidence: file_record(data, &artifact.path)
  end
  visioncraft__deepsec_evidence->>_serde_json: serde_json::to_vec_pretty(&evidence)
  opt via map_err
    visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review evide…)
  end
  visioncraft__deepsec_evidence->>_visioncraft: ~write_new(&join(), &bytes)?
  opt runs.len() != 1
    visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;missing or a…)
    Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;missing or ambiguous fresh proces…
  end
  loop each via filter
    visioncraft__deepsec_evidence->>visioncraft: crate::identifier(s)
  end
  opt via ok_or_else
    visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;invalid proc…)
  end
  visioncraft__deepsec_evidence->>_sealmap_corpus: Artifact::len()
  opt run [#quot;phase#quot;] != #quot;done#quot; || run [#quot;projectId#quot;] != proj…
    visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;processing r…)
    Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;processing run is incomplete or c…
  end
  loop for artifact in expected
    opt record [#quot;filePath#quot;] != artifact.path || record [#quot;pro…
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review file …)
      Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;review file identity, content or …
    end
    opt via ok_or_else
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;missing per-…)
    end
    opt entries.len() != 1
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;missing or a…)
      Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;missing or ambiguous analysis for…
    end
    opt entry [#quot;refusal#quot;] [#quot;refused#quot;].as_bool() != Some(fals…
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review was r…)
      Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;review was refused, partly skippe…
    end
    opt entry [#quot;usage#quot;] [#quot;outputTokens#quot;].as_u64().is_none_or…
      visioncraft__deepsec_evidence->>_visioncraft: ~error(70, #quot;review lacks…)
      Note over visioncraft__deepsec_evidence: return Err(error(70, #quot;review lacks actual model identit…
    end
  end
```
