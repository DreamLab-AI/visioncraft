---
sealmap: 2
source: src/process.rs
module: "sym:cargo visioncraft . process/"
language: rust
source_hash: blake3:3653d7e09689bd761ffeff3e445975bfa935814efde0ebb6aa9c1ce2adcd903b
lines: 841
fragments: 7
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

## `sym:cargo visioncraft . process/group_running().`
`fn group_running(pid: u32, deadline: Instant) -> io::Result<bool>` · L305-L318
```mermaid
sequenceDiagram
  participant visioncraft__process as process mod
  participant _nix as nix ext
  participant visioncraft__process__linux_proc as linux_proc mod
  visioncraft__process->>_nix: Pid::from_raw(group)
  visioncraft__process->>_nix: signal::killpg(from_raw(), None)
  visioncraft__process->>visioncraft__process: group_probe(killpg())?
  opt !group_probe(killpg(Pid::from_raw(group), None))?
    Note over visioncraft__process: return Ok(false)
  end
  visioncraft__process->>visioncraft__process__linux_proc: linux_proc::group_running(group, deadline)
```

## `sym:cargo visioncraft . process/linux_proc/no_longer_member().`
`fn no_longer_member(pid: u32, group: i32) -> io::Result<bool>` · L421-L429
```mermaid
sequenceDiagram
  participant visioncraft__process__linux_proc as linux_proc mod
  participant _nix as nix ext
  participant _io as io ext
  opt via map_err
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: unknown()
  end
  visioncraft__process__linux_proc->>_nix: Pid::from_raw(_)
  visioncraft__process__linux_proc->>_nix: unistd::getpgid(Some())
  opt Err(error)
    visioncraft__process__linux_proc->>_io: Error::from(error)
  end
```

## `sym:cargo visioncraft . process/linux_proc/tasks_dead().`
`fn tasks_dead(process: &Path, group: i32, deadline: Instant) -> io::Result<bool>` · L431-L451
```mermaid
sequenceDiagram
  participant visioncraft__process__linux_proc as linux_proc mod
  visioncraft__process__linux_proc->>visioncraft__process__linux_proc: ids(&join(), deadline)?
  opt tasks.is_empty()
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: unknown()
    Note over visioncraft__process__linux_proc: return Err(unknown())
  end
  loop for tid in tasks
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: bounded_read(&join(), 8192)
    alt Err(error) if error.kind() == io::ErrorKind::NotFound &…
      visioncraft__process__linux_proc->>visioncraft__process__linux_proc: no_longer_member(tid, group)?
    else Err(error)
      Note over visioncraft__process__linux_proc: return Err(error)
    end
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: state(&record, tid, Some())?
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: dead(_)
    opt !dead(state(&record, tid, Some(group))?)
      Note over visioncraft__process__linux_proc: return Ok(false)
    end
  end
```

## `sym:cargo visioncraft . process/linux_proc/dead_members().`
`fn dead_members(group: i32, deadline: Instant) -> io::Result<Option<BTreeSet<u32>>>` · L453-L487
```mermaid
sequenceDiagram
  participant visioncraft__process__linux_proc as linux_proc mod
  visioncraft__process__linux_proc->>visioncraft__process__linux_proc: ids(new(), deadline)?
  loop for pid in ids(Path::new(#quot;/proc#quot;), deadline)?
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: no_longer_member(pid, group)?
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: bounded_read(&join(), 8192)
    alt Err(error) if error.kind() == io::ErrorKind::NotFound &…
      visioncraft__process__linux_proc->>visioncraft__process__linux_proc: no_longer_member(pid, group)?
    else Err(error)
      Note over visioncraft__process__linux_proc: return Err(error)
    end
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: state(&record, pid, Some())?
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: dead(_)
    opt !dead(state(&record, pid, Some(group))?)
      Note over visioncraft__process__linux_proc: return Ok(None)
    end
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: tasks_dead(&process, group, deadline)
    alt Ok(false)
      Note over visioncraft__process__linux_proc: return Ok(None)
    else Err(error) if error.kind() == io::ErrorKind::NotFound &…
      visioncraft__process__linux_proc->>visioncraft__process__linux_proc: no_longer_member(pid, group)?
    else Err(error)
      Note over visioncraft__process__linux_proc: return Err(error)
    end
  end
```

## `sym:cargo visioncraft . process/linux_proc/group_running().`
`pub(super) fn group_running(group: i32, deadline: Instant) -> io::Result<bool>` · L489-L521
```mermaid
sequenceDiagram
  participant visioncraft__process__linux_proc as linux_proc mod
  participant _nix as nix ext
  participant visioncraft__process as process mod
  visioncraft__process__linux_proc->>visioncraft__process__linux_proc: bounded_read(new(), 1_048_576)?
  opt via map_err
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: unknown()
  end
  visioncraft__process__linux_proc->>visioncraft__process__linux_proc: unrestricted(_)
  opt !unrestricted(std::str::from_utf8(&mounts).map_err(|…
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: unknown()
    Note over visioncraft__process__linux_proc: return Err(unknown())
  end
  visioncraft__process__linux_proc->>visioncraft__process__linux_proc: bounded_read(new(), 8192)?
  visioncraft__process__linux_proc->>visioncraft__process__linux_proc: state(&_, id(), None)?
  loop for _ in 0..3
    visioncraft__process__linux_proc->>visioncraft__process__linux_proc: dead_members(group, deadline)?
    opt let-else
      Note over visioncraft__process__linux_proc: return Ok(true)
    end
    opt members.is_empty()
      visioncraft__process__linux_proc->>_nix: Pid::from_raw(group)
      visioncraft__process__linux_proc->>_nix: signal::killpg(from_raw(), None)
      visioncraft__process__linux_proc->>visioncraft__process: super::group_probe(killpg())?
      opt super::group_probe(nix::sys::signal::killpg(Pid::fro…
        visioncraft__process__linux_proc->>visioncraft__process__linux_proc: unknown()
      end
      Note over visioncraft__process__linux_proc: return if super::group_probe(nix::sys::signal::killpg(P…
    end
    opt previous.as_ref() == Some(&members)
      Note over visioncraft__process__linux_proc: return Ok(false)
    end
  end
  visioncraft__process__linux_proc->>visioncraft__process__linux_proc: unknown()
```

## `sym:cargo visioncraft . process/run_command_with_env().`
`pub(crate) fn run_command_with_env(invocation: &Invocation, root: &Path, output: &Path, cancelled: &AtomicBool, inherit_stdin: bool, overrides: &std::collections::BTreeMap<String, String>,) -> io::Re…` · L613-L841
```mermaid
sequenceDiagram
  participant visioncraft__process as process mod
  participant visioncraft__plan___tInvocation as Invocation
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
  loop loop
    visioncraft__process->>visioncraft__process: leader_exited(&child)
  end
  opt result.termination == Termination::Completed
    visioncraft__process->>visioncraft__process: group_running(pid, _)
  end
  opt result.termination != Termination::Completed
    visioncraft__process->>visioncraft__process: signal_owned_group(&child, SIGINT)
    loop while owns_child && grace.elapsed()<Duration::from_mill…
      opt !exited
        visioncraft__process->>visioncraft__process: leader_exited(&child)
      end
    end
    visioncraft__process->>visioncraft__process: signal_owned_group(&child, SIGKILL)
  end
  visioncraft__process->>visioncraft__process: unix_now()
```
