# Current State: Memory and Value Integrity (Batch N)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Baseline**: head `f6da78f` (the batch-M close-out commit)
> **Probe method**: AR-3 — container build; host `target/debug/clc`; strict `cc`; ASan/LSan.
> Line numbers are the head tree; the executor re-resolves them before editing.

## Existing Implementation

### What Exists

Batches I–M built the machinery each row extends. The relevant hooks at `f6da78f`:

- **Loan bookkeeping** (`compiler/src/own/place.rs`): `set_binding_loans` (:1096) drops a
  binding's `element_loans` and replaces its binding-loan union; `replace_element_loans` (:1111)
  replaces one path's loans, keeping a runtime-index union and only adding; `move_element_loans`
  (:1192) snapshots the source positions, strips the read prefix, and re-records them on the
  destination (the snapshot is taken **after** the binding effect); `record_copy_element_loans`
  (:1236) builds the copy union; `collect_copy_element_loans` (:1274) handles conditional arms,
  array literals (including the fill), `Unsafe`/casts; `collect_place_copy_loans` (:1369) reads a
  place's recorded positions with the constant/runtime rule and the prefix fallback, and the
  non-place branch records only `view_loans`; `provenance_loan_owners` (:1435) names the owners a
  value's provenance reaches; `recorded_position_frame_owners` (:975) reads only `element_loans`;
  `place_target` (:1931) has `Name`, `Qualified`, `Member`, `Index`, and `Unsafe` arms — **no
  `Deref`**.
- **Binding effects** (`compiler/src/own/statements.rs`): `bind_value` (:793) prepares and applies
  the initializer/assignment effect; `prepare_binding_effect` (:816) resolves a view loan, a
  move, a conditional move (`ConditionalMove`), or a maybe-owning move (`MoveIfOwning`);
  `apply_binding_effect` (:881) calls `set_binding_loans` for a view and `move_target`/
  `mark_moved_if_owning` for a move; `emit_store` (:422) and `prepare_store` (:445) lower a named
  store and already call `materialize_owning_deref` (:495).
- **Per-instance lend duties** (`compiler/src/own/place.rs:541-590`,
  `compiler/src/own/instance_check.rs:210-260`): `mark_moved_if_owning_at` records a
  `RequiresNotLend` for a parameter whose value is moved; the audit substitutes each instance's
  type and reports `E10034` for the first owning substitution. `note_borrowed_element_move`
  (:629) records an element-move duty only for a `BindingKind::Local` borrow.
  `conditional_move_sources` (`own/expr.rs:175-205`) skips an arm whose type mentions a type
  parameter (:191).
- **Store lowering** (`compiler/src/own/expr.rs`): `check_assign` (:990) handles the raw-deref
  target (`raw_deref_target` :2765) at :1003-1073 and otherwise lowers through
  `place_target`/`emit_assigned` (:2280); `indexed_member_target` (:2361) pins a member target;
  `indexed_target` (:2490) builds a whole-element target and pins only when
  `pin_location` (`:2564`, `contains_update(value)`); `emit_indexed_store` (:2557) stores a whole
  element through `StorePointer` and calls `consume_place` (:2612) for an owned value;
  `emit_indexed_update` (:2639) shares the target builder.
- **Deref materialization** (`compiler/src/own/lower.rs`): `materialize_owning_deref_at`
  (:3699-3721) matches only a top-level `Deref`; `materialize_owned_argument` (:5238) has the
  take-path materializer and the non-take "return the Deref unchanged" rule
  (:5390-5401); `bind_intrinsic_argument` (:4171-4242) binds an owned intrinsic argument without
  that rule.
- **Guarded move-outs** (`compiler/src/own/call.rs:804-821`,
  `compiler/src/codegen/function.rs:2400-2418`): `take_pointee_move_outs` records only a
  top-level `Deref`; `arm_consumes_target` scans `Materialize.move_outs` but not
  `value_move_outs`.
- **Fixed-array stores** (`compiler/src/codegen/function.rs`): `emit_fixed_array_assign_to`
  (:1533) handles the literal/conditional/call forms; the literal `reads_target` path saves and
  clears the destination before the element stores (:1556-1605);
  `emit_fixed_array_conditional_assign` (:1723) branches per arm;
  `struct_literal_needs_stores` (:2071) is the unguarded codegen predicate; the `Statement::Move`
  emission (:659-690) skips an identical source/target only when the source has an `ElementTemp`
  projection.
