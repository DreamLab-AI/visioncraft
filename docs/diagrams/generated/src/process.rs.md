---
sealmap: 2
source: src/process.rs
module: "sym:cargo visioncraft . process/"
language: rust
source_hash: blake3:b24b8fb96eba5f197c4a00e8da464df9d6d53b98c4db058f9b684e8da1c64e3c
lines: 425
fragments: 2
---
# `sym:cargo visioncraft . process/` · src/process.rs
> Bounded local command execution.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__process___tCapturedRun["CapturedRun"] {
    <<struct>>
    +termination: Termination
    +exit_code: Option#lt;i32#gt;
    +signal: Option#lt;i32#gt;
    +started_at_ms: u64
    +finished_at_ms: u64
    +elapsed_ms: u64
    +stdout: PathBuf
    +stderr: PathBuf
    +error: Option#lt;String#gt;
  }
  class visioncraft__process___tTermination["Termination"] {
    <<enum>>
    Completed
    Unavailable
    FailedToStart
    TimedOut
    Interrupted
    CaptureFailed
  }
  class visioncraft__process["visioncraft::process"] {
    <<module>>
    +private_directory(path: &Path) io::Result#lt;#40;#41;#gt;
    +private_file(path: &Path) io::Result#lt;File#gt;
    +run_command(invocation: &Invocation, root: &Path, output: &Path, cancelled: &AtomicBool, inherit_stdin: bool,) io::Result#lt;CapturedRun#gt;
    ~run_command_with_env(crate) io::Re…
  }
  class visioncraft__plan___tInvocation["Invocation"] {
    <<struct in src/plan.rs>>
  }
  visioncraft__process ..> visioncraft__plan___tInvocation
  visioncraft__process ..> visioncraft__process___tCapturedRun
  visioncraft__process___tCapturedRun *-- visioncraft__process___tTermination : termination
```

## `sym:cargo visioncraft . process/run_command_with_env().`
`pub(crate) fn run_command_with_env(invocation: &Invocation, root: &Path, output: &Path, cancelled: &AtomicBool, inherit_stdin: bool, overrides: &std::collections::BTreeMap<String, String>,) -> io::Re…` · L225-L425
```mermaid
sequenceDiagram
  participant visioncraft__process as process mod
  participant visioncraft__plan___tInvocation as Invocation
  participant _nix as nix ext
  visioncraft__process->>visioncraft__plan___tInvocation: validate()
  visioncraft__process->>visioncraft__process: unix_now()
  visioncraft__process->>visioncraft__process: private_file(&stdout_path)?
  visioncraft__process->>visioncraft__process: private_file(&stderr_path)?
  opt cancelled.load(Ordering::SeqCst)
    Note over visioncraft__process: return Ok(result)
  end
  opt invocation.timeout_ms == 0 || invocation.max_output_…
    Note over visioncraft__process: return Err(io::Error::new(io::ErrorKind::InvalidInput, …
  end
  opt Err(err)
    visioncraft__process->>visioncraft__process: unix_now()
    Note over visioncraft__process: return Ok(result)
  end
  visioncraft__process->>visioncraft__process: capture(expect(), stdout_file, clone(), clone(), clone(…
  visioncraft__process->>visioncraft__process: capture(expect(), stderr_file, total, clone(), clone(),…
  opt result.termination == Termination::Completed
    opt via is_ok_and
      visioncraft__process->>_nix: Pid::from_raw(p)
      visioncraft__process->>_nix: signal::killpg(from_raw(), None)
    end
  end
  opt result.termination != Termination::Completed
    visioncraft__process->>visioncraft__process: signal_group(pid, SIGINT)
    visioncraft__process->>visioncraft__process: signal_group(pid, SIGKILL)
  end
  visioncraft__process->>visioncraft__process: unix_now()
```
