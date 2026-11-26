<!-- title: [correctness] Minor findings from the independent assessment: ownership and drops -->
<!-- labels: assessment,kind:tracking,severity:minor,priority:P3,domain:ownership,tier:advanced,tier:core -->
<!-- parent: - -->

## Summary
The independent assessment of baseline `e24a8752` left 2 surviving Minor finding(s) in the ownership and drops workstream. Each line names the remediation theme that closes it; details are in `plans/independent-assessment/findings/WS2-ownership.md`.

- [ ] **ASM-WS2-002** — A field move out of a call result is accepted (effort S, verified, verdict confirmed (Minor, verified); theme {THEME:R4})
  - A program the spec rejects compiles and runs memory-safely; portability of sources to a conforming compiler is the only cost.
  - Location: `compiler/src/own/lower.rs:5174-5184` and the partial-move check in `compiler/src/own/place.rs` at e24a8752
  - Fix: report E10026 for a field move whose root is a call result or other temporary, or ask for a spec ruling that admits it (the semantics are already sound).
- [ ] **ASM-WS2-003** — Move, copy, and drop decisions are re-derived at about 230 sites in two stages with different predicates (effort XL, probable, verdict confirmed (Minor, probable); theme {THEME:R7})
  - Move, copy, and drop decisions are made in the ownership pass and re-derived in codegen with a different predicate (`codegen/drop.rs:283` against `own.rs:675`), at about 230 sites in two stages. The split has already produced recorded defects (DEF-56/57/59/61); every new expression shape must be taught to both stages.
  - Location: `compiler/src/own/` (`.duplicable(` 79 sites, `maybe_owns(` 70 sites, 12 files); `compiler/src/codegen/` (`needs_drop` 84 sites in `function.rs` 38, `guards.rs` 17, `program.rs` 11, `drop.rs` 9, `cleanup.rs` 9); predicates `compiler/src/own.rs:675` (`maybe_owns_ty`, which counts generic-parameter mentions) and `compiler/src/codegen/drop.rs:283` (`needs_drop`, which excludes parameters) at e24a8752
  - Fix: make the ownership stage the single authority — annotate every IR store and place with its release obligation — and have codegen consume the annotation instead of `needs_drop`; WS8-08 weighs this against the DEF classes.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