- **Literal predicates** (`compiler/src/own/lower.rs`): `literal_read_needs_stores` (:4839),
  `literal_field_read_needs_stores` (:4849), and `array_element_read_needs_stores` (:4864) decide
  whether a read literal needs the store form; none handles a `Conditional` value at its own
  level.
- **Division guard** (`compiler/src/codegen/guards.rs`): `division_guard_condition` (:1444-1460)
  reads `left.ty`; `emit_division_guard` (:1435) emits `E11210`.

### Probe Method (AR-3)

```
make -C docker exec CMD="cargo build --workspace"
target/debug/clc --profile host -o out/<name>.c <name>.clo
cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out/<name>.bin out/<name>.c
timeout 30 out/<name>.bin
cc -std=c99 -O0 -g -fsanitize=address -no-pie -o out/<name>.asan out/<name>.c
ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30 out/<name>.asan
```

The probes are reproduced below and recorded under the gitignored
`build/batchN-plan/probes/` tree by the executor; the planning scratch was
`/tmp/opencode/batchN/probes/`. A plain exit that varies because it reads freed storage is
recorded as "varies"; the sanitizer report is the oracle. `cc=1` means the strict C build failed.

## Probe Matrix

| Row | Probe | Emit | Plain run | ASan/LSan | Verdict |
|-----|-------|------|-----------|-----------|---------|
| `DEF-410` | `d410` (filed `condnullhandle1`) | 0 | 134 (double free) | `attempting double-free` | 🔴 red |
| `DEF-410` | `d410b` (concrete twin) | 1 (`E10034` at 6:5) | — | — | control |
| `DEF-421` | `d421a` (`(*p).f`, `(*p).f.x`), `d421b` (`unsafe (*p).f`), `d421c` (`(*p).arr[i].x`) | 0 | 249 / 249 / 251 | clean | 🔴 red |
| `DEF-421` | `d421d` (conditional-value base) | 1 (`E10221` at 10:5) | — | — | control |
| `DEF-423` | `d423a` (fixed-array call RHS), `d423b` (`at1`), `d423c` (`at2`) | 0 | 139 SEGV / 134 / 134 | stack-buffer-overflow / double-free / double-free | 🔴 red |
| `DEF-423` | `d423d` (member route with an updating RHS) | 0 | 0 | clean | control |
| `DEF-424` | `d424a` (`au1`), `d424b` (`ak1`), `d424f` (generic constant self) | 0 | 139 / 139 / 139 | SEGV | 🔴 red |
| `DEF-425` | `d425a` (generic) | 0 | 0 | clean | 🔴 red (accepted) |
| `DEF-425` | `d425b` (concrete borrow), `d425d` (take twin) | 1 (`E10034` 9:23) / 1 (`E10026` 9:23) | — | — | controls |
| `DEF-426` | `d426d` (concrete `/=`/`%=`), `d426e` (generic) | 0 | 0 / 0 | — | guards 3 / **0** |
| `DEF-426` | `d426c` (generic `/0`) | 0 | 136 SIGFPE | FPE | 🔴 red |
| `DEF-427` | `d427p` (filed double index) | 0 | 254 | 254 | 🟡 false positive |
| `DEF-427` | `d427y` (loop alias `len(w.items[i])`), `d427l` (String element) | 0 | 0 / 0 | clean | controls |
| `DEF-428` | `d428a` (`s = s`), `d428b` (`ys = ys`) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-428` | `d428c` (direct fill) | 1 (`E10030` 13:5) | — | — | control |
| `DEF-429` | `d429a` (conditional row), `d429b` (conditional element), `d429c` (array literal) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-430` | `d430b` (runtime store k=1, constant read) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-430` | `d430c`/`d430e` (direct runtime fill), `d430d` (constant store, direct fill) | 0 / 0 / 1 | 0 / 0 / — | clean | control/residual note |
| `DEF-431` | `d431a` (`rows[0] = xs; let s = rows[0][0]`) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-431` | `d431b` (direct fill) | 1 (`E10030` 15:5) | — | — | control |
| `DEF-432` | `d432a` (`[mk().v; "EE"]`) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-432` | `d432b` (direct `fill(mk().v)`) | 1 (`E10030` 12:5) | — | — | control |
| `DEF-433` | `d433a` (`let s = pick(xs)`), `d433b` (`fill(pick(xs))`) | 0 | varies / varies | heap-use-after-free | 🔴 red |
| `DEF-435` | `d435a` (copy `g[1][0]`) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-435` | `d435b` (direct read) | 1 (`E10030` 13:5) | — | — | control |
| `DEF-436` | `d436a` (`xs[i] = *alt(p,q)`) | 0 | 134 | double-free | 🔴 red |
| `DEF-436` | `d436b` (raw-target form) | 0 | 100 (wrong value) | clean | 🔴 red |
| `DEF-437` | `d437a` (store/field/take) | 0 | 134 | heap-use-after-free | 🔴 red |
| `DEF-437` | `d437b` (conditional arm) | 0 | 0 | clean | control |
| `DEF-439` | `d439a` (`g = true`) | 0 | 134 | double-free | 🔴 red |
| `DEF-439` | `d439b` (`g = false`) | 0 | 0 | clean | control |
| `DEF-440` | `d440a` (`while (i < 2 && len(*alt(p,q)) == 2)`) | 0 | 134 | double-free | 🔴 red |
| `DEF-441` | `d441a` (`g && eat(take (*alt(p,q)).name)`) | 0 | 134 | heap-use-after-free | 🔴 red |
| `DEF-445` | `d445a` (read base), `d445b` (nested field) | 0 | `cc=1` | — | 🔴 red |
| `DEF-446` | `d446c` (ternary arm), `d446d` (if-condition control) | 0 / 0 | `cc=1` / 0 | — | 🔴 red / control |
| `DEF-447` | `d447a` (filed), `d447c`, `d447d`, `d447f`, `d447g` | 0 | 0 | **clean** | 🟡 false positive |
| `DEF-448` | `d448c` (`str[2]` places) | 0 | `cc=1` | — | 🔴 red |
| `DEF-448` | `d448a` (`String[2]`), `d448b` (`Box[2]`) | 0 | 0 / 0 | clean | controls |

