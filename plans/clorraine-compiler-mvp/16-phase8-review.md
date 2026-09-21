# Phase 8 Review: Cross-Toolchain Suites, Corpus E2E, Packaging

> **Document**: 16-phase8-review.md
> **Parent**: [Index](00-index.md)
> **Phase diff**: baseline `e7d8f37639f944eb6b2ce4a9117051c8dfe71345` → accepted commit `9601be6`
> **Scope mode**: strict
> **Reviewers**: correctness (`RV-6xx`) and semantics/runtime auditor (`SA-6xx`), dispatched in parallel on the phase diff

## Result

`make verify` is green and the MVP was accepted. Both reviewers confirmed the
Phase 8 work is correct as a whole — the corpus host leg and E11200 diagnostic,
the `(void)` → `if (sizeof(...))` emitter change (warning-free on host cc, m68k
GCC, and vbcc; inert; no `sizeof` on a `void` or bare function type), the E90001
`comptime` body diagnostic, the E11200 import check for the corpus, the cross
legs, and the compile-fail scope honesty. The findings below are incomplete
edges of the new code and two pre-existing defects the audit surfaced.

## Findings

| ID | Severity | Title | Location | Evidence |
|---|---|---|---|---|
| SA-603 / RV-607 | 🔴 CRITICAL | A module-path global read emits invalid C | `compiler/src/codegen/program.rs:455-468` | `module test.PA; export let a: i32 = test.PB.b + 1;` compiles (exit 0) and emits `(u32)test.PB.b`, which is not valid C. The resolver guarantees `A.B.x` (MOD-13), so this is a real miscompile. Pre-existing at the baseline, surfaced by this audit. |
| RV-601 / SA-604 | 🟠 MAJOR | A qualified `std.amiga.*` reference under `host` bypasses E11200 | `compiler/src/driver.rs:165-185` | `std.amiga.dos.delayTicks(1);` with no import under `--profile host` is accepted and emits a call to an undefined host symbol; only the import form is rejected. `inject_stdlib` loads modules named by qualified references, so the guard must check that set too. |
| RV-602 / SA-601 / SA-602 | 🟠 MAJOR | The E11100 graph misses module-path, qualified, and import edges | `compiler/src/sema/init_cycle.rs:44-55,110-143,221-226` | Three facets: (a) `b.v` parses as `Qualified(["b","v"])` and `qualified_global` requires an import alias, so a `b.field` read is not an edge; (b) a full module path `test.Beta.y` is not resolved as a module path; (c) import-only module cycles with runtime globals are accepted. Each contradicts the spec's init dependency rule; `codegen/init.rs` shares the same wrong assumption. |
| RV-603 | 🟠 MAJOR | Same-module globals are not ordered by their reads | `compiler/src/codegen/program.rs:392-406` | `let a: i32 = b + 1; let b: i32 = 41;` returns 1, while swapping the declarations returns 42. `init_order()` orders modules only, never globals within a module. Pre-existing; a legal forward reference (MOD-17) is miscompiled. |
| RV-604 | 🟡 MINOR | The run-fault Amiga leg accepts a timeout as a pass | `docker/scripts/run-fault-amiga.sh:63-75` | `timeout` returns 124, which is nonzero, so a hanging case passes the exit check. |
| RV-605 | 🟡 MINOR | A user module named `std.amiga.*` is falsely rejected on host | `compiler/src/driver.rs:174`, `compiler/src/profile.rs:52-57` | `module std.amiga.custom;` imported on host reports E11200 although it is user code; the check keys on the name prefix, not stdlib provenance. |
| RV-606 | 🟡 MINOR | The skip report does not require pending to be empty | `compiler/tests/compile_fail_spec_test.rs:438-449` | The report prints `pending` with reasons but never asserts it is empty; the accepted rule is that pending is zero. |
| SA-605 | 🟡 MINOR | The corpus E11200 test does not assert exit 1 | `compiler/tests/corpus_spec_test.rs:166-170` | ST-CORPUS-2 requires exit 1; the test accepts any nonzero exit. |

