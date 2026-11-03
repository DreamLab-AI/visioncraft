---
sealmap: 2
source: src/deepsec.rs
module: "sym:cargo visioncraft . deepsec/"
language: rust
source_hash: blake3:fe5f7e3e8985a7eeb253589267c66d64771af816b56d031e6022a81c9b5649ae
lines: 1013
fragments: 5
---
# `sym:cargo visioncraft . deepsec/` · src/deepsec.rs
> Executed Deepsec 2.3.10 gates with explicit policy and retained error states.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__deepsec___tDeepsecOptions["DeepsecOptions"] {
    <<struct>>
    +diff: Option#lt;String#gt;
    +diff_working: bool
    +diff_staged: bool
    +files_from: Option#lt;PathBuf#gt;
    +full: bool
    +scan_only: bool
    +root: Option#lt;PathBuf#gt;
    +report_dir: Option#lt;PathBuf#gt;
    +policy_json: Option#lt;String#gt;
    +fail_on: Option#lt;String#gt;
    +thinking_level: Option#lt;String#gt;
    +agent: Option#lt;String#gt;
    +model: Option#lt;String#gt;
    +model_auth: Option#lt;String#gt;
    +dry_run: bool
  }
  class visioncraft__deepsec___tDeepsecPolicy["DeepsecPolicy"] {
    <<struct>>
    +enabled: bool
    +agent: String
    +model_auth: String
    +model: String
    +thinking_level: String
    +fail_on: String
    +max_duration: String
    +batch_size: u32
    +concurrency: u32
    +ai_provider: String
    +ai_api_key_env: String
    +ai_base_url: String
    +policy_source: String
    +max_output_bytes: u64
    +Default::default() Self
    +validate(&self) Result#lt;u64, GateError#gt;
  }
  class visioncraft__deepsec___tGateError["GateError"] {
    <<struct>>
    +code: i32
    +message: String
  }
  class visioncraft__deepsec___tStrictJson["StrictJson"] {
    <<struct>>
    -0: Value
    +Deserialize#lt;'de#gt;::deserialize(deserializer: D) Result#lt;Self, D::Error#gt;
  }
  class visioncraft__deepsec["visioncraft::deepsec"] {
    <<module>>
    +execute(options: &DeepsecOptions, cancelled: &AtomicBool) Result#lt;i32, GateError#gt;
  }
  class _serde__Deserialize["serde::Deserialize"] {
    <<external>>
  }
  visioncraft__deepsec ..> visioncraft__deepsec___tDeepsecOptions
  visioncraft__deepsec ..> visioncraft__deepsec___tGateError
  visioncraft__deepsec___tDeepsecPolicy ..> visioncraft__deepsec___tGateError
  visioncraft__deepsec___tStrictJson ..|> _serde__Deserialize
```

## `sym:cargo visioncraft . deepsec/DeepsecPolicy#validate().`
`pub fn validate(&self) -> Result<u64, GateError>` · L142-L229
> Validate policy and return the whole-attempt deadline in milliseconds.
```mermaid
sequenceDiagram
  participant visioncraft__deepsec___tDeepsecPolicy as DeepsecPolicy
  participant visioncraft__deepsec as deepsec mod
  participant _url as url ext
  opt ![#quot;claude#quot;, #quot;codex#quot;, #quot;pi#quot;].contains(&self.agent.as_s…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;unsupported …)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;unsupported Deepsec agent#quot;))
  end
  opt ![#quot;local#quot;, #quot;direct#quot;, #quot;custom#quot;].contains(&self.model_…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;unsupported …)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;unsupported model authentication …
  end
  opt ![#quot;minimal#quot;, #quot;low#quot;, #quot;medium#quot;, #quot;high#quot;, #quot;xhigh#quot;].conta…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;unsupported …)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;unsupported thinking level#quot;))
  end
  opt !SEVERITIES.contains(&self.fail_on.as_str())
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;unsupported …)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;unsupported severity threshold#quot;))
  end
  opt !(1..= 100).contains(&self.batch_size) ||!(1..= 32).…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;invalid batc…)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;invalid batch, concurrency or out…
  end
  opt [&self.model, &self.policy_source, &self.ai_provider…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;invalid poli…)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;invalid policy string#quot;))
  end
  opt !self.ai_api_key_env.is_empty() && !(self.ai_api_key…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;invalid cred…)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;invalid credential environment na…
  end
  opt !self.ai_base_url.is_empty()
    visioncraft__deepsec___tDeepsecPolicy->>_url: Url::parse(&self.ai_base_url)
    opt via map_err
      visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;invalid mode…)
    end
    opt ![#quot;http#quot;, #quot;https#quot;].contains(&url.scheme()) || url.ho…
      visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;base URL mus…)
      Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;base URL must be HTTP(S), without…
    end
  end
  alt #quot;local#quot; if self.agent == #quot;pi#quot;
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;local authen…)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;local authentication requires cla…
  else #quot;direct#quot; if ![#quot;anthropic#quot;, #quot;openai#quot;].contains(&self.ai_…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;direct authe…)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;direct authentication requires a …
  else #quot;custom#quot; if self.agent != #quot;pi#quot; || self.ai_api_key_env.i…
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;custom authe…)
    Note over visioncraft__deepsec___tDeepsecPolicy: return Err(error(78, #quot;custom authentication requires pi…
  end
  visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: duration_ms(&self.max_duration)
  opt via ok_or_else
    visioncraft__deepsec___tDeepsecPolicy->>visioncraft__deepsec: error(78, #quot;invalid dura…)
  end
```