**Expected post-fix behavior:**

| Row | Verdict | Expected post-fix behavior |
|-----|---------|----------------------------|
| `DEF-428` | 🔴 red | One `E10030` at the fill after the self-assignment; the in-scope read-back runs |
| `DEF-429` | 🔴 red | One `E10030` per affected fill; the copies stay accepted |
| `DEF-430` | 🔴 red | One `E10030` at the constant-read copy's fill |
| `DEF-431` | 🔴 red | One `E10030` at the deeper-projection copy's fill |
| `DEF-432` | 🔴 red | One `E10030` at the temporary-derived element's fill |
| `DEF-433` | 🔴 red | One `E10030` for both the copy and the call-argument forms |
| `DEF-435` | 🔴 red | One `E10030` at the element-store copy's fill |
| `DEF-437` | 🔴 red | Each projection moves once; read-backs correct; ASan/LSan clean |
| `DEF-439` | 🔴 red | Both selections run 0; no double-free of the consumed save |
| `DEF-440` | 🔴 red | No bind/release; the loop runs twice; ASan/LSan clean |
| `DEF-441` | 🔴 red | The guarded take empties the selected member; both selections clean |
| `DEF-421` | 🔴 red | The four `Deref`-rooted store forms emit and read back; the conditional base keeps `E10221` |
| `DEF-423` | 🔴 red | The pinned target stores in bounds; the conditional self-store is a legal move with a correct destination read; `E11211` stays |
| `DEF-424` | 🔴 red | The literal self-read runs before the clear; the constant-index self-copy is skipped |
| `DEF-436` | 🔴 red | One `cl_alt` call; the correct value is stored; ASan/LSan clean |
| `DEF-410` | 🔴 red | One `E10034` per owning instance; the duplicable instance runs |
| `DEF-425` | 🔴 red | One `E10034` per owning borrow instance; the duplicable instance runs |
| `DEF-426` | 🔴 red | The generic and concrete guards agree; the faults trap `E11210` |
| `DEF-445` | 🔴 red | The conditional-of-literals read base builds strictly and runs 0 |
| `DEF-446` | 🔴 red | The ternary-arm take literal builds strictly and runs 0 |
| `DEF-448` | 🔴 red | The conditional array literal of places builds strictly and runs 0 |
| `DEF-427` | 🟡 false positive | The view-length matrix reads 2; the byte double index is the documented shape |
| `DEF-447` | 🟡 false positive | The filed inline read-base literal and variants are LSan-clean |

