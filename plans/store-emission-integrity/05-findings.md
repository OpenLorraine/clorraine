# Findings Ledger: Store-Emission Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

Independent per-phase review record. One compact row per ruling batch; the full evidence lives in
the reviewers' reports. Append rows as each phase's review closes — never batch across phases.

Review dispatch: P1/P2 — correctness reviewer + memory-safety auditor + semantics reviewer;
P3/P4 — correctness reviewer + memory-safety auditor + emission-integrity auditor. Packets include
the blast-radius and test-integrity steps.

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MINOR | plan progress header | fixed — the counter and stamp were corrected to 6/26 with the phase close (the review read the header before the last promotion landed) |
| RV-002, RV-003, SR-001, SR-002 | 1 | MINOR | pre-existing multi-arm read-only reports; impl-helper empty-emission reads; ASan env flag; standalone-pin discrimination | dismissed — pre-existing behavior or established helper conventions with no phase action; the suite discriminates the change through ST-1/ST-2 (suppression removed) and ST-5/ST-6 (non-target lend arms retained) |
| — (clean) | 1 | — | `own/expr.rs` `lend_moves_judged` filter, the spec suite, the impl pins | no critical/major findings — correctness clean (one report per site, direct branch unchanged, blast radius confined to the single suppression test); memory-safety clean (the amended shapes are rejected before emission after a base-compiler ASan use-after-free repro; every accepted shape builds and runs ASan-clean with leak detection, including nested, both-arms-self, call-arm, loop, and view instances; 23 related suites pass); semantics clean (conforms to the lend-store rule and every pinned spelling; no evaluation change) |
| RV-004 | 2 | CRITICAL | `own/expr.rs` value-position update walk | fixed — an update used as a value (`i32(s[0]++)`) now judges the immutable-view rule through a shared helper; the statement path evaluates its operand directly so the site keeps exactly one report; ST-14b, an impl pin, and positional probes (condition, for-step, two sites) pin the behavior; re-reviewed on the fix diff |
| SR-003 | 2 | MAJOR | `own/place.rs` `projection_ty` field step through `raw` | fixed — the field step now reaches through a raw pointer to its pointee, closing the `raw S` field spelling (`p.tag[0] = 65` reports one `E10042` naming `p.tag`); impl pin added; re-reviewed on the fix diff |
| RV-005 | 2 | MINOR | element loop over `str` | deferred — filed as DEF-118 (pre-existing, separate mechanism; the read-only loop also emits C that does not build) |
| RV-006 | 2 | MINOR | `ASSERTED_CODES` line length | fixed — reflowed to the sibling line width |
| SR-004 | 2 | MINOR | module-level fixed-array literal (memory-audit Finding 2) | deferred — this is the batch's own DEF-116, fixed by Phase 4 |
| SR-005 | 2 | MINOR | `cstring` element writes still accepted | dismissed — explicitly out of scope (AR-15); the mutability-table question stays open for a user decision |
| SR-006 | 2 | MINOR | nested field-path naming (`s.inner.tag[0]` → `'s.tag'`) | dismissed — matches the design's documented binding/field naming rule |
| SR-007 | 2 | MINOR | rejected local writes still lower IR after the report | dismissed — no observable effect (the driver gates output on a zero exit code; codegen never runs) |
| — (clean) | 2 | — | mutability-table conformance, taxonomy ordering, spec boundary | no findings — the check rejects exactly the `str`/`static str` element forms with one report per site and the E10023/E10024 precedence preserved; the spec delta is exactly the authorized row, golden, and driver registration; the audit found no memory-safety defect in the reviewed change (the rejected forms are compile-time rejections; accepted controls build and run ASan-clean) |
| RV-007 | 3 | MINOR | `struct_units()` computed twice; enum member macros widened in scope | dismissed — no observable defect (the computation is pure and stable; the macro scope widening is inherent to the required enum-before-struct order) |
| SR-008 | 3 | MAJOR | multi-extent row slice of a user struct (`P[][2]`) | deferred — filed as DEF-120 (pre-existing, base == head; the pointer-to-array element needs the complete struct, so the minimal fix revises the recorded AR-8 order — a scope choice for the user) |
| SR-009 | 3 | MAJOR | module-level array/slice initializer (auditor's second pre-existing finding) | deferred — already the batch's DEF-116, fixed by Phase 4 (the same residual recorded as SR-004 in Phase 2) |
| — (clean) | 3 | — | `codegen.rs` run order, the aggregate split, the position and re-emission pins | no findings introduced by the change — correctness clean (all dependency edges enumerated and satisfied; declaration-order-only proven by identical line multisets); memory-safety clean (every reorder-touched shape builds, runs, and is ASan-clean; bodies byte-identical; release emission unchanged); emission-integrity clean (796 accepted programs and the corpus diff order-only, 0 content diffs, 0 head-only build regressions, no duplicate typedefs, the six base-only failures newly fixed) |
| RV-008 | 4 | CRITICAL | `materialize_init_local` dropped the operand's runtime guards | fixed — a materialized operand now emits its guards (statement guards ahead of the store; `guarded_value` inside a guarded store, so the guards run only on the reaching path); ST-29b and an impl pin added; both the correctness and emission-integrity reviews reproduced the same defect independently and the memory audit confirmed the ASan out-of-bounds read; re-reviewed on the fix diff |
| RV-009 | 4 | MAJOR | module `slice<T>` explicit type argument agreement | fixed — the module path reports `E10200` exactly as the function path does; impl pin added |
| RV-010 | 4 | MAJOR | fixed-array intrinsic operand materialized as an array temporary | fixed — the fixed-array carve-out mirrors the function path, so the operand keeps its form and decays; impl pin added |
| RV-011 | 4 | MINOR | module re-emission pin is determinism-only | dismissed — a determinism guard by design; the discriminating pins are the value and position assertions |
| SR-010 | 4 | MINOR | nested/2D conditional re-materialized per row (redundant condition copies) | fixed — the fill prepares the value once and recurses without re-preparing, so no redundant init locals are emitted |
| SR-011 | 4 | MAJOR | module-init guarded operand evaluated twice (guard versus read) | deferred — filed as DEF-121 (pre-existing, base == head; the fix is the function path's materialize-before-guard contract) |
| SR-012 | 4 | MINOR | slice-typed module binding from an array literal | deferred — filed as DEF-122 |
| SR-013 | 4 | MINOR | struct-with-array-field literal from an array place (compound literal) | deferred — the same class as the tracked DEF-80 (compound literals against the C99-subset contract) |
| SR-014 | 4 | MAJOR | freestanding `bool` has no declaration | deferred — filed as DEF-123 |
| — (clean) | 4 | — | the fill walk, the intrinsic lowering, the init-local materialization, the pins | no findings introduced beyond the fixed ones — the corpus emission is unchanged (0 content diffs; the corpus does not exercise module-initializer shapes), there are no head-only build regressions, every covered shape builds and runs ASan-clean, and the memory audit found no leak, double-free, or use-after-free in the changed shapes (module-level `let` rejects non-duplicable types, so owned-element cleanup is unreachable) |
