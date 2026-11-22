---
sealmap: 2
source: src/git_metadata.rs
module: "sym:cargo visioncraft . git_metadata/"
language: rust
source_hash: blake3:01e4dec28477d4cb0734158fc900f8a00a153cb92f6441ce8cd88c1713d91e2d
lines: 184
fragments: 1
---
# `sym:cargo visioncraft . git_metadata/` · src/git_metadata.rs
> Shared policy for read-only Git metadata inspection, not user build commands.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__git_metadata___tEntry["Entry"] {
    <<struct>>
    +mode: String
    +object: String
  }
  class visioncraft__git_metadata["visioncraft::git_metadata"] {
    <<module>>
    ~fixed_environment(crate) BTreeMap#lt;String, String#gt;
    ~index_entries(crate) Result#lt;BTreeMap#lt;String, Entry#gt;, &'static str#gt;
    ~prefix(crate) Vec#lt;String#gt;
    ~raw_hash_arguments(crate) Vec#lt;String#gt;
    ~raw_hashes(crate) Result#lt;BTreeMap#lt;String, String#gt;, &'static str#gt;
    ~tree_arguments(crate) Vec#lt;String#gt;
    ~tree_entries(crate) Result#lt;BTreeMap#lt;String, Entry#gt;, &'static str#gt;
  }
  visioncraft__git_metadata ..> visioncraft__git_metadata___tEntry
```