## Phase 1 — Loan transfer on stored copies

### `DEF-428` — An exact whole-binding self-assignment erases the element loans

**Repro (`d428a`):**

```clorraine
struct Box { v: String; }
func fill(p: str, borrow slot: str?): void { slot = p; }
func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let s: str = xs[0];
    s = s;
    fill(s, borrow slot);
}
```

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free` on the fill read. The
array form (`d428b`, `ys = ys` after `let ys: str[2] = xs;`) is the same. The direct control
`fill(xs[0], ...)` (`d428c`) reports one `E10030` at 13:5.

**Verdict:** 🔴 red. **Fix direction:** snapshot the source positions before the binding effect,
or skip the destination replacement on an exact self-assignment.

**Sites:** `own/statements.rs:793` (`bind_value`), `:951` (`set_binding_loans` before the copy
transfer), `own/place.rs:1096` (`set_binding_loans` drops `element_loans`), `own/expr.rs:1296`
(the snapshot after the effect).

### `DEF-429` — A composite copy source stored into an element or projected destination loses the loans

**Repro (`d429a`/`d429b`/`d429c`):**

```clorraine
// d429a: an element destination
grid[0] = c ? xs : zs;          // grid: str[2][2]; xs/zs: str[2] with xs[0] = a.v
fill(grid[0][0], borrow slot);
// d429b: a projected destination
out2[0] = c ? xs[0] : zs[0];    // out2: str[2]
fill(out2[0], borrow slot);
// d429c: an array-literal source
g[0] = [xs[0], xs[1]];          // g: str[2][2]
fill(g[0][0], borrow slot);
```

**Observed:** all accepted; the plain exit varies; ASan/LSan `heap-use-after-free` at each fill.

**Verdict:** 🔴 red. **Fix direction:** route the element/projected destination value through the
composite-aware collector with its `base` path; write the union once.

**Sites:** `own/expr.rs:1296-1307` (the `projections.is_empty()` gate),
`own/place.rs:1236` (`record_copy_element_loans`), `own/place.rs:1111`
(`replace_element_loans`).

### `DEF-431` — A single-view copy of a deeper projection loses the loan

**Repro (`d431a`):** `rows[0] = xs; let s: str = rows[0][0]; fill(s, borrow slot);` with
`xs[0] = a.v;`. The direct control (`d431b`) reports one `E10030` at 15:5.

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free`.

**Verdict:** 🔴 red. **Fix direction:** add the collector's prefix fallback to
`move_element_loans`.

