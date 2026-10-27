# Batch D — Borrow and Loans

> **Document**: 03-D-borrow-and-loans.md
> **Parent**: [Index](00-index.md)
> **Rows**: 18

## Scope

The rows listed below are fixed together because they share an owning stage and regression helpers. Each row keeps its own ST case and its own acceptance evidence.

## Per-Row Specification

### DEF-250 — A concrete read-only capture moved at a `let`/assignment site reports `E10034` instead of `E10401` (minor)

- **Observed:** 🟡 minor, pre-existing (base-identical), noted by the batch G Phase 5 implementation (the DEF-160 twin): `let s: String = stringOf("ab"); ([s] () => { let y = s; })();` reports `E10034 cannot release or store borrowed parameter 's'` at the let site where Ch 06 L-3 assigns `E10401`; the phase's fix covers by-value (Copy) captures, while a non-duplicable concrete capture is a read-only borrow whose move `reject_capture_move` only covers at the return/take/expression-body sites.
- **Fix:** route the borrow-capture move through the capture rule at the `let`/assignment/composite-field sites as well (the lend duty is the safety net for uncovered positions, so the sites must move together); the batch G Phase 5 fix round adds the writable `[borrow s]` spelling to the same class.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-252 — Moving out of a `const` binding is accepted (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch G Phase 5 semantics review while probing the dedupe (PRF-4): `const s: String = stringOf("ab"); let t: String = s;` is accepted, where Ch 03 §2 V3 says moving out of a `const` is an error.
- **Fix:** report the move-out-of-const rule at the consuming site.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-311 — The residual-root hook misses the operand and guarded read positions (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-J phase-1 correctness review of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md) (J1R-2): `1 + (c ? b.n : mkBox(7).n)` leaks 11 B and the guarded read `g && i32((c ? mk() : b).ys[0]) > 0` leaks 12 B; the phase-1 root-fill hook covers the unguarded positions.
- **Fix:** extend the hook to the operand and guarded positions.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-312 — The element-carrier loan conflict is owner-granular (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-J phase-1 correctness review of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md) (J1R-4): `Holder { names: [b.w] }` then a write to `b.v` falsely reports `E10031` (the loan is keyed on the owner, not the field path).
- **Fix:** key the conflict on the member path.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-354 — A two-root conditional capture store reports `E10401` twice (minor)

- **Observed:** 🟡 minor, pre-existing (both sides; probe `p5_capture_cond_copy`), found by the batch-K phase-5 memory audit of [residual-major-closeout](residual-major-closeout/00-index.md): a conditional store whose two arms both move a distinct capture reports two `E10401` where the one-report rule asks for one per site.
- **Fix:** the per-span dedupe at the capture-store duty.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-370 — A write-through-borrow callee makes a move-only array type read as duplicable (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-L phase-3 semantics review of [residual-major-closeout-ii](residual-major-closeout-ii/00-index.md) (P-3): the own-stage field list drops an owning field (`fields_S=Some(["y:i32"])` for a `String` field), so `duplicable(S[2])` reads true and a move-only array lend warns `W10001` spuriously (`w2`/`w4`/`u3`/`d8`). Cosmetic so far.
- **Fix:** the own-stage field list must carry every owning field.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-422 — A standalone raw-prefix store is rejected although the prefix is the unsafe context (minor)

- **Observed:** 🟡 minor, pre-existing (base `E10221`; head `E10600` on the same shape), found by the batch-M phase-4 correctness review of [residual-major-completion](residual-major-completion/00-index.md) (P-2): `unsafe p.f = ...` with `p` from a call (`getp()`) reports `E10600`, while the block form is accepted; `check_place_raw` does not treat the prefix as the unsafe context.
- **Fix:** treat the prefix as the enclosing unsafe context for the raw store check.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-434 — A direct-owner literal overwritten at its element still reports through the provenance fallback (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-M phase-5 correctness review of [residual-major-completion](residual-major-completion/00-index.md) (P-3): `[a.v, "EE"]` overwritten at `[0]` still reports `E10030` via the binding provenance fallback (`own/place.rs:1395-1402`); the owners branch is unpinned.
- **Fix:** pin the overwrite expectation and give the fallback the same position authority.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-444 — A place-deref take in a loop head moves the same pointee twice without a report (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-M phase-6 correctness review of [residual-major-completion](residual-major-completion/00-index.md) (L6C-4): `loop_place_take` moves the same pointee on the second iteration with no use-after-move report.
- **Fix:** record the loop-head move-out per iteration (the semantics backlog).
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-463 — The raw-store RHS pointer binding reports a false unused-local warning (minor)

- **Observed:** 🟡 minor, pre-existing root (the bare `*p = *q` form warns at base), found by the batch-N phase-4 correctness review: the member/element raw route lowers the RHS pointer without marking the binding used, so `let q: raw S = &a; (*p).items = (*q).items;` emits `W10000: unused local 'q'`.
- **Fix:** mark the raw RHS pointer bindings used at the store.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-474 — A generic view conditional return diverges from the concrete twin (minor)

- **Observed:** 🟡 minor, pre-existing, found by the batch-N phase-5 emission review: `return c ? h : h` from a borrow parameter as a view returns E10036 in the generic body while the concrete twin accepts; no TU.
- **Fix:** align the view-return elision for a type-parameter view target.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-484 — A local static view carrier through a loop alias is falsely rejected (minor)

- **Observed:** 🟡 minor, phase-1 over-rejection (base accepts and runs clean): `let hs: Holder[1] = [Holder { names: ["AA","BB"] }]; for (borrow e in hs) { store(e, borrow slot); }` reports `E10030: borrow of 'hs' does not live long enough`; the alias element's known-empty static positions are not visible to the frame judgment.
- **Fix:** resolve the alias root's recorded positions in the judgment and pin the control.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-490 — An inline static aggregate literal is falsely rejected (minor)

- **Observed:** 🟡 minor, phase-1 over-rejection (base and `5741d5c` accept and run clean): `store(Holder { names: ["AA","BB"] }, borrow slot)` reports `E10030` naming a statement temporary; the inline literal's static positions are not recognized.
- **Fix:** treat the inline literal as a frame literal with known static positions.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-491 — A frame-backed literal arm reports an extra `E10033` (minor)

- **Observed:** 🟡 minor, pre-existing (base and head): `c ? h3 : Holder { names: [b.v, "EE"] }` reports `E10033` at the literal plus `E10031` at the write, while the plain literal twin reports only the write.
- **Fix:** judge a conditional literal arm as a frame literal when its destination is a local.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-492 — A zero-argument static factory carrier is falsely rejected (minor)

- **Observed:** 🟡 minor, phase-1 over-rejection (base accepts and runs clean): `makeStatic()` returning `Holder { names: ["AA","BB"] }` is rejected `E10030` when bound and stored; the call-returned carrier has no view-like argument to establish a safe source.
- **Fix:** consult the callee's returned positions or document the conservative rejection.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-494 — A taken view parameter reports twice (minor)

- **Observed:** 🟡 minor, pre-existing (base and head): `take p: str` called with a view reports `p` at the callee store and the caller expression at the call; the aggregate take reports once.
- **Fix:** keep one site for a view take or record the duplicate as the intended two-site rule.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-524 — Two distinct later-use sites with the same moved-path set share one report (minor)

- **Observed:** 🟡 minor, pre-existing: two `sink(h)` sites in one loop with the same moved-path set report once for the generic body while the concrete twin reports twice; the recording key is per binding and moved-path set.
- **Fix:** key by the enclosing statement span as well, merging sub-expression visits of one statement.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-530 — Two carrier return sites report one escape (minor)

- **Observed:** 🟡 minor, pre-existing: a frame-backed opaque store followed by `if (c) { return h; } return h;` reports one `E10030` while the concrete twin reports each return site; the duty keeps one `return_span`.
- **Fix:** record a list of distinct return spans per duty and report each. Found by the batch-R phase-1 correctness review.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.
