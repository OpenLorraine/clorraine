# Testing Strategy: Critical/Major DEF Closeout

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
|---|---|
| Ownership/source-state changes | 100% of the 34 active DEF reproductions and their safe controls |
| Codegen/materialization changes | 100% of the accepted emission shapes; strict C and ASAN |
| Parser/type-table changes | The DEF reproducer plus the frozen stage/golden suites |
| Regression surface | Full `make check` and `make verify`; no red suite at phase close |

- Test names state behavior; in-code comments state the behavior in plain
  language, never a DEF/RD/plan identifier.
- Reuse the existing per-file compile/run/ASAN/strict-C helper pattern; do not
  build a new shared harness. The four new families are four **pairs** (eight
  files): `def_raw_store_integrity_{spec,impl}_test.rs`,
  `def_conditional_value_integrity_{spec,impl}_test.rs`,
  `def_semantic_parity_integrity_{spec,impl}_test.rs`, and
  `def_strict_build_integrity_{spec,impl}_test.rs`. Batch A stays in
  `borrow_store_integrity_*`, which already owns the out-slot rule.
- Internal shape pins that cannot be observed from an integration test (the
  source-union map) live in `compiler/src/own/tests.rs`.
- The plan tracks 35 DEF rows: 34 active plus the stale `DEF-470`. ST-12 is the
  green stale pin. A `DEF-506`/`DEF-507` Phase 0 stale close under AR-16 is also
  a green HEAD pin with its `05-findings.md` proof. Every other ST case is red
  before its fix.

## 🚨 Specification Test Cases (MANDATORY)

> Expected behavior is derived from the concrete twin, the frozen `spec/`
> chapters, and the confirmed register. These are an immutable oracle: a failing
> ST case means the implementation is wrong, never the test. Each row gives the
> concrete expected diagnostic/value/trace; the test file supplies the module
> wrapper, imports, and helper definitions.

### Batch A — Critical Flow

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `func fill(p: str, c: bool, borrow slot: str?): void { let s: str = p; if (c) { s = "AA"; } slot = s; }`, called same-frame from `main` | One `E10034` at the store naming `p`, no C. The local-root twin (`let l: str = "AA"; s = l; slot = s`) reports one `E10030` naming `l`. A mixed parameter+local union reports one `E10034`, not two. A definite overwrite (`let s: str = p; s = "AA"; slot = s`) is accepted and exits 0. A conditional assignment (`s = c ? p : "AA"; slot = s`) reports one `E10034`. A `take` parameter root is accepted; a frame-dying `take` view reports one `E10030`. The static-only and owned-rebind controls exit 0 and are ASAN clean; the accepted controls re-emit byte-identically. The cross-frame call reports once (one `E10034`, no caller `E10030`); a later assignment that adds a foreign source must not suppress the earlier store. A rebound copy of the slot's own value is accepted (the destination-root exclusion); a `take` view through a conditional rebinding reports one `E10030`; an owned `take` value into a `String?` slot is accepted and ASAN clean; a generic `T = str` control is accepted; an all-static conditional-RHS assignment is accepted. | DEF-543; AR-5; BR-5 |

