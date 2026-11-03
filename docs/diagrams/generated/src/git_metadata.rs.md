---
sealmap: 2
source: src/git_metadata.rs
module: "sym:cargo visioncraft . git_metadata/"
language: rust
source_hash: blake3:36f06c2d0b9d416dc106fd7d20839a5fc424fa52dc1e40f572a1acb24050a467
lines: 30
fragments: 1
---
# `sym:cargo visioncraft . git_metadata/` · src/git_metadata.rs
> Shared policy for read-only Git metadata inspection, not user build commands.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__git_metadata["visioncraft::git_metadata"] {
    <<module>>
    ~fixed_environment(crate) BTreeMap#lt;String, String#gt;
    ~prefix(crate) Vec#lt;String#gt;
  }
```