## `sym:cargo visioncraft . deepsec/findings().`
`fn findings(bytes: &[u8]) -> Result<Vec<Value>, GateError>` · L371-L410
```mermaid
sequenceDiagram
  participant visioncraft__deepsec as deepsec mod
  participant _serde_json as serde_json ext
  visioncraft__deepsec->>_serde_json: serde_json::from_slice(bytes)
  opt via map_err
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;malformed or…)
  end
  opt via ok_or_else
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;unrecognised…)
  end
  loop for row in rows
    alt if let Some(nested) = row.get(#quot;findings#quot;).and_then(Valu…
      loop for finding in nested
        opt via ok_or_else
          visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;finding must…)
        end
      end
    else
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;unrecognised…)
      Note over visioncraft__deepsec: return Err(error(70, #quot;unrecognised finding row#quot;))
    end
  end
  loop for finding in &mut output
    opt via ok_or_else
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;finding lack…)
    end
    opt !SEVERITIES.contains(&severity.as_str())
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;finding has …)
      Note over visioncraft__deepsec: return Err(error(70, #quot;finding has an unrecognised sever…
    end
  end
```

## `sym:cargo visioncraft . deepsec/Attempt#command().`
`fn command(&mut self, stage: &str, program: &str, args: Vec<String>, cwd: &Path, model: bool, stdin: bool,) -> Result<(i32, Vec<u8>), GateError>` · L484-L548
```mermaid
sequenceDiagram
  participant visioncraft__deepsec___tAttempt as Attempt
  participant visioncraft__deepsec as deepsec mod
  participant visioncraft__process as process mod
  participant _serde_json as serde_json ext
  opt via ok_or_else
    visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(124, #quot;whole-attemp…)
  end
  visioncraft__deepsec___tAttempt->>visioncraft__process: private_directory(&directory)
  visioncraft__deepsec___tAttempt->>visioncraft__deepsec: environment(self.policy, model)
  visioncraft__deepsec___tAttempt->>_serde_json: serde_json::to_vec_pretty(&invocation)
  opt via map_err
    visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(70, #quot;invocation s…)
  end
  visioncraft__deepsec___tAttempt->>visioncraft__deepsec: write_new(&join(), &_)?
  visioncraft__deepsec___tAttempt->>visioncraft__process: run_command(&invocation, cwd, &directory, self.cancelle…
  visioncraft__deepsec___tAttempt->>visioncraft__deepsec: read_bounded(&run.stdout, self.policy.max_output_bytes)?
  visioncraft__deepsec___tAttempt->>visioncraft__deepsec: read_bounded(&run.stderr, self.policy.max_output_bytes)?
  visioncraft__deepsec___tAttempt->>_serde_json: serde_json::to_vec_pretty(&execution)
  opt via map_err
    visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(70, #quot;execution se…)
  end
  visioncraft__deepsec___tAttempt->>visioncraft__deepsec: write_new(&join(), &_)?
  alt Termination::Completed if run.signal.is_none()
    opt via ok_or_else
      visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(70, #quot;missing exit…)
    end
  else Termination::Unavailable
    visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(78, #quot;required exe…)
  else Termination::TimedOut
    visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(124, #quot;whole-attemp…)
  else Termination::Interrupted
    visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(130, #quot;review inter…)
  else _
    visioncraft__deepsec___tAttempt->>visioncraft__deepsec: error(70, #quot;review proce…)
  end
```

