# Requirements: Def Backlog Closeout

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The compiler has seven filed, reproduced defects remaining in its backlog. Each is a
place where a value crosses an ownership, place, or scope boundary without the treatment its type,
mode, or syntax requires. This plan closes all seven with the smallest fix that reuses the existing
machinery, adds exactly one approved diagnostic (`E10405`), and pins every fix with specification
tests, implementation tests, and deterministic host-AddressSanitizer probes.

The defects are DEF-63 – DEF-69, each filed with a reproduction in `plans/00-roadmap.md`. The
component documents `03-01` – `03-05` own the designs; `07-testing-strategy.md` owns the expected
behavior.

## Functional Requirements

### Must Have

- [ ] R1 — DEF-63: `flag && sink(mkv().a)` is accepted and, when the guard is true, transfers the
  taken field to the callee, zeroes the emptied slot in the guard, and releases the call result's
  remaining fields exactly once; when the guard is false, nothing leaks and nothing double-frees
  (AR-8, 03-01 §1).
- [ ] R2 — DEF-64: `Box<T> { v: mk()[i] }` (concrete and generic) emits the root call, evaluates the
  run-time index once, empties the `i`-th slot, and releases the result exactly once; the emitted C
  builds and runs (AR-9, 03-01 §2).
- [ ] R3 — DEF-66: the let-store form `let s: T = slice(mk(), 0, 2)[i]`, the take form, and the
  return form are probed on the current compiler first; each form that misbehaves is fixed by
  teaching the place derivation about the slice intrinsic root, and each clean form gets a
  regression pin closing the roadmap row as already-fixed (AR-2, 03-01 §3).
- [ ] R4 — DEF-65: a `borrow` out-slot of a nullable owned type (`String?`, `File?`, `str?`,
  `T[]?`) is lowered as a pointer; a non-null incoming slot is dropped exactly once by the callee's
  assignment and the caller observes the new value; the null-start case is unchanged; duplicable
  struct borrows keep their deliberate by-value path (AR-3, AR-10, 03-03).
- [ ] R5 — DEF-67: an ownerless owned call result consumed as a borrow argument inside a larger
  expression (`i32(len(stringOf("abc")))`, the dot `len` form, and `slice`-argument forms) is bound
  and released exactly once at statement end or return; guarded operands defer correctly and no
  double release appears where a destination adopts the value (AR-6, 03-02).
- [ ] R6 — DEF-68: a `return` inside a `defer` block in a do-while body receives the condition
  temporaries' releases; direct early returns, `while` bodies, and break/fall-through paths are
  unchanged and no release is appended outside the loop (AR-7, 03-04).
- [ ] R7 — DEF-69: a lambda that reads a binding from an enclosing function or lambda without
  listing it reports the approved `E10405`; listed captures, capture-less lambdas reading
  module-level globals/functions, and the existing move/write capture diagnostics are unchanged;
  an inner lambda must list an outer lambda's capture to read it (AR-4, AR-5, AR-13, AR-14,
  03-05).
- [ ] R8 — Workflow: every phase's spec cases that describe memory behavior are proven with a
  deterministic host-ASan probe at task time, not only at review (AR-15).
- [ ] R9 — Close-out: a probe runner under `build/def-backlog-closeout-probes/` reproduces each
  original defect as fixed, following the previous plan's runner pattern (AR-15).
- [ ] R10 — Specification change: the only `spec/` edits are the `E10405` registry row, its
  compile-fail golden, and the `ASSERTED_CODES` entry (AR-4, AR-13).
- [ ] R11 — Documentation: the `AGENTS.md` test map gains the new suites, and the roadmap rows
  DEF-63 – DEF-69 are set to Done with fresh timestamps (AR-15).
- [ ] R12 — No regression: unaffected programs keep their emitted C, `make check` is green at every
  phase, and `make verify` is green at close-out.

### Won't Have (Out of Scope)

- PLAN-05 (developer experience), PLAN-13 (Amiga library surface), PLAN-14 (generated-C
  optimization) — separate roadmap items.