**Sites:** `own/place.rs:1192` (`path.strip_prefix(source)` only), `own/place.rs:1369-1398`
(the collector's `read.starts_with(path)` fallback).

### `DEF-435` — An element-store nested fill with a fresh member value

**Repro (`d435a`):**

```clorraine
let a: Box = Box { v: stringOf("aa") };
let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
g[1] = [; a.v];
let s: str = g[1][0];
fill(s, borrow slot);
```

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free`. The direct read
(`d435b`, `fill(g[1][0], ...)`) reports one `E10030` at 13:5.

**Verdict:** 🔴 red. **Fix direction:** route the element-store value through the
composite-aware position collector (the `base` parameter is ready).

**Sites:** `own/expr.rs:2557-2621` (`emit_indexed_store`, the `StorePointer` path never calls
the collector), `own/place.rs:1236`.

## Phase 2 — Loan sources through runtime, temporary, and call reads

### `DEF-430` — A runtime-index store followed by a constant-read copy loses the loan

**Repro (`d430b`):** `xs[k] = a.v;` with `k = 1`, then `let s: str = xs[1]; fill(s, ...)`.

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free`. With `k = 0`
(`d430a`) the run is clean because the selected storage is not the loaned one, but the compiler
cannot know that. The direct runtime fill (`d430c`, `k = 1`) is accepted and clean in this probe;
the constant-index control (`d430d`) reports one `E10030` at 13:5.

**Verdict:** 🔴 red (the copy). **Fix direction:** include the binding-union loans in the
constant-read transfer when a runtime record exists. The direct runtime fill is the blast-radius
case (AR-8): record it as an intended rejection if the fix reaches it, or file a residual.

**Sites:** `own/place.rs:1192` (`move_element_loans`), `:1369` (`collect_place_copy_loans`),
`:1111` (`replace_element_loans`, the runtime union rule).

### `DEF-432` — A temporary-derived element in a view-array literal escapes

**Repro (`d432a`):** `let ys: str[2] = [mk().v; "EE"]; fill(ys[0], borrow slot);` with
`mk(): Box { v: String }`.

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free`. The direct
`fill(mk().v, ...)` (`d432b`) reports one `E10030` at 12:5.

**Verdict:** 🔴 red. **Fix direction:** record the provenance owners (including a temp) of a
non-place element/fill value under the destination path.

**Sites:** `own/place.rs:1424-1432` (the non-place branch: only `view_loans`), `:1435`
(`provenance_loan_owners`).

### `DEF-433` — A call-mediated element-view result escapes

**Repro (`d433a`/`d433b`):**

```clorraine
func pick(xs: str[2], i: i32): str { return xs[i]; }
...
xs[0] = a.v;
let s: str = pick(xs, 0);       // d433a
fill(pick(xs, 0), borrow slot); // d433b
```

**Observed:** both accepted; plain exit varies; ASan/LSan `heap-use-after-free`.

**Verdict:** 🔴 red. **Fix direction:** make the cross-frame fallback consult the binding-loan
owners.

**Sites:** `own/call_check.rs:585-620` (`frame_owned_view_owner`), `own/place.rs:975-1017`
(`recorded_position_frame_owners` reads only `element_loans`), `:944-962`
(`view_loan_destination_owners`/`binding_loan_owners`).

## Phase 3 — Raw-deref move-outs

### `DEF-437` — A member projection of a raw pointee is not materialized

**Repro (`d437a`):** `let v: String = (*p).s;`, `Holder { t: (*p).s }`, and
`eat(take (*p).s)` in one `unsafe` block; `p: raw S`, `S { s: String }`.

**Observed:** accepted; plain exit 134 (double free); ASan/LSan `heap-use-after-free`.

**Verdict:** 🔴 red. **Fix direction:** pin a `Deref` anywhere in the projection chain before the
move (or extend the materializer to a projected read).

**Sites:** `own/lower.rs:3699-3721` (`materialize_owning_deref_at` top-level only),
`own/statements.rs:495` (the named-store call). The conditional-arm control (`d437b`,
`g ? (*p).s : stringOf("zz")`) runs 0, matching the batch-M note.

### `DEF-439` — A guarded take through a raw pointer aliasing a conditional store's destination

**Repro (`d439a`):**

```clorraine
let v: String = stringOf("vv");
let g: bool = true;
unsafe {
    let p: raw String = &v;
    v = (g && (eat(take (unsafe *p)) == 1)) ? x : y;
}
```

**Observed:** accepted; plain exit 134 (double free); ASan/LSan `attempting double-free`. The
false path (`d439b`, `g = false`) runs 0, clean.

**Verdict:** 🔴 red. **Fix direction:** consult `value_move_outs` in `arm_consumes_target` (the
runtime address comparison the place channel uses).

**Sites:** `codegen/function.rs:2400-2418` (only `move_outs`), `compiler/src/ir.rs` (the
`Materialize.value_move_outs` field).

### `DEF-440` — A guarded intrinsic or borrow argument binds and releases a raw-deref pointee

**Repro (`d440a`):** `while (i < 2 && (i32(len(*alt(p, q))) == 2)) { ... }`.

**Observed:** accepted; plain exit 134; ASan/LSan `attempting double-free`.

**Verdict:** 🔴 red. **Fix direction:** return the `Deref` unchanged for a borrowing intrinsic
(the non-take rule at `own/lower.rs:5390-5401`).

**Sites:** `own/lower.rs:4171-4242` (`bind_intrinsic_argument`), `:5390-5401` (the rule to
reuse).

### `DEF-441` — A guarded take of a member over a raw pointer leaves the pointee owned

**Repro (`d441a`):** `g && (eat(take (*alt(p, q)).name) == 1)` with `p: raw S`, `q: raw S`,
`S { name: String }`.

**Observed:** accepted; plain exit 134; ASan/LSan `heap-use-after-free`.

**Verdict:** 🔴 red. **Fix direction:** record the pointee move-out for a projection rooted at a
`Deref`.

**Sites:** `own/call.rs:804-821` (only a top-level `Deref` is matched), `own/expr.rs` (the
projection chain), `compiler/src/ir.rs` (`MoveOutValue`).

## Phase 4 — Store lowering and evaluation order

### `DEF-421` — A deref-rooted member store is accepted and silently dropped

**Repro (`d421a`/`d421b`/`d421c`):** `(*p).f = Inner { x: 7 };`, `unsafe (*p).f = ...;`, and
`(*p).arr[i].x = 7;` with `p: raw S`.

**Observed:** accepted; strict build succeeds; the stores are absent; the read-backs run 249 /
249 / 251. The plain forms are base-identical; the `unsafe` spelling is newly accepted by the
batch-M `Unsafe` place arm and still dropped.

**Verdict:** 🔴 red. **Fix direction:** lower the `Deref`-rooted place through the raw-deref
store path (or make `place_target` recurse); make the `None` fallback a debug assertion. The
conditional-value base control (`d421d`) reports one `E10221` at 10:5, unchanged (batch-M AR-44).

**Sites:** `own/place.rs:1931-1976` (no `Deref` arm), `own/expr.rs:1003-1073` (the raw-deref
target matches only a bare `*p`), `:2280` (`emit_assigned`), `:2557` (`emit_indexed_store`).

### `DEF-423` — The whole-element store evaluates the RHS before the target index is pinned

**Repro (`d423a`):** `a[idx] = mkrow();` where `mkrow` sets `idx = 5`; **(`d423b`/`d423c`):**
`a[idx] = set_idx() ? a[idx] : Elem { ... };` where `set_idx` sets `idx = 1`.

**Observed:** `d423a` plain exit 139 (SEGV); ASan `stack-buffer-overflow` at index 5. `d423b`/
`d423c` plain exit 134; ASan `attempting double-free` (the store lands in `a[1]` while `a[0]` is
saved and both own the saved handle). The member-route equivalent (`d423e`) pins the target but
reads the emptied source `a[1]` and SEGVs; its move itself is legal (a[0] receives a[1], a[1] is
emptied), so the spec case reads the destination.

**Verdict:** 🔴 red. **Fix direction:** pin the whole-element base/index before the RHS (the
member route already pins the target). The member-route control (`d423d`, `a[idx].y = set_idx()`
with an updating RHS) runs 0. The conditional self-store is a legal move after the fix; the spec
case reads `a[0]` (the destination) rather than the moved-out source (AR-18).

**Sites:** `own/expr.rs:2490-2556` (`indexed_target`), `:2564` (`pin = contains_update(value)`),
`:2600-2610` (the whole-element path).

### `DEF-424` — The fixed-array literal save and clear run before the RHS

**Repro (`d424a`):** `a[i].names = [stringOf(a[i].names[0]), stringOf("xx")];` with
`names: String[2]`. **(`d424b`):** `a[i].inner = *(&a[i].inner);`. **(`d424f`):** a generic
`same<T>(borrow w: T) { w.items[1] = w.items[1]; }` instantiated owning.

**Observed:** `d424a`/`d424b`/`d424f` plain exit 139 (SEGV); ASan SEGV on a null read. The
concrete constant-index `w.items[1] = w.items[1]` (`d424c`) reports `E10026` (the partial-move
limit); the red constant-index shape is the generic owned-move acceptance.

**Verdict:** 🔴 red. **Fix direction:** evaluate the literal's self-reading RHS before the
drop/clear (or read the saved copy); skip the identical constant-index `Move` (extend batch-M
F3's self-copy rule).

**Sites:** `codegen/function.rs:1556-1605` (the literal `reads_target` save/clear),
`:659-690` (the `Statement::Move` emission, the runtime-only self-copy check),
`own/statements.rs:445-465` (`prepare_store` returns the `Move`).

### `DEF-436` — The whole-element and raw-deref-target stores bypass the owning-deref materializer

**Repro (`d436a`):** `xs[i] = *alt(p, q);` with an impure `alt`; **(`d436b`):**
`unsafe *alt(rp, rq) = *alt(p, q);`.

**Observed:** `d436a` accepted, plain 134, ASan `attempting double-free`; the emitted C calls
`cl_alt` twice. `d436b` accepted, plain 100 (the target is emptied to null instead of receiving
`bb`; `calls = 3`), ASan clean. The filed raw-target double-free is the same re-evaluation
defect; the head's pinned LHS leaves the RHS re-evaluation.

**Verdict:** 🔴 red. **Fix direction:** run the shared materializer on the RHS in both
lowerings.

**Sites:** `own/expr.rs:2599-2613` (`emit_indexed_store` → `consume_place`), `:1060-1070`
(the raw-deref target → `consume_place`), `own/statements.rs:495`
(`materialize_owning_deref` on the named store), `own/lower.rs:3699-3721`.

## Phase 5 — Per-instance generic judgments

### `DEF-410` — An owned conditional store of a generic borrowed parameter double-frees

**Repro (`d410`):**

```clorraine
func pick<T>(c: bool, h: T): i32 {
    let r: String? = c ? h : null;
    if (r == null) { return 1; }
    return 0;
}
func main(): i32 { let h: String = stringOf("ab"); return pick<String>(true, h); }
```

**Observed:** accepted; plain 134 (double free); ASan `attempting double-free`. The concrete twin
(`d410b`) reports one `E10034` at 6:5. The simple generic whole move (`d410c`,
`let r: T = h;`) already reports one `E10034` at 6:5, so the per-instance duty machinery works;
the conditional store is the gap.

**Verdict:** 🔴 red. **Fix direction:** record the conditional arm's maybe-owning source instead
of skipping a type-parameter arm; judge it per instance. The frozen outcome is `E10034` (BR-5),
matching the concrete twin; the note's "caller does not empty" is the accepted-program symptom.

**Sites:** `own/expr.rs:175-205` (`conditional_move_sources`, the `mentions_type_param` skip at
:191), `own/statements.rs:816-875` (`prepare_binding_effect`), `own/place.rs:541-590`
(`mark_moved_if_owning_at`), `own/instance_check.rs:210-260` (`check_not_lend_requirements`).

### `DEF-425` — A generic body accepts an owned element move through a borrow parameter

**Repro (`d425a`):**

```clorraine
struct Elem { name: String; }
struct W { items: Elem[2]; }
func copy<T>(borrow w: T, i: i32, j: i32): void {
    w.items[i].name = w.items[j].name;
}
func main(): i32 {
    let w: W = W { items: [Elem { name: stringOf("aa") }, Elem { name: stringOf("bb") }] };
    copy<W>(borrow w, 0, 1);
    return i32(len(w.items[0].name)) - 2;
}
```

**Observed:** accepted; plain 0; clean. The concrete `borrow` twin (`d425b`) reports one
`E10034` at 9:23; the `take` twin (`d425d`) reports one `E10026` at 9:23.

**Verdict:** 🔴 red (checker conformance). **Fix direction:** record the element move-out through
a borrowed parameter with its projection path and judge it per instance; the owning instance
reports `E10034` (not the take/local `E10026`).

**Sites:** `own/place.rs:629-666` (`note_borrowed_element_move`, `BindingKind::Local` only),
`:541-590` (`mark_moved_if_owning_at`), `own/instance_check.rs:210-260`.

### `DEF-426` — A generic compound division or remainder omits the zero and minimum guard

**Repro (`d426d`/`d426e`):**

```clorraine
struct H { n: i32; items: i32[2]; name: String; }
func go<T>(borrow h: T, d: i32): void {
    h.n /= d;
    h.items[0] %= d;
    h.items[1] /= d;
}
```

**Observed:** the concrete twin (`d426d`) emits three `E11210` guards; the generic (`d426e`)
emits **zero**. `d426c` (`/= 0`) dies with SIGFPE (exit 136; ASan `FPE`). `INT_MIN / -1` and
`INT_MIN % -1` are the same class.

**Verdict:** 🔴 red. **Fix direction:** feed the resolved per-instance operand type into the
division guard.

**Sites:** `codegen/guards.rs:1444-1460` (`division_guard_condition` reads `left.ty`),
`:1435` (`emit_division_guard`), `own/expr.rs:2329-2341` (`combine`), `:2557-2613` (the
compound element path).

## Phase 6 — Literal destinations and conditional arrays

### `DEF-445` — The conditional-of-literals read base is accepted and emits invalid C

**Repro (`d445a`):** `i32((ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0])` with
`n1/n2: str[2]`. **(`d445b`):** the same conditional nested in an `Outer { inner: ... }` field.

**Observed:** accepted; the strict C build fails: `too few arguments to function
'cl_probe_d445a_n1'; expected 1, have 0` (the hidden `str[2]` destination is not bound).

**Verdict:** 🔴 red. **Fix direction:** recurse into the conditional arms in the read/store
predicates.

**Sites:** `own/lower.rs:4839` (`literal_read_needs_stores`), `:4849`
(`literal_field_read_needs_stores`), `:4864` (`array_element_read_needs_stores`),
`codegen/function.rs:2071` (`struct_literal_needs_stores`).

### `DEF-446` — A take conditional literal argument in an eager ternary arm emits invalid C

**Repro (`d446c`):** `g ? consumeO(ok ? Outer { ... n1() ... } : Outer { ... n2() ... }) : 0`.
**Control (`d446d`):** the same literal in an `if`-condition builds and runs 0.

**Observed:** accepted; the strict C build fails: `too few arguments to function
'cl_audit_v379_terntakelit_n1'; expected 1, have 0`.

**Verdict:** 🔴 red. **Fix direction:** route the ternary-arm literal through the guarded
literal store (the mechanism the `if`-condition/phase-7 carrier uses).

**Sites:** `own/lower.rs:5238-5270` (`materialize_owned_argument`), `:1726`
(`materialize_guard_literal`), `own/lower.rs:4943` (`literal_field_needs_stores`).

### `DEF-448` — A take of a conditional array literal of places emits invalid C

**Repro (`d448c`):**

```clorraine
func consume(take xs: str[2]): i32 { return i32(xs[0][0]) - 65; }
func use(ok: bool): i32 {
    let a: str = "AA"; let b: str = "BB"; let c: str = "CC"; let d: str = "DD";
    return consume(take (ok ? [a, b] : [c, d]));
}
```

**Observed:** accepted; the emitted C is `consume((ok ? { a, b } : { c, d }))`; the strict build
fails: `expected expression before '{'` (plus unused-variable errors for the unchosen arm).

**Verdict:** 🔴 red. **Fix direction:** route the conditional array literal through the
element-wise store form (the conditional fixed-array assign machinery). The `String[2]`/`Box[2]`
variants (`d448a`/`d448b`) already run 0 as controls.

**Sites:** `codegen/function.rs:1723` (`emit_fixed_array_conditional_assign`),
`own/lower.rs` (the conditional value lowering), `own/expr.rs:2557` (the take/argument path).

## Phase 7 — Control resolution

### `DEF-427` — A `len` call on a runtime-indexed String element folds to zero

**Repro (`d427p`):** `struct W { items: str[2]; } ... i32(len(w.items[i][0])) - 2` with a
runtime `i`.

**Observed:** accepted; plain 254; ASan 254 (no memory error). The emitted expression is
`(i32)((u32)(i32)(0) - (u32)2)`: `len` returns 0 for the **byte** selected by the second index.

**Controls:** `d427y` (loop alias, `len(w.items[i])`) runs 0; `d427l` (`String` element) runs 0;
`d427a`/`d427e`/`d427i`/`d427j`/`d427m`/`d427q`/`d427r`/`d427t`/`d427u`/`d427v`/`d427w` (the
concrete/generic/borrow/`str`/`String` view-length spellings) all run 0; the single-index member
form `len(w.items[i].name)` runs 0.

**Verdict:** 🟡 **false positive** (AR-27). The double index selects a `u8`; the expected 2
belongs to `len(w.items[i])`, which is green. No code change.

**Sites:** `codegen/calls.rs` (`intrinsic`, the `len` `_ => "0"` fallback), `types/mod.rs`
`index_element` (`Str`/`StaticStr` → `u8`), `own/lower.rs:550` (the fixed-array fold).

### `DEF-447` — An inline read-base owning literal leaks its trailing fields

**Repro (`d447a`):** `G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0][0]` with
`G { rows: str[2][2]; tag: String; }` and `n1(): str[2]`.

**Observed:** accepted; plain 0; ASan/LSan **clean**. The nested-field (`d447f`), `String`-field
(`d447c`), `String[2][2]`-row (`d447d`), and direct-`String` (`d447g`) variants are clean. No ref
that builds leaks.

**Verdict:** 🟡 **false positive** (AR-28). The batch-M phase-7 C1 body-scope read-base binding
is the mechanism the row describes; no reproduction remains. No code change.

**Sites:** `codegen/function.rs` (the read-base statement-temporary path and the phase-7
body-scope binding), `own/lower.rs:4839` (the read predicates).