## `sym:cargo visioncraft . deepsec/execute().`
`pub fn execute(options: &DeepsecOptions, cancelled: &AtomicBool) -> Result<i32, GateError>` · L571-L975
> Execute an explicitly requested security gate, returning its legacy exit code.
```mermaid
sequenceDiagram
  participant visioncraft__deepsec as deepsec mod
  participant visioncraft as visioncraft mod
  participant visioncraft__deepsec___tDeepsecPolicy as DeepsecPolicy
  participant _serde_json as serde_json ext
  participant visioncraft__process as process mod
  participant _tempfile as tempfile ext
  participant visioncraft__deepsec___tAttempt as Attempt
  participant _recorded as recorded ext
  opt modes.into_iter().filter(| x | * x).count() != 1 || …
    visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;choose exact…)
    Note over visioncraft__deepsec: return Err(error(78, #quot;choose exactly one nonempty sourc…
  end
  visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;Deepsec proc…)
  alt Some(text)
    visioncraft__deepsec->>visioncraft: from_json(as_bytes())
    opt via map_err
      visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;invalid poli…)
    end
  else None
    visioncraft__deepsec->>visioncraft__deepsec___tDeepsecPolicy: DeepsecPolicy::default()
  end
  opt !policy.enabled
    visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;Deepsec gate…)
    Note over visioncraft__deepsec: return Err(error(78, #quot;Deepsec gate disabled by policy#59; …
  end
  visioncraft__deepsec->>visioncraft__deepsec: root_for(options)?
  opt Some(path)
    opt via map_err
      visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;file manifes…)
    end
    visioncraft__deepsec->>visioncraft__deepsec: read_bounded(&path, _)
    opt via map_err
      visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;file manifes…)
    end
  end
  opt options.dry_run
    visioncraft__deepsec->>_serde_json: serde_json::to_string_pretty(&plan)
    opt via map_err
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;plan seriali…)
    end
    Note over visioncraft__deepsec: return Ok(0)
  end
  opt !options.scan_only && policy.model_auth != #quot;local#quot; &…
    visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;configured c…)
    Note over visioncraft__deepsec: return Err(error(78, #quot;configured credential environment…
  end
  alt Ok(_)
    visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;review works…)
    Note over visioncraft__deepsec: return Err(error(78, #quot;review workspace must be an ordin…
  else Err(e) if e.kind() == std::io::ErrorKind::NotFound
    visioncraft__deepsec->>visioncraft__process: private_directory(&workspace)
  else Err(_)
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;review works…)
    Note over visioncraft__deepsec: return Err(error(70, #quot;review workspace unavailable#quot;))
  end
  visioncraft__deepsec->>visioncraft__deepsec: workspace_lock(&workspace)?
  opt fs::symlink_metadata(&report).is_ok()
    visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;report direc…)
    Note over visioncraft__deepsec: return Err(error(78, #quot;report directory must be new#quot;))
  end
  visioncraft__deepsec->>visioncraft__process: private_directory(&report)
  visioncraft__deepsec->>visioncraft__process: private_file(&join())
  visioncraft__deepsec->>_serde_json: serde_json::to_vec_pretty(&plan)
  opt via map_err
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;plan seriali…)
  end
  visioncraft__deepsec->>visioncraft__deepsec: write_new(&join(), &_)?
  opt !workspace.join(#quot;.gitignore#quot;).exists()
    visioncraft__deepsec->>visioncraft__deepsec: write_new(&join(), b#quot;*\n#quot;)?
  end
  opt fs::symlink_metadata(&config).is_ok_and(| m |!m.is_f…
    visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;configuratio…)
    Note over visioncraft__deepsec: return Err(error(78, #quot;configuration path is not an ordi…
  end
  visioncraft__deepsec->>_serde_json: serde_json::to_string_pretty(&config_value)
  opt via map_err
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;configuratio…)
  end
  visioncraft__deepsec->>_tempfile: NamedTempFile::new_in(&workspace)
  opt via map_err
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;cannot final…)
  end
  opt closure
    visioncraft__deepsec->>visioncraft__deepsec___tAttempt: command(#quot;version#quot;, #quot;deepsec#quot;, _, &workspace, false, fal…?
    visioncraft__deepsec->>visioncraft__deepsec: supported_version()
    opt rc != 0 || version_text != supported_version()
      visioncraft__deepsec->>visioncraft__deepsec: error(78, #quot;Deepsec vers…)
      Note over visioncraft__deepsec: return Err(error(78, #quot;Deepsec version does not match th…
    end
    visioncraft__deepsec->>visioncraft__deepsec___tAttempt: command(#quot;revision#quot;, #quot;git#quot;, _, &root, false, false)?
    visioncraft__deepsec->>visioncraft__deepsec___tAttempt: command(#quot;dirty#quot;, #quot;git#quot;, _, &root, false, false)?
    visioncraft__deepsec->>_recorded: recorded::select(options, as_deref(), &attempt, &root)?
    visioncraft__deepsec->>_recorded: recorded::snapshot(&root, &selected)?
    visioncraft__deepsec->>_serde_json: serde_json::to_vec_pretty(&before)
    opt via map_err
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;source encod…)
    end
    visioncraft__deepsec->>visioncraft__deepsec: write_new(&join(), &_)?
    visioncraft__deepsec->>visioncraft__deepsec: write_new(&source_list, as_bytes())?
    opt options.scan_only || options.full
      visioncraft__deepsec->>visioncraft__deepsec___tAttempt: command(#quot;scan#quot;, #quot;deepsec#quot;, args, &workspace, false, fal…?
      opt rc != 0
        visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;Deepsec scan…)
        Note over visioncraft__deepsec: return Err(error(70, #quot;Deepsec scan failed#quot;))
      end
      opt options.scan_only
        Note over visioncraft__deepsec: return Ok(json!({ #quot;result#quot;: #quot;SCANNED#quot;, #quot;deepsec_exit#quot;: …
      end
    end
    opt options.full
      visioncraft__deepsec->>_recorded: recorded::pending(&data, &before)?
    end
    visioncraft__deepsec->>_recorded: recorded::run_files(&data)?
    visioncraft__deepsec->>visioncraft__deepsec___tAttempt: command(#quot;process#quot;, #quot;deepsec#quot;, args, &workspace, true, f…?
    opt (options.full && rc != 0) || (!options.full && ![0, …
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;Deepsec proc…)
      Note over visioncraft__deepsec: return Err(error(70, #quot;Deepsec processing failed#quot;))
    end
    visioncraft__deepsec->>visioncraft__deepsec: read_bounded(&join(), policy.max_output_bytes)
    opt rc == 1 && read_bounded(&report.join(#quot;comment.md#quot;), …
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;exit 1 witho…)
      Note over visioncraft__deepsec: return Err(error(70, #quot;exit 1 without fresh completed fi…
    end
    visioncraft__deepsec->>_recorded: recorded::snapshot(&root, &selected)?
    visioncraft__deepsec->>_serde_json: serde_json::to_vec_pretty(&after)
    opt via map_err
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;source encod…)
    end
    visioncraft__deepsec->>visioncraft__deepsec: write_new(&join(), &_)?
    opt before != after
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;review input…)
      Note over visioncraft__deepsec: return Err(error(70, #quot;review inputs changed while the r…
    end
    visioncraft__deepsec->>_recorded: recorded::validate_review(&data, &prior_runs, &expected…?
    visioncraft__deepsec->>visioncraft__deepsec___tAttempt: command(#quot;export#quot;, #quot;deepsec#quot;, args, &workspace, false, f…?
    opt export_rc != 0
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;Deepsec find…)
      Note over visioncraft__deepsec: return Err(error(70, #quot;Deepsec findings export failed#quot;))
    end
    visioncraft__deepsec->>visioncraft__deepsec: read_bounded(&join(), policy.max_output_bytes)?
    visioncraft__deepsec->>visioncraft__deepsec: findings(&raw)?
    opt rc == 1 && findings.is_empty()
      visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;findings exi…)
      Note over visioncraft__deepsec: return Err(error(70, #quot;findings exit with an empty expor…
    end
  end
  visioncraft__deepsec->>_serde_json: serde_json::to_vec_pretty(&receipt)
  opt via map_err
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;receipt enco…)
  end
  visioncraft__deepsec->>visioncraft__deepsec: write_new(&join(), &_)?
  visioncraft__deepsec->>_serde_json: serde_json::to_string_pretty(&receipt)
  opt via map_err
    visioncraft__deepsec->>visioncraft__deepsec: error(70, #quot;receipt enco…)
  end
```