## Checked clean

- E90001: exactly one diagnostic per `comptime` declaration; the body is not
  type-checked; no collision; the reserved-intrinsic golden still reports only
  E20006.
- E11200 (import form): reported before `inject_stdlib`/`load`, so no cascade;
  `hosted-amiga` untouched; the interrupt-counter reports it as the first line
  and exits 1.
- Emitter idiom: warning-free on host `cc`, m68k GCC, and vbcc; semantically
  inert; no `sizeof` on a `void` or bare function type; private functions stay
  `static`.
- E11100 false positives: `const` reads, function names, enum/type names, and
  a call to a function that reads a global create no edge; self-reference and
  alias/ternary/cast/struct-literal cycles report; one diagnostic per cycle.
- Scripts: `bash -n`/`shellcheck` clean; mismatches fail the legs; scratch is
  confined to `build/`; the FS-UAE leg terminates the emulator.
- Compile-fail scope matches the ST-CF table: 91 asserted, 15 excluded with
  reasons, 0 pending.

## Rulings

User ruling (2026-09-21): **fix all eight** — the critical module-path global
lowering, the E11200 qualified-reference check, the E11100 graph
(module-path/qualified/import edges), same-module global ordering, and the four
minors. Verify, then run the one permitted scoped re-review of the fix diff.

## Fix batch (commit `fed4857`)

All eight were fixed; `make check` and `make verify` are green (269 unit,
46 codegen-run, 3 compile-fail, 4 corpus, 1 determinism, 6 run-fault, 46 stage,
8 stdlib). A shared `compiler/src/init_graph.rs` now backs both the E11100 check
and the codegen ordering, and the sema init-cycle tests moved to
`sema/init_tests.rs`.

## Re-review of the fix diff

The single permitted re-review confirmed RV-601/SA-604, RV-602/SA-601/SA-602,
RV-603, RV-604, RV-606, and SA-605 resolved. It rejected SA-603/RV-607 and
RV-605 as only partially resolved and found one new major:

| ID | Severity | Title | Location | Evidence |
|---|---|---|---|---|
| RV-701 | 🟠 MAJOR | The module-path global fix discards the module, so a same-named global in another module resolves to the wrong one | `codegen/program.rs:451-459`, `own/place.rs:434-440`, `codegen/expr.rs:161-165`, `codegen.rs:85,251` | Two modules each `export let y`; `let a: i32 = test.PB.y + 1` emits `cl_test_PA_y` and never declares `cl_test_PB_y` (`cc`: undeclared identifier). `resolve_global` returns the module but the fix drops it and re-resolves by simple name through `Emitter.globals`, which is keyed by name only. SA-603 is therefore not fixed at the root for a name collision. |
| RV-605r | 🟠 MAJOR | A user module named exactly `std.amiga.dos`/`std.amiga.interrupt` is still rejected on host while `hosted-amiga` accepts it | `driver.rs:203-205`, `inject_stdlib` at `:137` | `inject_stdlib` prefers a user-provided module of that name (treats it as present), but the profile check keys on `stdlib::source(name).is_some()` and rejects it, so the two decisions disagree. |
| RV-702 | 🟡 MINOR | The profile scan walks staged `comptime` bodies, masking E90001 with E11200 | `driver.rs:184-188`, `load/scan.rs:45` | A staged `comptime` body referencing `std.amiga.dos` on host reports E11200 instead of the expected E90001. Both reject the program, so only the diagnostic is wrong. |

No new critical findings. `sema/tests.rs` is 564 lines and all moved tests run.

### Residual ruling

User ruling (2026-09-21): **fix all three** — carry the module through the
global place and key the emitter's global table by `(module, name)` (RV-701);
make the profile check agree with `inject_stdlib` when a user module shadows a
stdlib name (RV-605r); skip staged items in the profile scan so E90001 wins
(RV-702). The single permitted re-review is spent, so this follow-up is verified
by `make check` plus `make verify`, with no further independent review.