### Batch B — Raw Store Integrity

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-2 | `if (g && consume(H { s: unsafe { (*p).s } }) == 1)` with `consume(take h: H)` and `let p: raw H = unsafe { &h }`; plus the receiver spelling `let r = H { s: unsafe { (*p).s } }.consume(1)` | The pointee is moved, not copied; the skipped-guard control reads the pointee and is LSan clean; strict C builds; exit 0 | DEF-454 |
| ST-3 | `a.items = (g && eatRows(take (unsafe { (*p).items })) == 1) ? x : y` with `let p: raw A = unsafe { &a }` | Take captured; no double-free; ASAN clean; strict C builds; exit 0 | DEF-455 |
| ST-4 | (a) a super-object take into a member destination at a nonzero offset: `struct Pair { head: i32; tail: Elem }`, `pair.tail = g ? (unsafe { *p }) : y` with `p = unsafe { &pair }`; (b) a first-member take into a whole-struct destination with an owning sibling `7` | Both source/destination containment directions are contained; no double-free; the sibling in (b) still reads `7`; ASAN clean; strict C builds; exit 0 | DEF-456 |
| ST-5 | a nested conditional take materialized as a nested call argument: `outer(inner(take (g ? x : y)))`, with an outer and an inner destination | The outer capture survives (capture stack); ASAN clean; strict C builds; exit 0 | DEF-457 |
| ST-6 | `let p: raw String = unsafe { &v }; v = g ? (unsafe { *p }) : y;` and the member twin `a.name = g ? (unsafe { (*p).name }) : y` with `p = unsafe { &a }` | Self-alias recognized; the save is restored into `v` and zeroed; `len(v)` reads the preserved length; ASAN clean; exit 7; strict C builds | DEF-460 |
| ST-7 | `copyInto<S>(unsafe { &a }, a.items)` with a take fixed-array parameter, plus the `unsafe { &a.items }` address-of spelling (adjusted to a `raw` fixed-array parameter) | Alias guarded with the adjusted array address, pinned before the call; ASAN clean; no lost store (the array still reads its original values); strict C builds; exit 0 | DEF-461 |
| ST-8 | `unsafe { (*p) }.next.x = change(unsafe { &c })` where `change` updates `a.next` | Intermediate pinned; the store reaches the pre-call field; strict C builds; exit 3 | DEF-462 |
| ST-9 | `let idx: i32 = 0; func set_idx(): bool { idx = 1; return true; }` then `a[idx] = set_idx() ? a[idx] : Elem { ... }; let s: str = a[1].name` (destination `a[0]`, run-time source `a[1]`) | One `E10020` at the later `a[1].name` read, no C; the destination read `a[0]` stays accepted and exits 0. The destination index is pinned before the condition; the source path is the run-time element the arm reads | DEF-464 |
| ST-10 | `a[0] = [a[0][1], a[0][0]]` with `a[0] = [1, 2]`, and the fill twin `a[0] = [9, 9; a[0][0] + a[0][1]]` at extent 2 | RHS evaluated before the clear; the literal returns `[2, 1]`; the fill twin returns `[3, 3]`; strict C builds; exit 0 | DEF-465 |
| ST-11 | `xs[0] = unsafe { *alt(p, q) }` where `alt` selects `unsafe { &xs[0] }` | Runtime address test; value preserved; no double-free; strict C builds; exit 0 | DEF-466 |
| ST-35 | `v = (g \|\| eat(take v) == 2) ? x : y` with `g = true`, and the `g = false` twin | The skipped right does not zero the save; LSan clean; the taken arm stores the value; strict C builds; exit 0 | DEF-458 |

### Batch C — Conditional Materialization

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-12 | `take h: H; let r: String? = c ? h.name : null; let s = h.name;` | Current HEAD already reports one `E10020`; green stale pin, no code change | DEF-470; AR-7; AR-16 |
| ST-13 | `func go<T>(h: T, d: i32): void { unsafe { (*h.p).n /= d; } }` with `T = H { p: raw Inner }`, `d = 0` | `E11210` guard emitted; run prints `error E11210: integer division by zero` and exits nonzero, not SIGFPE | DEF-471 |
| ST-14 | `func pick<T>(c: bool, take h: T, s: String): i32 { let r: T = c ? h : s; }` at `T = String` | One `E10034` naming `s`, no C; the `T = str` control (`pick<str>(...)`) exits 0 and is ASAN clean | DEF-472 |
| ST-15 | `func pick<T>(c: bool, take h: T): i32 { let s: String = stringOf("cd"); let r: T = c ? h : s; let n = i32(len(s)); return n - 2; }` at `T = String` | One `E10020` at the later `s` use, no C | DEF-473 |
| ST-16 | `(ok ? other : Inner { row: n2(), tag: 7 }).row[0][0]` with an owning `Inner` | Valid C with a bound per-arm destination; exit 3 | DEF-475 |
| ST-17 | `g ? i32((ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]) : 0` | Per-arm store on the evaluated path; strict C; the skipped arm is not called; exit 3 | DEF-476 |
| ST-18 | Nested conditional arms in a literal store and a guarded take: `Outer { tag: tagOf(), inner: Inner { names: ok1 ? n1() : (ok2 ? n2() : n3()) } }` and `consumeT(take (ok1 ? O { names: n1() } : (ok2 ? O { names: n2() } : O { names: n3() })))` | Recursed per-arm stores; strict C; only the selected arm runs; with `ok1 = false, ok2 = true` the trace is exactly `n2`; exit 0 | DEF-477 |
| ST-19 | Guarded `consumeT(take (ok ? O { names: n1() } : O { names: n2() }))` | The skipped arm is not called; the trace is exactly `n1`; exit 2 | DEF-478 |
| ST-20 | Guarded `Outer { tag: tagOf(), inner: Inner { names: n1() } }` and a scalar nested twin `Outer { tag: tagOf(), inner: Inner { n: n2() } }`, with the struct declared `struct Outer { inner: Inner; tag: i32; }` — **declaration order differs from source order** | Source order: `tagOf` then `n1`/`n2`; the trace is exactly `tagOf,n1` (or `tagOf,n2`); exit 0 | DEF-479 |
| ST-21 | `struct Inner { name: String; row: i32[1][1]; }`; `(ok ? Inner { name: stringOf("aa"), row: [[3]] } : mk().inner).row[0][0]` with an owning root | Fresh root bound; the selected member moved; the remaining fields released; `row[0][0]` is `3` when `ok`; LSan clean; exit 3 | DEF-480 |
| ST-22 | Owning conditional read base over call arms: `(ok ? mkA() : mkB()).tag.ptr` where `mkA`/`mkB` return an owning `S` and `tag` is a `str` view field | The selected call root is bound and released once; LSan clean; strict C builds; exit 0 | DEF-481 |

