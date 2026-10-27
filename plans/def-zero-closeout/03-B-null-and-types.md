# Batch B — Null and Type Rules

> **Document**: 03-B-null-and-types.md
> **Parent**: [Index](00-index.md)
> **Rows**: 16

## Scope

The rows listed below are fixed together because they share an owning stage and regression helpers. Each row keeps its own ST case and its own acceptance evidence.

## Per-Row Specification

### DEF-190 — A nullable call-result widened into a non-nullable view position is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base == head == pre-phase), surfaced by the Phase 4 probes: `func get(): String? ...; let g: str = get();` (a module initializer, or any narrowing-check position fed by a nameless nullable value) is accepted and the widening reads the handle with no test — the frozen `E10010` message template names the source (`value '{name}' may be null here`), and a call result has no name.
- **Fix:** a nameless source needs its own diagnostic shape — a spec/registry change, so it pauses for a user ruling.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-191 — The `unsafe` exemption for the null-use rule differs between sites (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the Phase 4 reviews: the index and store-index null checks exempt `unsafe`, while the conversion (`check_value_into`) and member-read sites report inside it; the frozen TY-6 rule lists no exemption.
- **Fix:** one ruling — extend or drop the exemption at the index sites (a spec-consistency change).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-192 — A qualified value read of a nullable binding is never scrutinized (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the Phase 4 semantics review (N-1): `v.ptr` / `v.len` with `v: u8[]?` and `f.name` with `f: File?` are accepted; `eval_qualified` does not run the member-read null check.
- **Fix:** scrutinize qualified value reads like member reads.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-193 — Narrowing does not survive `&&`/` (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), surfaced by the Phase 4 semantics review (N-2): `if (s != null && len(s) > 0)` reports on the chain and in the guarded body for direct and widened uses alike; the frozen narrowing scope (TY-6) may intend the short-circuit chain to narrow.
- **Fix:** extend the narrowing state through right-hand operands and loop-head conditions.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-195 — `len` of a `cstring` or `usize` is emitted as the constant zero (minor)

- **Observed:** 🟡 minor, pre-existing (base == head == `125f357`), surfaced by the Phase 4 review round: `len(c)` for a `cstring` and `len(u)` for a `usize` are accepted and lowered to the constant `0` (e.g. `len(c) - 2` emits `0 - 2`), outside the frozen SL-2 length surface (arrays, slices, `str`, `cstring`?).
- **Fix:** reject or lower the out-of-surface spellings.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-246 — The literal `null` at a non-nullable aggregate position is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 4 semantics review (SR-002): `P { n: null, x: 1 }`, `i32[2] = [1, null]`, and `ok ? null : 1` are accepted and fail the strict build, where TY-7/E10012, TY-15/E10202, and AL-2 govern.
- **Fix:** judge the aggregate-field, array-element, and conditional-arm positions through the same null gate.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-247 — A member read or store through the null-shaped head is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 4 semantics review (SR-003): `x.len`, `x.foo`, `x.foo = 1`, and `x.len = 1` on a `let x = null;` head are accepted and fail the strict build, where Ch 04 §8 (unknown member is `E10212`) and §11 (`len`) govern; DEF-192 covers qualified reads of nullable bindings and DEF-238 the untyped view members, but not the unresolved head or the store.
- **Fix:** scrutinize qualified reads and stores when the head's type is unresolved.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-248 — A null-shaped value at an arithmetic or comparison operand is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 4 semantics review (SR-004): `x + 1` and `x == 5` on the shape are accepted and fail the strict build, where Ch 04 §5's operator table and TY-13 govern; the same review notes an uncontracted `ffi` aggregate argument accepts the shape (the gate is skipped at `own/call_check.rs:735-749`).
- **Fix:** judge operator operands through the conversion rules, and include the uncontracted aggregate argument.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-249 — A null-shaped value at a condition is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), from the batch G Phase 4 semantics review (SR-005): `if (x)` on the shape is accepted and builds (C pointer truthiness, always false at run time), where Ch 05 §5 requires `bool` with no truthiness (`E10218`).
- **Fix:** report `E10218` for a non-bool condition.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-383 — An inferred call's null literal at a nullable instance is rejected (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-L phase-6 semantics review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (L6S-4): `pair(s, null)` reports `E10012` while the explicit `pair<str?>(s, null)` accepts.
- **Fix:** the literal-null branch (`own/expr.rs:946-951`) consults the resolved parameter, or routes through the deferred record.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-414 — A nullable-gated instance reports a redundant null-arm error after its nullable rejection (minor)

- **Observed:** 🟡 minor, pre-existing pattern (no accepted-program effect), found by the batch-M phase-2 emission audit of [residual-major-completion](residual-major-completion/00-index.md) (P4): `condinstnullable` gains a redundant `E10012` after its existing `E10011` (`T?` drops its nullable flag through `types::substitute`).
- **Fix:** preserve the nullable flag through the substitution or suppress the dependent report.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-417 — TY-4 does not reach every declaration and type position (minor)

- **Observed:** 🟡 minor, pre-existing, found by the batch-M phase-3 correctness and semantics reviews of [residual-major-completion](residual-major-completion/00-index.md) (P-3/L3S-005): `enum E: i32?`, a lambda parameter `(x: i32?)`, `func(i32?)` types, `ffi` signature params, and `sizeof(i32?)` are accepted.
- **Fix:** walk every node of a declared type for TY-4.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-420 — A negated float literal at an f32 destination reports a conversion error (minor)

- **Observed:** 🟡 minor, pre-existing, found by the batch-M phase-3 correctness review of [residual-major-completion](residual-major-completion/00-index.md) (P-2b): `let x: f32 = -5.0;` reports `E10200` while `5.0` accepts.
- **Fix:** propagate the context into the float literal's folded negation (the integer folded-fit rule without the integer fit).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-467 — An out-slot borrow raw self-assign is accepted (minor)

- **Observed:** 🟡 minor, pre-existing, found by the batch-N phase-4 semantics review (PE-6): `func wipe(borrow p: String?): void { p = *(&p); }` is accepted at base and head; the direct self-assign follows the lend rule.
- **Fix:** decide whether a raw self-copy is lend-exempt and pin both spellings.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-483 — The scalar-`len` fold reports `0` for a `u8` argument (minor)

- **Observed:** 🟡 minor, pre-existing current output (not a frozen rule), recorded by the batch-N phase-7 controls: `len(w.items[i][0])` emits `(i32)(0)` and the filed probe runs 254. The frozen view-length rule (Ch 08 SL-2) covers arrays, slices, and `str`, not a scalar; a future scalar-`len` change needs a user ruling.
- **Fix:** Match the concrete twin and the frozen specification; remove the invalid C or wrong value.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-542 — A generic non-nullable view out-slot store is accepted and the write is lost (minor)

- **Observed:** 🟡 minor, pre-existing (PF-010): `func copy<T>(borrow v: T, borrow slot: T) { slot = v; }` with `copy<str>(borrow v, borrow slot)` is accepted; the emitted C passes the slot by value (`ClStr slot`) and assigns it, so the caller's slot is unchanged. The concrete twin reports `E10033`.
- **Fix:** extend the per-instance slot judgment to a non-nullable view substitution, or reject at the instance with the concrete twin's code. Outside T-07's nullable ownership-slot scope, per the PF-010 ruling.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.
