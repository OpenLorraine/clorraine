# Requirements: Borrow-Store Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The plan fixes five open write-path defects (DEF-100, DEF-101, DEF-102, DEF-105, DEF-106) filed by
the `borrow-generic-integrity` reviews. Every defect is a write or a move *through a borrowed
place*: a whole-binding self-assign on a parameter-typed lend root, a slot-value return that does
not empty the caller's slot, a runtime-indexed element self-assign on a nullable out-slot, an
element store through a possibly-null view, and a constant-index element store without its bounds
trap.

Each fix implements an existing rule of the frozen specification — it adds no language surface and
no diagnostic code. The one internal addition is the per-instance self-assign duty record and its
audit, following the existing `Requires*` pattern (AR-2). DEF-102 changes no semantics at the plan
baseline: the filed "unguarded" premise does not reproduce, and the plan pins the conservative
guard that already exists (AR-4).

## Functional Requirements

### Must Have

- [x] R1 — DEF-100: a whole-binding self-assign through a parameter-typed lend root
  (`copySelf<T>(borrow slot: T) { slot = slot; }`) is judged per substituted type: a nullable
  instance is rejected `E10020` (the concrete out-slot self-assign diagnostic — nullability takes
  precedence even when the substitution is also a view, `raw`, or `cstring`), an owning
  non-nullable instance is rejected `E10034` (the concrete spelling's diagnostic), and a
  non-nullable duplicable, view, or unknown instance stays accepted; one report per site — the
  first offending instance in instantiation order.
  Source: spec/03 OW-16, spec/14 E10020/E10034 · AR-2, AR-7.
- [x] R2 — DEF-102: the runtime-indexed element self-assign on a nullable out-slot root
  (`if (xs != null) { xs[i] = xs[i]; }`) is rejected `E10020`; the conservative comparison (any
  two runtime-indexed projections on the same root compare equal) is documented and pinned; a
  distinct-constant pair (`xs[1] = xs[0]`) stays accepted.
  Source: spec/03 E10020 rule · AR-4.
- [x] R3 — DEF-101: a direct whole-slot return (`func cid(borrow v: String?): String? { return v; }`)
  transfers the value to the caller and empties the caller's slot; the caller observes a null slot
  and the returned value; the run is ASan-clean (no double free, no leak).
  Source: spec/06 §4 out-parameter ownership, spec/16 in-out slot · AR-3.
- [x] R4 — DEF-105: an element store through a possibly-null view requires the same non-null
  narrowing the read path requires; without it the store is rejected `E10010` ("value '…' may be
  null here"); a narrowed base and a non-nullable view stay accepted.
  Source: spec/decisions T8/D9, spec/02 TY-6 · AR-5.
- [x] R5 — DEF-106: a constant-index element store into a dynamically-bounded view emits the
  `E11211` bounds trap before the store, exactly as the read path and the runtime-index store path
  do — in every store form (plain assignment, compound assignment, update, and the row store);
  the guard statement is constructed only when a level's base has a dynamic bound, and a
  fixed-array-only base keeps the semantic-layer check and its emission is unchanged.
  Source: spec/14 E11211, spec/06 FN · AR-6.

### Should Have

- [x] R6 — Each fix ships with specification tests proving the corrected behavior (red where the
  baseline is wrong; green pins where the baseline is already right) and implementation tests
  pinning the observable internal shapes listed in
  [07-testing-strategy.md](07-testing-strategy.md). Source: AR-11.

### Won't Have (Out of Scope)

- Other open backlog rows (DEF-76, DEF-80, DEF-90, DEF-92–DEF-99, DEF-103, DEF-104,
  DEF-107–DEF-112).
- Any change under `spec/` (AR-10).
- Refining the conservative runtime-index comparison to index-expression identity — the user chose
  the conservative guard (AR-4).
- Relaxing the generic-instance audit for nullable lend moves into owned positions: the concrete
  return is fixed, the generic spelling stays `E10034`-rejected (AR-3; a documented conservatism).
- Judging a conditional-result store through a lend root (`slot = cond ? slot : other`): a
  verified adjacent defect (`lend_moves_judged` defers on any lend source, so the emitted store
  frees the caller's saved value when the non-slot arm runs). Documented as a known limitation in
  [03-01](03-01-self-assign-rules.md) and filed as `DEF-113` (user ruling at preflight).
- New diagnostic codes, new language surface, or a new verification harness (minimum-sufficient
  baseline, [00-index.md](00-index.md)).

## Technical Requirements

### Compatibility

- No emitted-C ABI change: new emissions are statements inside existing function bodies (the
  bounds-trap line, the slot-empty store) and no symbol or signature changes.
- The accepted-shape emissions of fixed-array element stores, runtime-index stores, and non-null
  element stores stay byte-identical except where R5 requires the new trap line (slice/`str`
  constant-index stores). A fixed-array-only target constructs no guard statement, so its emission
  — including a `for`-step header — is unchanged; a dynamic-bound store inside a `for` step
  degrades the header to the flat label/goto form, as any non-`Assign` step statement does.

### Security

- N/A for input handling — the change surface is compiler-internal. Memory safety is the security
  surface: every accepted shape is probed under host AddressSanitizer (R1–R3, R5; AR-11), and a
  double free, use-after-free, null dereference, or out-of-bounds write is a release blocker.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Batch membership | Store-path five / wider sweeps | The store-path five (DEF-100/101/102/105/106) | Highest severity density, one coherent write/move surface | AR-1 |
| DEF-100 mechanism | New `RequiresSelfAssign` duty / flag on `RequiresStorableSlot` | New duty, following the `Requires*` pattern | Two distinct judgments (self-assign vs any owning store); one report per site | AR-2 |
| DEF-101 direction | Move out / reject `E10034` | Move out | The callee owns the slot value; matches the accepted `let local = v` store and the local return | AR-3 |
| DEF-102 resolution | Conservative guard pinned / index-identity refinement | Conservative guard pinned | The filed premise is stale; the guard is sound and consistent with the root-based rule | AR-4 |
| DEF-105 check | Compile-time narrowing requirement / runtime trap | Compile-time `E10010`, mirroring the read path | The read path already rejects; a trap would diverge | AR-5 |
| DEF-106 emission | Trap statement before the store / route through `StorePointer` | Trap statement | Reuses the read path's guard spelling; avoids broad emission churn | AR-6 |
| Frozen spec | Untouched / clarify | Untouched | Every rule already exists; a spec edit stays a separate ruling | AR-10 |

> **Traceability:** every decision above back-references the register entry that resolved it. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Acceptance Criteria

1. [x] R1–R5 verified by the specification tests ST-1 – ST-30 (P1 extension ST-23 – ST-27, P3
   extension ST-28 – ST-30; [07-testing-strategy.md](07-testing-strategy.md)); the red-phase cases
   fail at the plan baseline and pass after the fix.
2. [x] Every memory-relevant accepted shape is ASan-clean on the host (`RDC_ASAN` probes).
3. [x] `make check` green per phase; `make verify` green before the plan is marked complete.
4. [x] No regression in the existing suites, including the pinned borrow-generic-integrity
   rejections (ST-11, ST-12 guards).
5. [x] Emitted C stays deterministic (byte-identical re-emission), and the review's base-versus-head
   emission diff stays within the plan's modification set (AR-13).
