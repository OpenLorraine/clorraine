<!-- title: [correctness] Minor findings from the independent assessment: semantic core -->
<!-- labels: assessment,kind:tracking,severity:minor,priority:P3,domain:semantics,tier:advanced,tier:core -->
<!-- parent: - -->

## Summary
The independent assessment of baseline `e24a8752` left 1 surviving Minor finding(s) in the semantic core workstream. Each line names the remediation theme that closes it; details are in `plans/independent-assessment/findings/WS1-semantic-core.md`.

- [ ] **ASM-WS1-003** — Data inclusion (`embed`) is staged out, but chapter 13 reads stable and the limitation is undocumented (effort M, verified, verdict confirmed (Minor); theme {THEME:R4})
  - Every program that bakes an asset with `embed` (the spec's own sprite, palette, and level examples) is rejected on every profile; the spec suite hides the gap by excluding its golden.
  - Location: `compiler/src/sema/expr.rs:1734-1749`; `compiler/tests/compile_fail_spec_test.rs:24-52` at e24a8752
  - Fix: list data inclusion in `docs/known-limitations.md`, report a staged diagnostic (E9xxxx, as `comptime` does with E90001) instead of E10224, and either implement chapter 13 or ask for a spec ruling that marks it staged.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
