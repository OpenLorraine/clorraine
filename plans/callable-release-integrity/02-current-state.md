# Current State: Callable Release Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

All 13 defects are pre-existing at the current head (`1fe9e50`); every one carries an independent
reproduction and base-identical evidence in `plans/eval-emission-integrity/05-findings.md` (the
per-finding citations are in the register's PD blocks). The compiler already contains every piece
of machinery the fixes need: generated owners with a rename precedent, signature→mode propagation
for func-typed bindings (the DEF-125 fix), scoped owners, guarded materialization, the type-key
renderer, cleanup edges, and the view-return elision postlude. No new module or pass is required
(AR-15).

### Per-defect state

| Defect | Current behavior (filed evidence) | Fix locus (recon) |
| ------ | --------------------------------- | ----------------- |
| DEF-138 🔴 | A source local spelled `__cl_owned0` and a later generated owner collide in the name-keyed binding lookup; the accepted program double-frees (ASan) | Generation `own/lower.rs:1839`, `:1881`; name map `own.rs:799` (`declare_local`), `:920` (`lookup`); scope transfer `own/lower.rs:1908`; C-rename precedent `05-findings.md:99` |
| DEF-139 🟠 | A func-field call through a temporary receiver (`H{…}.call(5)`, `makeH().call(5)`) never binds the receiver root; its owned sibling fields leak | Receiver extraction `own/call.rs:183`; ownerless-root binder `own/lower.rs:1411`; member-receiver bind `own/call.rs:194` |
| DEF-140 🟠 | `TyKind::Func(_)` keys as `fn`, so `Holder<func(i32): i32>` and `Holder<func(u32): u32>` both spell `ClHolder__fn`; the unit redefines the struct | `types/keys.rs:66` (the single key owner) |
| DEF-141 🟠 | `len(s)` on a null `String` reads the handle through the widening and segfaults; the accepted program has no narrowing report | Null-use check `own/expr.rs:481`; widening sites `sema/call.rs:68`, `codegen/calls.rs:212`, `codegen/function.rs:1926`, `codegen/program.rs:119` |
| DEF-155 🟠 | A func-typed field call passes `modes: None` (the receiver branch), so a `take` field call double-frees, a `borrow` field call records no exclusive lend, and a plain field call leaks its owned temporary argument | Mode branches `own/call.rs:219-235` (receiver ⇒ `None`); field receiver `own/call.rs:183` |
| DEF-156 🟠 | A module-level func-typed global is not resolved as a callee (source `lookup` sees no globals); the call keeps the no-effects path and double-frees on `take` | Callee lookup `own/call.rs:165-182`; `callee_modes` fallback `own/call.rs:231` |
| DEF-157 🟠 | A `Param`-typed callee yields `modes: None` (`own/call.rs:229`), so an owned temporary argument leaks; a parameter shadowing a same-named `take` module function reports a false `E10020` through name-based fallback | `own/call.rs:221-232`; `own/call_check.rs:243` (`func_binding_callee`) |
| DEF-158 🟠 | `check_defer` lowers the body with `materialize_owned_temporaries = false`, so view bindings and func-value temporaries inside `defer` leak | `own/statements.rs:846-847` (suppression); region/cleanup handling `:857-873` |
| DEF-159 🟠 | A conditional view projection with a place-rooted arm keeps the conditional inline (`conditional_borrows_place`), so the taken call arm's backing and composite siblings leak; only the both-call spelling owns | `own/lower.rs:1533-1545`; guarded sibling `bind_guarded_projection_root` `own/lower.rs:1562` |
| DEF-163 🟡 | A fixed array of view elements returned from a temporary is not "an elided view", so the elision/escape checks skip it and the backings leak | `own/place.rs:1114-1128` (`elision_postlude`); `borrow.rs:66` (`is_elided_view`) |
| DEF-164 🟠 | A discarded owned call statement emits the bare call; the result is never registered for release (edge copies cover statement temporaries, not this shape) | Pending-release copies `own/statements.rs:895`; the expression-statement lowering path |
| DEF-165 🟠 | An inline capturing lambda emits `struct ClEnv_0` at two emission sites, so the unit redefines it and the strict build fails | Names/sites `codegen/function.rs:275-276`, `codegen/program.rs:429-430`, `codegen/expr.rs:845` |
| DEF-166 🟠 | A capture-less generic lambda called with a by-value owning argument at an owning instance double-frees; the root is a lambda parameter, which neither the capture walk nor the call path judges per instance | Closure-call lowering/modes (`own/call.rs` value-call path built for DEF-125); instance judging (`own/instance_check.rs`) |

## Gaps Identified

### Gap 1: Generated identities are name-keyed (DEF-138, DEF-140, DEF-165)

**Current Behavior:** generated bindings, type keys, and env structs are identified by spellings
that can collide with each other or with source code.
**Required Behavior:** generated identities are unique by construction (R1, R3, R12).
**Fix Required:** identity-keyed generated bindings, a signature-bearing func key, and env-struct
dedupe (03-01).

### Gap 2: A callee's own signature does not reach its call (DEF-155 – DEF-157, DEF-166)

**Current Behavior:** field receivers, module func globals, `Param` callees, and lambda parameters
lose their modes; resolution can fall back to a wrong same-named module function.
**Required Behavior:** every callee kind applies its own signature's modes, per instance for
generics (R5 – R7, R13).
**Fix Required:** thread the signature through every callee kind and keep resolution identity-based
(03-02).

### Gap 3: Five owned-release holes (DEF-139, DEF-158, DEF-159, DEF-163, DEF-164)

**Current Behavior:** temporary receivers, defer bodies, mixed conditional view arms, view-element
arrays, and discarded call statements bypass the owner/release machinery.
**Required Behavior:** every created owner releases exactly once; the array-return spelling reports
what the single-view spelling reports (R2, R8, R9, R10, R11).
**Fix Required:** extend the existing binder/cleanup/prelude machinery to each hole (03-03).

### Gap 4: The possibly-null widening bypasses the narrowing check (DEF-141)

**Current Behavior:** the implicit `String`→`str` widening reads a null handle; the use is
accepted.
**Required Behavior:** the widening is a use; a possibly-null operand reports `E10010` unless
narrowed (R4).
**Fix Required:** run the narrowing-state check at the widening use (03-04).

## Dependencies

### Internal Dependencies

- The Phase-5 owner machinery (scoped owners, guarded materialize, cleanup edges) — reused as-is.
- The DEF-125 signature→mode propagation — extended to the remaining callee kinds.
- The type-key renderer and the monomorphization identity — one key owner (`types/keys.rs`).
- The `E10036` postlude and the escape checks — extended to array-of-view returns.

### External Dependencies

- None. Host ASan is the verification tool for the release shapes (available in the pinned
  container; `RDC_ASAN=1`).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Identity-keying (138) touches the binding-map contract | Med | Med | Keep `declare_local`/`lookup` semantics for source bindings; add generated bindings outside the source map; memory auditor probes variants |
| A new owner shape double-releases | Low | High | ASan probes per shape; emission auditor's corpus diff; one release per creation invariant |
| Per-instance mode judging misses a concrete spelling | Med | Med | Every generic ST case paired with its concrete twin control |
| 140's symbol-name change desynchronizes ownership and emission | Low | Med | Single key owner; emission auditor's base-vs-head diff |
| 163's rejection reports a different code than the single-view spelling | Low | Low | The ST asserts diagnostic parity between the two spellings; a new code pauses to the user |