### Batch D — Semantic Parity

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-23 | (a) `if (c ? true : null)`; (b) `if (c ? x : null)` with `x: i32`; (c) `i32(c ? 1 : null)`; (d) `(c ? 1 : null) + 1`; (e) `let y: i32 = c ? 1 : null;`; (f) `let y = c ? h : null` with `h: String`; (g) `for (v in c ? a : null)` with `a: i32[2]` | (a) one `E10012`; (b) one `E10218` (the condition is not bool); (c), (d), (e) one `E10012` each; (f) accepted as a nullable binding; (g) one `E10012` (the existing null rule; the filed row's SEGV is mandatory, not optional). No C for the rejected cases. | DEF-411 |
| ST-24 | `func first<T>(borrow xs: T): str[1] { return [xs[0].name]; }` | Accepted like the concrete twin; exits 0; the existing E10035/E10036 suite is the safety pin (or add the local-backed/duplicate-candidate twins) | DEF-412 |
| ST-25 | `f<u8[2]>` with a nested loop over the `u8` element | No `#element` placeholder; strict C builds; exits 0 | DEF-413 |
| ST-26 | `c ? xs[0].name : xs[0].name` with both arms `String?` at a `str` destination | Two `E10010` (one per nullable arm), no C; the message names `xs` | DEF-415 |
| ST-27 | (a) `func g<T, U>(c: bool, h: T, d: U): U { return c ? h.inner.n : d; }` with `Inner { n: i32 }`; (b) the same with `Inner { n: String }` and `take h`/`take d`; (c) the `h.inner.name` field form | (a) accepted, no `E10200`, no duty; (b) one `E10026` (the concrete nested-move limit), no `E10200`; (c) one concrete-twin `E10034` for the lend form. No C for (b)/(c). | DEF-505 |
| ST-28 | `struct H { name: String; tag: static str; }`; `func viewTag(borrow h: H): str { return h.tag; }`; `let s: str = viewTag(h);` | Revalidate at HEAD; if the exact filed shape is accepted, close stale with a green pin; otherwise accept only with the conservative proof; the name-viewing twin (`return h.name`) still reports one `E10034` | DEF-506 |
| ST-29 | `func always(xs: str[2]): str { return "ZZ"; }` followed by `return always(xs)`, and the slot-store form `slot = always(xs)` | Revalidate at HEAD; the documented safe shapes are accepted, so close stale with a green pin unless the exact filed shape still reports E10035; the unsafe return twin still rejects | DEF-507 |
| ST-30 | `for (borrow e in [make(b.v)]) { return e; }` | One `E10030` at the return, no C | DEF-513 |
| ST-31 | `h1: borrow H`, `h2: borrow H`; (a) `S { p: (d ? h1 : h2).tag.ptr }` (also `.len`, `tag[0]`); (b) the read/return form `return (d ? h1 : h2).tag.ptr;`; (c) a local-backed arm `let l: H = ...; S { p: (d ? h1 : l).tag.ptr }` | (a) accepted; strict C builds; exit 0. (b) accepted; strict C builds; `len` of the selected arm is `2`; exit 0. (c) one `E10030` (the local view dies with the frame), no C | DEF-522 |

### Batch E — Strict Build and Type Identity

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-32 | `let call_shaped = a < b > (c);` with `a`, `b`, `c` all i32 locals (the exact stage-test shape), **and** the non-generic-function form (the `DEF-418` twin); also the free, qualified, dot, and func-typed forms | The parse-only mode accepts the stage parse. A **full compile of the local-i32 shape** reports one `E10300` and writes no C; the non-generic function form also reports one `E10300`; each other non-generic callee form reports one. The stage-spec parse test uses the parse-only mode | DEF-167; AR-14; AR-19 |
| ST-33 | A function-body/local `let data: u8[16] = embed("assets/logo.bin");`, a module `const LEVEL: u8[16] = embed(...)`, a module-level `let palette: u8[64] = embed(...)`, and a non-`embed` constant control | The function-body/local `let` reports only `E10213`, no C; the module `const` reports only `E10224` (staged); the module-level `let` is the R-6 conflict and records the user's ruling; the non-`embed` constant control builds; the E10213 golden is re-enabled and asserted | DEF-171; AR-15 |
| ST-34 | `probe.coll2.a.Thing` (`u8`) and `probe.coll2.b.Thing` (`i32`) declared in two modules; a same-module bare `Thing` control in a separate one-module program | The second declaration reports one `E11101`, no C; the same-module bare `Thing` resolves normally; `qinitstructname` is retargeted from its current success assertion to the collision rejection | DEF-416; AR-20 |

> The executor may split an ST row into several source cases if one program
> cannot express all its controls; the row's expected behavior and source stay
> the oracle. ST-12, and any `DEF-506`/`DEF-507` stale close under AR-16 with a
> HEAD proof, are the green rows. Every other row must be red before its fix.

## Test Categories

### Specification Tests

| Test File | ST Cases | Component |
|---|---|---|
| `borrow_store_integrity_spec_test.rs` | ST-1 | Batch A |
| `def_raw_store_integrity_spec_test.rs` | ST-2…ST-11, ST-35 | Batch B |
| `def_conditional_value_integrity_spec_test.rs` | ST-12…ST-22 | Batch C |
| `def_semantic_parity_integrity_spec_test.rs` | ST-23…ST-31 | Batch D |
| `def_strict_build_integrity_spec_test.rs` | ST-32…ST-34 | Batch E |

### Implementation Tests

| Test File | Description | Priority |
|---|---|---|
| `borrow_store_integrity_impl_test.rs` | source-union pins, cross-frame one-report, determinism; the internal flow-state pin is in `own/tests.rs` | High |
| `def_raw_store_integrity_impl_test.rs` | save/capture shapes, both containment ranges, self-alias restore, strict C | High |
| `def_conditional_value_integrity_impl_test.rs` | per-arm stores, deferred take, source order, LSan counts, read/return C | High |
| `def_semantic_parity_integrity_impl_test.rs` | substituted shapes, one-report counts, concrete/generic C comparison, stale-row pins | High |
| `def_strict_build_integrity_impl_test.rs` | diagnostic shape, type-table collision, parse-only mode, strict C | High |

### Integration and End-to-End

| Test | Components | Description |
|---|---|---|
| `make check` | all stages | Spec check and full Rust workspace |
| `make verify` | host + hosted-amiga + freestanding | Corpus, fault, FS-UAE, strict-C legs |
| `RDC_ASAN=1` focused suites | ownership/codegen | Leak/UAF/double-free probes |
| Determinism/immediacy pins | codegen | Accepted changed output re-emits identically |

## Test Data

### Fixtures Needed

- The concrete-twin source for each of the 34 active DEFs, including `DEF-458`.
- Safe controls for every precision fix (`DEF-412`, `506`, `507`, `522`).
- The unsafe twin for every accepted control (`DEF-412`, `506`, `507`, `522`).
- The E10213 golden (re-enabled) and the parse-only stage case.
- The two-module `Thing` collision fixture and the same-module bare-name control.
- The `DEF-458` skipped-right and taken-right twins.

### Mock Requirements

None. The compiler, C compiler, ASAN, and the existing emulator/corpus legs are
the real test surface.

## Verification Checklist

- [ ] All 35 ST rows have concrete input/output pairs, an exact expected
  diagnostic/value/trace, and a source.
- [ ] A red-phase evidence table (command, base commit, observed failure) is
  recorded in `05-findings.md` for every active row. ST-12 and any
  `DEF-506`/`DEF-507` stale close under AR-16 are the documented green pins and
  carry a HEAD proof instead of a red phase.
- [ ] Every accepted control has an unsafe twin or a named safety pin. Every
  source that takes an address or dereferences a raw pointer writes the exact
  `unsafe { ... }` context, so a red phase fails for the defect and never for
  `E10600`/`E10601`.
- [ ] ASAN/LSan is run for every memory-class row, including `DEF-458` and
  `DEF-464`'s destination-read control.
- [ ] Strict C is run for every emission-class row, including the `DEF-522`
  read/return form and the `DEF-461` address-of spelling.
- [ ] `make check` and `make verify` pass at each phase close.
- [ ] Accepted changed outputs have determinism pins.
- [ ] No spec or golden change without approval (AR-11); the `DEF-167` stage test
  uses the parse-only mode and the `DEF-416` collision is recorded as a
  conservative limitation.
