# Current State: Clorraine v1.0 Production Readiness

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The compiler is a std-only Rust workspace with one member, `compiler/`, and two binaries:
`clc` emits one C99 translation unit for the `host`, `hosted-amiga`, and `freestanding`
profiles; `clorraine` builds and runs programs. The standard library and all three runtime
shims are embedded in the binary with `include_str!`, so a release binary needs no installed
library tree.

The tracked defect backlog is complete: `plans/00-roadmap.md:7` records 546/546 rows Done and
zero open. The close-out ledger records `make check` green (27 spec files, 111 compile-fail
goldens, 85 Rust test targets) and `make verify: all legs passed`
(`plans/def-zero-closeout/05-findings.md:276`). There are 81 integration test files under
`compiler/tests/`, plus focused unit tests in `compiler/src/`.

The v1.0 boundary is explicitly defined in `plans/00-roadmap.md:589`: every non-staged rule and
the borrow checker at NLL precision. The deferred language features are listed at
`plans/00-roadmap.md:594`.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `plans/00-roadmap.md` | Programme and feature status | Add PLAN-15 and sync the stage |
| `plans/def-zero-closeout/05-findings.md` | Residual evidence and acceptance | Cite and close the release-blocking rows |
| `spec/README.md` | Specification status and chapter index | Change the v1.0 status to stable |
| `spec/*.md` | Frozen language rules | Review stale text; mark `stable (v1.0)` |
| `spec/14-diagnostics.md` | Diagnostic registry | No new code expected; confirm E10012 text |
| `compiler/src/sema/expr.rs` | Null, cast, and boolean surface | Route `null -> bool` through E10012 |
| `compiler/src/codegen/program.rs` | Initializer value retention | Extend retention to nested fill rows, multi-element view arrays, and initializer field call roots |
| `compiler/src/own/lower.rs` | Ownerless projection lowering | Keep the run-time index when binding a module-initializer view root |
| `compiler/tests/*.rs` | 81 integration suites; 21 honour `RDC_ASAN` | Add the production-readiness spec/impl suites |
| `Makefile` | `check`, `verify`, and the Amiga legs | Add `asan` and `differential`; include differential in `verify` |
| `docker/scripts/corpus-amiga.sh` | m68k corpus build/run and recorded-output diff | Reuse its compile flags for the differential runner |
| `docker/scripts/corpus-fs-uae.sh` | FS-UAE-only legs | Unchanged; remains local |
| `.github/workflows/` | CI | Add the host-only `check.yml` |
| `README.md` | Project status table | Update the stale NLL line and the licence section |
| `compiler/Cargo.toml` | Package metadata | Add `license = "MIT"` and release metadata |
| `plans/clorraine-dx/` | Existing DX plan, 0/28 | Execute as-is |
| `plans/00-roadmap.md` | PLAN-13 backlog row | Create the separate PLAN-13 plan |
| `docs/known-limitations.md` | Accepted residuals | Add the document |

### Code Analysis

The residual code paths are:

- **`null -> bool`.** `let b: bool = null;` already reports `E10012` through the nullability rule.
  The explicit cast spelling, `bool(null)`, reports `E10201` at
  `compiler/src/sema/expr.rs:1109-1125`; `!null` reports `E10218` at `:1616-1628`. The frozen rule
  is TY-7 (`spec/02-types.md:127`) for a null value where a non-nullable type is required, while
  the cast table (`spec/04-expressions.md:307-330`, TY-14) says an unsupported cast is E10201. The
  user ruling of 2026-11-01 makes the explicit cast and `!null` report E10012; `spec/04` records
  the null-cast exception.
- **View-array initializer.** `compiler/src/codegen/program.rs:1133-1430` binds fresh projection
  roots for initializer views. `retain_view_array_roots` (`:1399-1423`) walks the top-level
  elements and the fill value but does not recurse into a nested `ArrayLit`; a module-level nested
  fill row therefore leaks the unselected row's root (11 bytes). A module-level multi-element view
  array with a sibling owning field (`Box { names: [mk().v, mk().w], tag: stringOf("t") }`) retains
  the selected element but leaks the unselected element root (11 bytes). A local struct field
  initialized from a call through a fresh projection
  (`Box { n: sink(mk().xs[0]) }`) leaks the sibling element (11 bytes).
- **Indexed call-root projection.** A module-level view global
  `let g: str = mk()[pick()];` is accepted and binds a fresh projection root, but the run-time
  index is not part of the constant accessor chain, so the rebound root loses the index and the
  unselected slot leaks (11 bytes). The local owning spelling
  `let s: String = mk()[pick()]` is `E10026` and is not the residual.

The 21 ASan-capable suites are the files matching `RDC_ASAN` under `compiler/tests/`. The
existing `docker/scripts/corpus-amiga.sh` builds the four portable corpus programs for
hosted-amiga and diffs their recorded output; it does not run the host profile or compare
host and m68k output. `buffer-container`, `file-copier`, and `string-parser` are the three
host-portable corpus programs used by the host and m68k legs. `freestanding` also compiles on the
host (it imports only `std.io`), but it is the freestanding-profile program and is not part of the
host-vs-m68k seed. `graphics`, `interrupt-counter`, and `volatile-mmio` are profile-specific.

