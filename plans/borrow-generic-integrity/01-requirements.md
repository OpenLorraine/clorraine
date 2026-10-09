# Requirements: Borrow/Generic Integrity

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The plan fixes the six remaining defects of the borrow/out-slot and generic-instance machinery
filed by `def-backlog-closeout` (DEF-78 – DEF-83) plus the may-lend callee-identity defect
(DEF-77). Each defect has a verified reproduction at the plan baseline
([02-current-state.md](02-current-state.md)); each fix implements an existing rule of the frozen
specification. The plan adds no language surface, no diagnostic code, and no support machinery.

## Functional Requirements

### Must Have

- [ ] R1 — A whole-binding store into a non-nullable non-duplicable `borrow` parameter is
  rejected `E10034` (concrete spelling). Source: spec/03 OW-16, spec/06 FN-7 · AR-2, AR-13.
- [ ] R2 — The same store in a generic body is rejected `E10034` for every owning instantiation;
  a duplicable instantiation stays accepted. Source: spec/03 OW-16 · AR-2.
- [ ] R3 — The nullable out-slot exemption is preserved: stores into `borrow T?` parameters
  (including the `T[]?` instance) stay accepted. Source: spec/16 out-parameter carve-out · AR-2.
- [ ] R4 — `slot = slot` on an out-slot parameter is rejected `E10020`, the same diagnostic as a
  local self-assignment. Source: spec/03 E10020 rule · AR-8.
- [ ] R5 — A nullable `borrow` parameter (out-slot) does not warn `W10001`; a non-nullable
  duplicable lend still does. Source: spec/decisions D9, spec/16 writable-lend rules · AR-9.
- [ ] R6 — `Wrap<String>` and `Wrap<String?>` (and every other keyed spelling that carries a
  nullable type argument) produce distinct C symbols; accepted source builds strict. Source:
  spec/07 §9 · AR-4.
- [ ] R7 — Function-instance symbol spellings are unchanged by R6. Source: spec/17 §7 · AR-4,
  AR-6.
- [ ] R8 — `==`/`!=` accept a view with `null` in either order, so a nullable view narrows; the
  non-nullable view-versus-`null` form is accepted; `view == view` and ordering stay `E10222`.
  Source: spec/decisions T8, spec/02 TY-6 · AR-7.
- [ ] R9 — A func-typed parameter named like a module lender no longer makes the may-lend gate
  leak the owned call result; the result is bound and released once. Source: spec/03 loan rules ·
  AR-3.

### Should Have

- [ ] R10 — Each fix ships with a specification test proving the corrected behavior (red before,
  green after) and an implementation test pinning the internal shape. Source: AR-12,
  [07-testing-strategy.md](07-testing-strategy.md).

### Won't Have (Out of Scope)

- DEF-76, DEF-80, DEF-90, DEF-92–DEF-95 (other open backlog rows).
- Any change under `spec/` (AR-10).
- New diagnostic codes, new language surface, generalized store-policy layers, or new
  verification harnesses (minimum-sufficient baseline, [00-index.md](00-index.md)).

## Technical Requirements

### Compatibility

- The emitted-C ABI change is limited to keyed spellings that carry a nullable type argument
  (DEF-79); function-instance symbols stay byte-identical (R7).
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

1. [ ] The six verified repros of [02-current-state.md](02-current-state.md) behave as required
   by R1–R9.
2. [ ] `compiler/tests/borrow_generic_integrity_spec_test.rs` is green with its red phase
   recorded.
3. [ ] `compiler/tests/borrow_generic_integrity_impl_test.rs` is green.
4. [ ] Host-ASan probes for the P1 store shapes report no leak, double free, or invalid free.
5. [ ] `make check` is green after every phase; `make verify` passes every leg at final
   acceptance (AR-12).
6. [ ] No change under `spec/` (AR-10).