- Capture inference for unlisted reads — DEF-69 resolves the other way (AR-4).
- Any diagnostic other than `E10405`.
- Converting every `borrow` parameter to a pointer; the duplicable-struct deviation stays (AR-3).
- Expression-wide materialization of every owned call result (AR-6).
- A new test harness, runtime dependency, or crate — probes reuse the existing host toolchain
  (AR-15).

## Technical Requirements

### Performance

- No performance targets; the fixes must not change the emitted C of programs that do not use the
  affected shapes. The phase reviews dispatch the emission-integrity auditor on every IR/codegen
  change (AR-16).

### Compatibility

- Host, hosted-amiga (m68k, 32-bit, big-endian), and freestanding profiles stay buildable; the
  `borrow` pointer lowering follows the existing `spec/17-codegen.md` mapping.
- The frozen specification changes only by the approved `E10405` row and golden (AR-13).

### Security

- No external input reaches these paths; the relevant threat is memory unsafety (leak, double
  free, use-after-free, invalid free). The deterministic host-ASan probe matrix is the guard, and
  the phase reviews dispatch the memory-safety auditor (AR-16).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Defect coverage | All 7 / split DEF-65 / fewer | All 7, one plan | One bounded cycle closes the filed backlog; all fixes share the own/codegen surface | AR-1 |
| DEF-66 handling | Reproduce first / fix regardless / defer | Reproduce first | A later fix may already cover it; probes decide, regression pins keep the closure honest | AR-2 |
| DEF-65 width | Nullable out-slots / all borrows / plus structs | Nullable out-slots | Fixes the ownership bug without reopening the deliberate W10001 deviation | AR-3 |
| DEF-69 | Reject (E10405) / implicit capture / defer | Reject with E10405 | Matches the spec's explicit capture-list design (L-1/L-2) | AR-4, AR-13, AR-14 |
| Nested captures | List each level / outer flows in | List each level | One consistent rule; the lifted body receives only listed captures | AR-5 |
| DEF-67 | Bind intrinsic args / expression-wide / minimal | Bind intrinsic args | Smallest change through the existing `materialize_owned_argument` path | AR-6 |
| DEF-68 | Defer-marker scan / loop membership | Defer-marker scan | Keeps the fix inside the existing release walk; no new loop state | AR-7 |
| Phase grouping | Mechanism groups / per-defect / two large | Mechanism groups | Fixes that share functions land in one phase and one review | AR-11 |
| Names | `def-backlog-closeout` / continuation names | `def-backlog-closeout` | Independent plan with its own suites and modules | AR-12 |
| Spec set | Row+golden+code / plus chapter | Row+golden+code | L-1/L-2 already state the rule; no chapter edit needed | AR-13 |
| Workflow | Adopt all / subset / prior | Adopt all | Task-time probes and blast-radius packets caught the last cycle's defects earlier | AR-15 |

> **Traceability:** Every scope decision references the Ambiguity Register entry (AR #) that
> resolved it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] All seven defects (DEF-63 – DEF-69) are fixed or, for DEF-66, proven already fixed with
   regression pins; every R1 – R7 behavior is covered by at least one specification test and one
   implementation test.
2. [ ] The host-ASan probe matrix in `07-testing-strategy.md` is clean for every fixed shape.
3. [ ] `make check` is green at every phase; `make verify` is green at close-out.
4. [ ] The only `spec/` changes are the approved `E10405` row, golden, and `ASSERTED_CODES` entry;
   `spec/check.sh` passes.
5. [ ] The close-out probe runner reproduces each original defect as fixed and exits zero.
6. [ ] Unaffected programs keep their emitted C (emission-integrity review finds no unintended
   differences).
7. [ ] `AGENTS.md` lists the new suites; roadmap DEF-63 – DEF-69 are Done; the plan is Complete.
8. [ ] Every phase review is dispatched per the `AGENTS.md` matrix with blast-radius and
   test-integrity steps; no open critical or major finding remains.