## Gaps Identified

### Gap 1: Release-blocking residuals
**Current Behavior:** `let b: bool = null;` already reports E10012; the explicit `bool(null)` cast
reports E10201 and `!null` reports E10218. The user ruling of 2026-11-01 makes both explicit
spellings E10012. A module-level nested fill row, a module-level multi-element view array, a local
initializer field call root, and a module-level indexed view global each leak 11 bytes. The local
owning element move and the move-only fill are rejected by the frozen spec and are not residual
defects.
**Required Behavior:** E10012 for the explicit cast/condition spellings, with the spec exception
recorded; each rooted element released exactly once under ASan; no accepted-program regression.
**Fix Required:** 03-01; spec tests, ASan probes, and the recorded E10012 ruling.

### Gap 2: Draft specification
**Current Behavior:** `spec/README.md` says "working draft (v0.4)" and every chapter is
`draft (Phase C)`.
**Required Behavior:** the v1.0 chapters read `stable (v1.0)`; stale statements are corrected;
`./spec/check.sh` passes.
**Fix Required:** 03-02.

### Gap 3: Opt-in sanitizers and no differential signal
**Current Behavior:** 21 suites honour `RDC_ASAN`, but no target runs them; the corpus script
checks m68k against recorded output only.
**Required Behavior:** `make asan` runs every ASan suite in a bounded process with serial tests and
a `-no-pie` ASan `cc` wrapper; `make differential` compares host and m68k output for the three
portable programs as a convenience gate over the recorded outputs.
**Fix Required:** 03-03.

### Gap 4: No CI
**Current Behavior:** no `.github/workflows/`; verification is local and container-based.
**Required Behavior:** a least-privilege host-only workflow runs `./spec/check.sh` and
`cargo test --workspace` on push and pull request.
**Fix Required:** 03-03.

### Gap 5: DX slice not executed
**Current Behavior:** `plans/clorraine-dx/99-execution-plan.md` is 0/28.
**Required Behavior:** the plan is 28/28 and its verification is green.
**Fix Required:** 03-04; execute the existing plan.

### Gap 6: PLAN-13 acceptance absent
**Current Behavior:** PLAN-13 is a backlog row; no `std.amiga.exec` wrapper and no external
console programs.
**Required Behavior:** a separate PLAN-13 plan delivers `std.amiga.exec` plus two allowlisted
console programs; both run on hosted-amiga against an independent oracle, and any language-surface
gap stops for a user ruling.
**Fix Required:** 03-04.

### Gap 7: No licence or release preparation
**Current Behavior:** no `LICENSE`, no Cargo licence metadata, no tags, no `RELEASE.md`.
**Required Behavior:** MIT licence and metadata; a release checklist with the AR-12 decision
(v1.0.0, source + Linux x86_64 tarball); publication in the conditional sub-plan.
**Fix Required:** 03-05.

## Dependencies

### Internal Dependencies

- The residual fixes depend on the frozen spec and the existing ASan suite helpers.
- `make differential` depends on the SDK container's `m68k-amigaos-gcc` and `vamos`.
- CI depends on the host-only tests' existing skip behaviour for missing m68k tools.
- The DX execution depends on `plans/clorraine-dx/`; PLAN-13 depends on a new plan.
- The release artifact is the conditional publish sub-plan (AR-12 resolved to v1.0.0/x86_64); the
  licence and metadata are in the 45-task main plan.

### External Dependencies

- The Docker SDK image (`docker/Dockerfile`, `docker/versions.env`) for the Amiga legs.
- GitHub Actions' hosted Ubuntu runner and a stable Rust toolchain for host CI.
- The existing `cc` host C compiler for the ASan and differential host legs.
- FS-UAE, the SDK X display, `docker/.env`, and the local Kickstart ROM path for `make verify`;
  these are a named preflight dependency, not an assumed environment.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Residual fixes expand beyond the named classes | Medium | Medium | Keep the scope to AR-4; file new rows instead of absorbing them |
| Spec stabilisation becomes a semantic rewrite | Medium | High | Review for stale text and status only; semantic changes need a separate ruling |
| Host-only CI gives false confidence for m68k | High | Medium | Document the skip in the workflow and keep `make verify` as the local gate |
| Differential script duplicates corpus logic | Medium | Low | Reuse the existing compile flags and `vamos` invocation; keep the seed list explicit |
| ASan PIE startup failure | Medium | Low | Run one suite per bounded process with a `-no-pie` `cc` wrapper; verify the backlog suite 50/50 |
| PLAN-13 scope grows | High | High | Bound the gate to `std.amiga.exec` plus two programs; the PLAN-13 plan owns the wider surface |
| Conditional publish cannot run | Medium | Low | Close the 45-task main plan as production-ready; keep the publish sub-plan blocked with the reason recorded |
