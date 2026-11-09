# Requirements: Borrow/Generic Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The plan fixes the six remaining defects of the borrow/out-slot and generic-instance machinery
filed by `def-backlog-closeout` (DEF-77, DEF-78, DEF-79, DEF-81, DEF-82, DEF-83). Each defect
has a verified reproduction at the plan baseline
([02-current-state.md](02-current-state.md)); each fix implements an existing rule of the frozen
specification. The plan adds no language surface and no diagnostic code; the one internal
addition is the `RequiresStorableSlot` duty record and its audit, following the existing
`Requires*` pattern (AR-5).

## Functional Requirements

### Must Have

- [x] R1 — A whole-binding store into a non-nullable non-duplicable `borrow` parameter is
  rejected `E10034` (concrete spelling). Source: spec/03 OW-16, spec/06 FN-7 · AR-2, AR-13.
- [x] R2 — The same store in a generic body is rejected `E10034` when any instantiation owns;
  one report per site (the first offending instance). A duplicable instantiation stays accepted.
  Source: spec/03 OW-16 · AR-2, PF-006.
- [x] R3 — The nullable out-slot exemption is preserved: stores into `borrow T?` parameters
  (including the `T[]?` instance) stay accepted. Source: spec/16 out-parameter carve-out · AR-2.
- [x] R4 — `slot = slot` on an out-slot parameter — whole-binding and exact projected
  (`slot[0] = slot[0]`) — is rejected `E10020`, the message the local moved-value rule uses.
  Source: spec/03 E10020 rule · AR-8, PF-005.
- [x] R5 — A nullable `borrow` parameter (out-slot) does not warn `W10001`; a non-nullable
  duplicable lend still does. Source: spec/decisions D9, spec/16 writable-lend rules · AR-9.
- [x] R6 — `Wrap<String>` and `Wrap<String?>` (and every other keyed spelling that carries a
  nullable type argument) produce distinct C symbols; accepted source builds strict. Source:
  spec/07 §9 · AR-4.
- [x] R7 — Top-level function-instance symbol spellings are unchanged by R6; nested nullable
  type arguments gain the `Q` marker (an expected fixing diff — they collide today). Source:
  spec/17 §7 · AR-4, AR-6, PF-003.
- [x] R8 — `==`/`!=` accept a view with `null` in either order, so a nullable view narrows; the
  non-nullable view-versus-`null` form is accepted; slice-view `==` slice-view and ordering stay
  `E10222` (`str`/`static str`/`cstring` already compare — pre-existing, out of scope). Source:
  spec/decisions T8, spec/02 TY-6 · AR-7, PF-010.
- [x] R9 — A func-typed parameter named like a module lender no longer makes the may-lend gate
  leak the owned call result; the result is bound and released once. Source: spec/03 loan rules ·
  AR-3.

### Should Have

- [x] R10 — Each fix ships with a specification test proving the corrected behavior (red where
  the baseline is wrong, green pins where the baseline is already right) and an implementation
  test pinning the observable internal shapes listed in
  [07-testing-strategy.md](07-testing-strategy.md). Source: AR-12, PF-015.

### Won't Have (Out of Scope)

- DEF-76, DEF-80, DEF-90, DEF-92–DEF-95 (other open backlog rows).
- Any change under `spec/` (AR-10).
- The runtime-indexed element self-assign variant (`xs[i] = xs[i]` with a run-time index) —
  noted as a residual and filed at close-out (PF-005).
- New diagnostic codes, new language surface, generalized store-policy layers, or new
  verification harnesses (minimum-sufficient baseline, [00-index.md](00-index.md)).

## Technical Requirements

### Compatibility

- The emitted-C ABI change covers keyed spellings that carry a nullable type argument and
  nested nullable function-instance arguments (DEF-79); top-level function-instance spellings
  stay byte-identical (R7).
- The may-lend fix changes emitted names only for bindings whose source name masks a module
  callee (DEF-77).

### Security

- N/A — the change surface is compiler-internal; there is no input, auth, network, or secret
  handling. Memory safety is covered by the host-ASan probes for the P1 shapes (AR-12).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| DEF-78 rule | Declaration-site + instance duty / other | Declaration-site + instance duty | Matches the existing deferred-duty pattern and keeps legal shapes accepted | AR-2, AR-13 |
| DEF-82 semantics | Reject `E10020` / no-op | Reject | Consistent with the local self-assignment rule | AR-8 |
| DEF-83 exemption | All nullable borrow params / `raw?` only | All nullable borrow params | The copy rationale does not apply to any out-slot | AR-9 |
| DEF-79 key surface | `type_key` / struct-instance key only | `type_key` | Fixes the class of key collisions, not one instance | AR-4 |
| DEF-81 rule | View-vs-`null` / general equality | View-vs-`null` only | Matches T8 and the existing `.ptr` null-test emission | AR-7 |
| DEF-77 mechanism | Name-guard extension / IR callee marker | Name-guard extension | Smallest change; no IR shape change | AR-3 |
| Frozen spec | Untouched / clarify | Untouched | The rules already exist; a spec edit stays a separate ruling | AR-10 |

> **Traceability:** every decision above back-references the register entry that resolved it. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Acceptance Criteria

1. [x] The six verified repros of [02-current-state.md](02-current-state.md) behave as required
   by R1–R9.
2. [x] `compiler/tests/borrow_generic_integrity_spec_test.rs` is green with its red phase
   recorded.
3. [x] `compiler/tests/borrow_generic_integrity_impl_test.rs` is green.
4. [x] Host-ASan probes for the P1 store shapes and the P4 leak shape report no leak, double
   free, or invalid free (`RDC_ASAN=1` in the container; PF-007).
5. [x] `make check` is green after every phase; `make verify` passes every leg at final
   acceptance (AR-12).
6. [x] No change under `spec/` (AR-10).
