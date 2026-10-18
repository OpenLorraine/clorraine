# Current State: Memory and Value Integrity (Batch N)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)
> **Baseline**: head `100180c` (the plan commit; the compiler sources are unchanged since the
> batch-M close-out `f6da78f`)
> **Probe method**: AR-3 — container build; host `target/debug/clc`; strict `cc`; ASan/LSan.
> Line numbers are the head tree; the executor re-resolves them before editing.

## Existing Implementation

### What Exists

Batches I–M built the machinery each row extends. The relevant hooks at `f6da78f`:

- **Loan bookkeeping** (`compiler/src/own/place.rs`): `set_binding_loans` (:1096) drops a
  binding's `element_loans` and replaces its binding-loan union; `replace_element_loans` (:1111)
  replaces one path's loans and, for a non-constant path, keeps the runtime union and only adds
  (the union branch is :1119-1127) — the fix also records the wildcard `Element(None)` path there;
  `move_element_loans` (:1192) snapshots the source positions, strips the read prefix
  (`path.strip_prefix(source)` at :1212, made wildcard-aware), and re-records them on the
  destination (the snapshot is taken **after** the binding effect);
  `record_copy_element_loans` (:1236) builds the copy union; `collect_copy_element_loans` (:1274)
  handles conditional arms, array literals (including the fill), `Unsafe`/casts;
  `collect_place_copy_loans` (:1369) reads a place's recorded positions with the constant/runtime
  rule and the prefix fallback, where a recorded `Element(None)` step matches any `Element(_)` at
  the same position prefix, and the non-place branch records only `view_loans` (it receives only
  `&Expr`, not the lowered value); `place_recorded_position_owners` (:913) reads a place's recorded
  position owners with the same wildcard match; `view_loans` (:1512) is the loan-chain mirror;
  `view_owners` (:1569) and `lowered_root_bindings` (:1603) name a lowered value's temp/root
  owners; `provenance_loan_owners` (:1435) names the owners a value's provenance reaches;
  `recorded_position_frame_owners` (:975) reads only `element_loans`, with the same wildcard match;
  `loan_owners` (:815) expands a loan set; `record_element_view_loans` (`own/expr.rs:1679`) records
  the backing loans a view element store gives its container; `place_target` (:1931) has `Name`,
  `Qualified`, `Member`, `Index`, and `Unsafe` arms — **no `Deref`**.
- **Binding effects** (`compiler/src/own/statements.rs`): `bind_value` (:793) prepares and applies
  the initializer/assignment effect; `prepare_binding_effect` (:816) selects a view loan, a move, a
  conditional move (`ConditionalMove`), or a maybe-owning move (`MoveIfOwning`); this is where the
  `DEF-410` effect is chosen. `apply_binding_effect` (:881) calls `set_binding_loans` for a view
  and `move_target`/`mark_moved_if_owning` for a move; `emit_store` (:422) and `prepare_store`
  (:445) lower a named store and call `materialize_owning_deref` at :502.
- **Per-instance lend duties** (`compiler/src/own/place.rs:541-590`,
  `compiler/src/own/instance_check.rs:210-260`): `mark_moved_if_owning_at` records a
  `RequiresNotLend` for a parameter whose value is moved; the audit substitutes each instance's
  type and reports `E10034` for the first owning substitution. `note_borrowed_element_move`
  (:629) records an element-move duty only for a `BindingKind::Local` borrow.
  `conditional_move_sources` (`own/expr.rs:175-205`) skips an arm whose type mentions a type
  parameter (:191).
- **Store lowering** (`compiler/src/own/expr.rs`): `check_assign` (:990) handles the raw-deref
  target (`raw_deref_target` :2765) at :1003-1073 and otherwise lowers through
  `place_target`/`emit_assigned` (:2280); `indexed_member_target` (:2361) pins a member target
  through `pin_store_target` (:2393), which materializes every element base/index in the chain;
  `indexed_target` (:2490) builds a whole-element target and pins only when `pin_location`
  (`contains_update(value)` at :2566); `emit_indexed_store` (:2557) stores a whole element through
  `StorePointer` and calls `consume_place` (:2621) for an owned value; `emit_indexed_update`
  (:2639) shares the target builder.
- **Deref materialization** (`compiler/src/own/lower.rs`): `materialize_owning_deref_at`
  (:3699-3721) matches only a top-level `Deref`; `materialize_owned_argument` (:5238) has the
  take-path pointer pin (:5239-5262) and the non-take "return the Deref unchanged" rule
  (:5393-5401); `bind_intrinsic_argument` (:4171-4234) binds an owned intrinsic argument without
  that rule. The take-path pin walks only a top-level `Deref`, not a projection chain.
- **Guarded move-outs** (`compiler/src/own/call.rs:804-821`,
  `compiler/src/codegen/function.rs:2400-2444`): `take_pointee_move_outs` records only a
  top-level `Deref`; `arm_consumes_target` scans `Materialize.move_outs` but not
  `value_move_outs`, and returns a bool, so it cannot carry a runtime alias comparison.
  `moves_pointee` (`codegen/function.rs:1382-1384`) and `return_source_needs_emptying`
  (`codegen/cleanup.rs:545-567`) likewise match only a top-level `Deref`.
- **Fixed-array stores** (`compiler/src/codegen/function.rs`): `emit_fixed_array_assign_to`
  (:1533) handles the literal/conditional/call forms; the literal `reads_target` path saves and
  clears the destination before the element stores (:1557-1592);
  `emit_fixed_array_conditional_assign` (:1723) branches per arm;
  `struct_literal_needs_stores` (:2071) is the unguarded codegen predicate and its field loop
  (:1421-1432) empties only a top-level `Deref`; the `Statement::Move` emission (:659-690) skips
  an identical source/target only when the source has an `ElementTemp` projection.
- **Literal predicates** (`compiler/src/own/lower.rs`): `literal_read_needs_stores` (:4839)
  accepts only a `StructLit` and returns false for a `Conditional`; `literal_field_read_needs_stores`
  (:4849) returns true for a fixed-array field that is not an `ArrayLit` (a `Conditional` field
  therefore already needs the store form); `array_element_read_needs_stores` (:4864) recurses only
  through nested literals. Only `literal_read_needs_stores` returns false for a `Conditional` at
  its own level; the OR rule (any arm needs the store form) is the planned change.
- **Division guard** (`compiler/src/codegen/guards.rs`): `division_guard_condition` (:1444-1460)
  already calls `self.substitute(&left.ty)`; the gap is an unresolved projection type, which
  `resolved_place_ty` (`codegen/expr.rs:321`) and `resolved_value_ty` (`:397`) resolve through the
  base and projections. `emit_division_guard` (:1435) emits `E11210`.

### Probe Method (AR-3)

```
make -C docker exec CMD="cargo build --workspace"
target/debug/clc --profile host -o out/<name>.c <name>.clo
cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out/<name>.bin out/<name>.c
timeout 30 out/<name>.bin
cc -std=c99 -O0 -g -fsanitize=address -no-pie -o out/<name>.asan out/<name>.c
ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30 out/<name>.asan
```

The probes are reproduced below and preserved under the gitignored
`build/batchN-plan/probes/` tree: the planning probes, the preflight re-probes
(`preflight-N/`), and the iteration-2 re-run (`apply/`). The planning scratch was
`/tmp/opencode/batchN/probes/`; the preflight scratch was `/tmp/opencode/preflight-N/`. A plain
exit that varies because it reads freed storage is recorded as "varies"; the sanitizer report is
the oracle. `cc=1` means the strict C build failed.

## Re-run record (preflight revision)

Every probe below was re-emitted with `target/debug/clc` at `100180c` and run plain and under
`ASAN_OPTIONS=detect_leaks=1:symbolize=0`. The UAF exits vary with allocator state; the ASan
report is the oracle.

| Probe | Shape | Emit | Plain run | ASan/LSan | Observation |
|-------|-------|------|-----------|-----------|-------------|
| `d425a` | generic `copy<T>(borrow w: T) { w.items[i].name = w.items[j].name; }` | 0 | 0 | clean | **accepted** (the park's conflict) |
| `d425b` | concrete `borrow w: W` twin | 1 | — | — | `E10034` |
| `d425d` | concrete `take w: W` twin | 1 | — | — | `E10026` |
| `p429_runtime_dest` | `grid[k] = c ? xs : zs; fill(grid[1][0])` | 0 | varies | heap-use-after-free | red; the runtime branch only unions `state.loans` and records no element position (`own/place.rs:1119-1127`) |
| `q429runtime_copy` | the same runtime store, then `let s = grid[1][0]; fill(s)` | 0 | varies | heap-use-after-free | red |
| `p430_sibling` | `ys[0] = a.v; let s = ys[1]; fill(s)` | 0 | 0 | clean | **accepted + clean** constant-store sibling control |
| `q435constant_copy` | `g[1] = [; a.v]; let s = g[1][0]; fill(s)` | 0 | varies | heap-use-after-free | red; constant destination is gated by `projections.is_empty()` |
| `q435runtime_copy` | `g[k] = [; a.v]; let s = g[1][0]; fill(s)` | 0 | varies | heap-use-after-free | red; the runtime element store never calls the collector |
| `q435runtime_direct` | `g[k] = [; a.v]; fill(g[1][0])` | 0 | varies | heap-use-after-free | red direct read |
| `p437_store_only` | `let v: String = (*p).s` | 0 | 134 | attempting double-free | red |
| `p437_return` | `return (*p).s` | 0 | 134 | attempting double-free | red |
| `p437_field_only` | `Holder { t: (*p).s }` | 0 | 134 | attempting double-free | red |
| `q441base` | `g && eat(take (*alt(p,q)).name)` | 0 | 134 | attempting double-free | red |
| `q441sibling` | the same take, then read the sibling member | 0 | 134 | attempting double-free | red; the root-wide empty loses the sibling |
| `q439alias_true` | `g = true`, `p = &v` | 0 | 134 | attempting double-free | red |
| `q439alias_false` | `g = false`, `p = &v` | 0 | 0 | clean | control (equal lengths; the distinct-length re-run below shows the false arm stores `y`) |
| `q439nonalias` | `g = true`, `p = &w` (non-aliasing) | 0 | 0 | clean | control; the save must drop, not zero |
| `q430sibling` | `xs[0] = a.v; let s = xs[1]; fill(s)` | 0 | 0 | clean | sibling acceptance control (same as `p430_sibling`) |
| `q424self` | `x = *(&x)` | 0 | 139 | SEGV | red self-alias |
| `q424inner` | `a[i].inner = *(&a[i].inner)` (`d424b`) | 0 | 139 | SEGV | red self-alias |
| `q424two` | two-element fixed-array self-read | 0 | 139 | SEGV | red |
| `q424generic` | generic constant-index `w.items[1] = w.items[1]` (`d424f`) | 0 | 139 | SEGV | red |
| `q445mixed` | `(ok ? inner : Inner { row: n2() }).row[0][0]` | 0 | `cc=1` | — | red; the literal arm keeps a hidden destination (`too few arguments` class) |
| `p445_mixed_bothlit` | both arms literals | 0 | `cc=1` | — | red |
| `q440borrow` | `read((*alt(p,q)).s)` with a borrowing parameter | 0 | 134 | attempting double-free | red |
| `q423bump` | `(*p).arr[idx].y = bump()` | 0 | 249 | clean | red; the store is dropped (DEF-421) and the index is not pinned |
| `q423at1` | `at1` reads `a[1]` after the conditional store | 0 | 134 | attempting double-free | accepted source read (residual row) |
| `q410dup` | generic whole move `let r: T = h` at `T = P` (duplicable) | 0 | 0 | clean | duplicable control |
| `q427p` | `len(w.items[i][0])` | 0 | 254 | 254 (no memory error) | false positive; emitted `len` is the scalar fallback `0` |
| `q447a` | filed inline read-base literal | 0 | 0 | clean | false positive (LSan clean) |

The preflight `p440_borrowarg` (`read((*p).s)` with a concrete pointer) and `p440_member`
(`len((*p).s)`) are clean; the red borrow-argument form is the impure `alt` variant above. The
preflight `p429_direct`/`p429_direct2` (constant destination, direct fill) are accepted and
ASan-red at the head because the conditional store recorded nothing; after the `DEF-429` gate they
become the direct-fill controls and must report `E10030`.

### Iteration-2 re-run (this revision)

The eight probes named by the iteration-2 corrections were re-emitted and run with the same
method. `q439alias_false` now uses distinct lengths (`v = "vv"`, `y = "yyyy"`) so the false arm's
store of `y` is visible; the earlier equal-length probe ran 0 and hid it.

| Probe | Shape | Emit | Plain run | ASan/LSan | Observation |
|-------|-------|------|-----------|-----------|-------------|
| `q424self` | `x = *(&x)` | 0 | 139 | ASan SEGV | red; the raw self-alias has no `place_target` |
| `q424inner` | `a[i].inner = *(&a[i].inner)` (`d424b`) | 0 | 139 | ASan SEGV | red; the same raw self-alias through a nested field |
| `q429runtime_copy` | `grid[k] = c ? xs : zs; let s = grid[1][0]; fill(s)` | 0 | 242 (varies) | heap-use-after-free | red; no recorded element position |
| `q429runtime_direct` | the same runtime store, then `fill(grid[1][0])` | 0 | 36 (varies) | heap-use-after-free | red; the direct runtime fill is the `DEF-429` blast radius |
| `q430sibling` | `xs[0] = a.v; let s = xs[1]; fill(s)` | 0 | 0 | clean | **accepted + clean** sibling acceptance control |
| `q439alias_false` | `g = false`, `p = &v`, `v = "vv"`, `y = "yyyy"` | 0 | 2 | clean | the false arm drops the save and stores `y`; the run reads `y`'s length (4 - 2), so the store is visible |
| `q435runtime_direct` | `g[k] = [; a.v]; fill(g[1][0])` | 0 | 24 (varies) | heap-use-after-free | red direct runtime read; AR-35(e) blast radius |
| `q410dup` | generic whole move `let r: T = h;` at `T = P` (duplicable struct) | 0 | 0 | clean | duplicable control; `T = P`, not `i32` |

## Probe Matrix

| Row | Probe | Emit | Plain run | ASan/LSan | Verdict |
|-----|-------|------|-----------|-----------|---------|
| `DEF-410` | `d410` (filed `condnullhandle1`) | 0 | 134 (double free) | `attempting double-free` | 🔴 red |
| `DEF-410` | `d410b` (concrete twin) | 1 (`E10034` at 6:5) | — | — | control |
| `DEF-410` | `q410dup` (generic whole move at a duplicable `T`) | 0 | 0 | clean | control |
| `DEF-421` | `d421a` (`(*p).f`, `(*p).f.x`), `d421b` (`unsafe (*p).f`), `d421c` (`(*p).arr[i].x`) | 0 | 249 / 249 / 251 | clean | 🔴 red |
| `DEF-421` | `q423bump` (`(*p).arr[idx].y = bump()`) | 0 | 249 | clean | 🔴 red; the store is dropped and the index is not pinned |
| `DEF-421` | `d421d` (conditional-value base), the structural `.len` target | 1 (`E10221` at 10:5) | — | — | controls |
| `DEF-423` | `d423a` (fixed-array call RHS), `d423b` (`at1`), `d423c` (`at2`) | 0 | 139 SEGV / 134 / 134 | stack-buffer-overflow / double-free / double-free | 🔴 red; `at1`'s source read is a residual use-after-move |
| `DEF-423` | `d423d` (member route with an updating RHS) | 0 | 0 | clean | control |
| `DEF-424` | `d424a` (`au1`), `d424b` (`ak1`), `d424f` (generic constant self) | 0 | 139 / 139 / 139 | SEGV | 🔴 red |
| `DEF-424` | `q424self` (`x = *(&x)`), `q424two` (two-element self-read) | 0 | 139 / 139 | SEGV | 🔴 red |
| `DEF-425` | `d425a` (generic) | 0 | 0 | clean | ⏸ **PARKED** (accepted; contract conflict) |
| `DEF-425` | `d425b` (concrete borrow), `d425d` (take twin) | 1 (`E10034` 9:23) / 1 (`E10026` 9:23) | — | — | park evidence |
| `DEF-426` | `d426d` (concrete `/=`/`%=`), `d426e` (generic) | 0 | 0 / 0 | — | guards 3 / **0** |
| `DEF-426` | `d426c` (generic `/0`) | 0 | 136 SIGFPE | FPE | 🔴 red |
| `DEF-427` | `d427p` (filed double index) | 0 | 254 | 254 | 🟡 false positive; the scalar `len` fallback is `0` |
| `DEF-427` | `d427y` (loop alias `len(w.items[i])`), `d427l` (String element) | 0 | 0 / 0 | clean | controls |
| `DEF-428` | `d428a` (`s = s`), `d428b` (`ys = ys`) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-428` | `d428c` (direct fill) | 1 (`E10030` 13:5) | — | — | control |
| `DEF-429` | `d429a` (conditional row), `d429b` (conditional element), `d429c` (array literal) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-429` | `p429_runtime_dest` / `q429runtime_copy` (runtime destination, direct fill / copy) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-430` | `d430b` (runtime store k=1, constant read) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-430` | `p430_sibling` / `q430sibling` (constant store, sibling read) | 0 | 0 | clean | acceptance control |
| `DEF-430` | `d430c`/`d430e` (direct runtime fill), `d430d` (constant store, direct fill) | 0 / 0 / 1 | 0 / 0 / — | clean | control/residual note |
| `DEF-431` | `d431a` (`rows[0] = xs; let s = rows[0][0]`) | 0 | varies | heap-use-after-free | 🔴 red; the `429` gate is the primary candidate |
| `DEF-431` | `d431b` (direct fill) | 1 (`E10030` 15:5) | — | — | control |
| `DEF-432` | `d432a` (`[mk().v; "EE"]`) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-432` | `d432b` (direct `fill(mk().v)`) | 1 (`E10030` 12:5) | — | — | control |
| `DEF-433` | `d433a` (`let s = pick(xs)`), `d433b` (`fill(pick(xs))`) | 0 | varies / varies | heap-use-after-free | 🔴 red |
| `DEF-435` | `q435constant_copy` (constant `g[1] = [; a.v]`), `q435runtime_copy` (runtime `g[k]`) | 0 | varies | heap-use-after-free | 🔴 red |
| `DEF-435` | `q435runtime_direct` (direct read after the runtime store) | 0 | varies | heap-use-after-free | 🔴 red direct read |
| `DEF-435` | `d435b` (direct read after the constant store) | 1 (`E10030` 13:5) | — | — | control |
| `DEF-436` | `d436a` (`xs[i] = *alt(p,q)`) | 0 | 134 | double-free | 🔴 red |
| `DEF-436` | `d436b` (raw-target form) | 0 | 100 (wrong value) | clean | 🔴 red |
| `DEF-437` | `d437a` (store/field/take) | 0 | 134 | heap-use-after-free | 🔴 red |
| `DEF-437` | `p437_store_only` / `p437_return` / `p437_field_only` | 0 | 134 | attempting double-free | 🔴 red; all three emission gates miss the chain |
| `DEF-437` | `d437b` (conditional arm) | 0 | 0 | clean | control |
| `DEF-439` | `d439a` (`g = true`) | 0 | 134 | double-free | 🔴 red |
| `DEF-439` | `d439b` (`g = false`, aliasing), `q439nonalias` (`g = true`, non-aliasing) | 0 / 0 | 0 / 0 | clean | controls |
| `DEF-440` | `d440a` (`while (i < 2 && len(*alt(p,q)) == 2)`) | 0 | 134 | double-free | 🔴 red |
| `DEF-440` | `q440borrow` (`read((*alt(p,q)).s)`) | 0 | 134 | attempting double-free | 🔴 red |
| `DEF-441` | `d441a` (`g && eat(take (*alt(p,q)).name)`) | 0 | 134 | heap-use-after-free | 🔴 red |
| `DEF-441` | `q441sibling` (the same take, sibling read) | 0 | 134 | attempting double-free | 🔴 red; the root-wide empty loses the sibling |
| `DEF-445` | `d445a` (read base), `d445b` (nested field) | 0 | `cc=1` | — | 🔴 red |
| `DEF-445` | `q445mixed` (mixed place/literal arm) | 0 | `cc=1` | — | 🔴 red; the OR rule is required |
| `DEF-446` | `d446c` (ternary arm), `d446d` (if-condition control) | 0 / 0 | `cc=1` / 0 | — | 🔴 red / control |
| `DEF-447` | `d447a` (filed), `d447c`, `d447d`, `d447f`, `d447g` | 0 | 0 | **clean** | 🟡 false positive |
| `DEF-448` | `d448c` (`str[2]` places) | 0 | `cc=1` | — | 🔴 red |
| `DEF-448` | `d448a` (`String[2]`), `d448b` (`Box[2]`) | 0 | 0 / 0 | clean | controls |

**Expected post-fix behavior:**

| Row | Verdict | Expected post-fix behavior |
|-----|---------|----------------------------|
| `DEF-428` | 🔴 red | One `E10030` at the fill after the self-assignment; the in-scope read-back runs |
| `DEF-429` | 🔴 red | One `E10030` per affected fill, including the runtime-destination forms; the copies stay accepted; the constant sibling stays clean |
| `DEF-430` | 🔴 red | One `E10030` at the constant-read copy's fill; the wildcard matches only the runtime-written path; the sibling read stays accepted |
| `DEF-431` | 🔴 red | One `E10030` at the deeper-projection copy's fill; incidental if the `429` gate greens it |
| `DEF-432` | 🔴 red | One `E10030` at the temporary-derived element's fill; the lowered owner is recorded |
| `DEF-433` | 🔴 red | One `E10030` for both the copy and the call-argument forms; the caller-parameter control stays accepted |
| `DEF-435` | 🔴 red | One `E10030` at the constant and runtime element-store copies' fills; incidental (constant) if the `429` gate greens it; the constant direct read keeps its report and the runtime direct read is the AR-35(e) delta/residual |
| `DEF-437` | 🔴 red | Each projection moves once through all three emission gates; read-backs correct; ASan/LSan clean |
| `DEF-439` | 🔴 red | Both aliasing selections run 0; the false arm stores `y` (distinct lengths); the non-aliasing control drops its save; no double-free |
| `DEF-440` | 🔴 red | No bind/release for the intrinsic or borrow argument; the loop and call paths run; ASan/LSan clean |
| `DEF-441` | 🔴 red | The guarded take empties the selected member only; the sibling stays owned; both selections clean |
| `DEF-421` | 🔴 red | The four `Deref`-rooted store forms emit and read back; the base/index is pinned before the RHS; the structural `.len` keeps `E10221` |
| `DEF-423` | 🔴 red | The pinned target stores in bounds; the conditional self-store is a legal move with a correct destination read; `E11211` stays; `at1` is a filed residual |
| `DEF-424` | 🔴 red | The literal self-read runs before the clear; the constant-index self-copy and the raw self-alias (`is_raw_self_alias`) are no-ops; the distinct-binding/place controls and following store make a lost store visible |
| `DEF-436` | 🔴 red | One call per pointer expression (two total for the raw-target form); the correct value is stored; ASan/LSan clean |
| `DEF-410` | 🔴 red | One `E10034` per owning instance; the duplicable generic whole move runs; no report at the definition |
| `DEF-425` | ⏸ parked | Recorded as parked with the conflict evidence and the ruling request; no code |
| `DEF-426` | 🔴 red | The generic and concrete guards agree through the resolved projection type; the faults trap `E11210` |
| `DEF-445` | 🔴 red | The conditional-of-literals and mixed read bases build strictly and run 0 (OR rule) |
| `DEF-446` | 🔴 red | The ternary-arm take literal builds strictly and runs 0 |
| `DEF-448` | 🔴 red | The conditional array literal of places builds strictly and runs 0 |
| `DEF-427` | 🟡 false positive | The view-length matrix reads 2; the byte double index is accepted and ASan/LSan-clean, and its exact `254`/`0` output is an implementation pin |
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

**Verdict:** 🔴 red. **Fix direction:** skip the destination replacement on an exact self-assignment
by reusing `is_exact_self_assign` (`own/expr.rs:1481-1493`); the store is a no-op and the
destination's element-loan record stays. The snapshot-before-effect alternative is **not** taken
for this batch (the single-choice resolution keeps the change local). The self-assignment stays
accepted.

**Sites:** `own/statements.rs:793` (`bind_value`), `:951` (`set_binding_loans` before the copy
transfer), `own/place.rs:1096` (`set_binding_loans` drops `element_loans`), `own/expr.rs:1296`
(the snapshot after the effect).

### `DEF-429` — A composite copy source stored into an element, projected, or runtime destination loses the loans

**Repro (`d429a`/`d429b`/`d429c`/`p429_runtime_dest`):**

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
// p429_runtime_dest: a runtime destination, then a direct fill or a copy
grid[k] = c ? xs : zs;
fill(grid[1][0], borrow slot);  // or: let s = grid[1][0]; fill(s, borrow slot);
```

**Observed:** all accepted; the plain exit varies; ASan/LSan `heap-use-after-free` at each fill.
The runtime branch records only the binding-union loans (`replace_element_loans`'s union branch,
`own/place.rs:1119-1127`); it never records an element position, so `fill(grid[1][0])` and the
copy both escape. The direct conditional fill (`fill(grid[0][0])` after the conditional store) is
also accepted and ASan-red at the head; after the fix it becomes the control that keeps its
`E10030`. The constant sibling `ys[0] = a.v; let s = ys[1]; fill(s)` is accepted and
clean, which is the control a union merge must not over-report.

**Verdict:** 🔴 red. **Fix direction:** the **primary** P1 fix is to remove the
`projections.is_empty()` gate at `own/expr.rs:1299-1306` and pass the destination's projections as
`base` to `record_copy_element_loans` (and to `move_element_loans` for a move transfer). For a
runtime destination, record a wildcard `Element(None)` position through the same collector so a
constant read under the runtime-written path matches; do **not** union every loan onto every read.
The preflight's local experiment (removing the gate alone) greened the constant ST-2/ST-3/ST-4
probes, so the gate is the primary root; the runtime-destination cases remain red and need the
wildcard/collector path.

**Sites:** `own/expr.rs:1291-1318` (the `projections.is_empty()` gate), `own/place.rs:1236`
(`record_copy_element_loans`), `:1274` (`collect_copy_element_loans`), `:1111`
(`replace_element_loans`, the runtime union branch `:1119-1127`), `:1512` (`view_loans`).

### `DEF-431` — A single-view copy of a deeper projection loses the loan

**Repro (`d431a`):** `rows[0] = xs; let s: str = rows[0][0]; fill(s, borrow slot);` with
`xs[0] = a.v;`. The direct control (`d431b`) reports one `E10030` at 15:5.

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free`.

**Verdict:** 🔴 red. **Fix direction:** **verify after `DEF-429`**. The destination `rows[0]` is an
element projection, so the removed gate alone should record the source positions under `[0]` and
green this case. If it does, close `DEF-431` as incidental with the emitted/red-green evidence; if
it stays red, add the collector's prefix fallback to `move_element_loans` (`path.strip_prefix`
only at `own/place.rs:1212`, mirroring `read.starts_with(path)` in
`collect_place_copy_loans`). The direct control keeps its report.

**Sites:** `own/expr.rs:1291-1318` (the gate), `own/place.rs:1192` (`move_element_loans`),
`:1212` (the fallback point), `:1369-1398` (the collector's `read.starts_with(path)` rule).

### `DEF-435` — An element-store nested fill with a fresh member value

**Repro (`q435constant_copy`/`q435runtime_copy`):**

```clorraine
let a: Box = Box { v: stringOf("aa") };
let g: str[2][2] = [["AA", "BB"], ["CC", "DD"]];
g[1] = [; a.v];            // constant destination (q435constant_copy)
g[k] = [; a.v];            // runtime destination (q435runtime_copy)
let s: str = g[1][0];
fill(s, borrow slot);
```

**Observed:** both accepted; plain exit varies; ASan/LSan `heap-use-after-free`. The constant
destination `g[1]` goes through the ordinary store path and is blocked by the `DEF-429` gate; it
never reaches `emit_indexed_store`. The runtime destination `g[k]` reaches `emit_indexed_store`
(`own/expr.rs:2557-2629`), which lowers the view value and emits `StorePointer` without calling the
collector. The **constant direct read** (`d435b`) reports one `E10030` at 13:5; the **runtime
direct read** (`q435runtime_direct`) is accepted and ASan-red at the head.

**Verdict:** 🔴 red. **Fix direction:** **verify the constant half after `DEF-429`**. If the gate
alone greens `q435constant_copy`, close that half as incidental (it is the `DEF-431` prefix-fallback
path); otherwise apply the `DEF-431` fallback. The runtime half needs the composite-aware collector
in `emit_indexed_store` with the element's destination path (the `base` parameter is ready). The
runtime direct read is the same collector's blast radius (AR-12) and is recorded per AR-35(e):
the intended delta is one `E10030`; if it stays accepted, the phase files a residual row with
`q435runtime_direct`. The constant direct read is a separate control and keeps its report.

**Sites:** `own/expr.rs:2557-2629` (`emit_indexed_store`, the `StorePointer` path never calls the
collector), `:1291-1318` (the constant store gate), `own/place.rs:1236`.

## Phase 2 — Loan sources through runtime, temporary, and call reads

### `DEF-430` — A runtime-index store followed by a constant-read copy loses the loan

**Repro (`d430b`):** `xs[k] = a.v;` with `k = 1`, then `let s: str = xs[1]; fill(s, ...)`.

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free`. With `k = 0`
(`d430a`) the run is clean because the selected storage is not the loaned one, but the compiler
cannot know that. The direct runtime fill (`d430c`, `k = 1`) is accepted and clean in this probe;
the constant-index control (`d430d`) reports one `E10030` at 13:5. The constant-store sibling
control `ys[0] = a.v; let s = ys[1]; fill(s)` (`p430_sibling`/`q430sibling`) is accepted and
clean, so a naive union merge of every binding loan onto every constant read false-rejects.

**Verdict:** 🔴 red (the copy). **Fix direction:** record a **wildcard runtime position**
(`Element(None)`) in `element_loans` when the runtime store cannot name its element, and match a
constant read only when it falls under the runtime-written path; not a whole-binding union. The
`k = 0` variant keeps the conservative report (the compiler cannot know the runtime index); the
sibling control stays accepted. The direct runtime fill is the blast-radius case (AR-8).

**Sites:** `own/place.rs:1119-1127` (the runtime union branch), `:1192` (`move_element_loans`),
`:1369` (`collect_place_copy_loans`), `:1512` (`view_loans`).

### `DEF-432` — A temporary-derived element in a view-array literal escapes

**Repro (`d432a`):** `let ys: str[2] = [mk().v; "EE"]; fill(ys[0], borrow slot);` with
`mk(): Box { v: String }`.

**Observed:** accepted; plain exit varies; ASan/LSan `heap-use-after-free`. The direct
`fill(mk().v, ...)` (`d432b`) reports one `E10030` at 12:5.

**Verdict:** 🔴 red. **Fix direction:** record the provenance owners (including a lowered temp) of
a non-place element/fill value under the destination path. The collector currently receives only
`&Expr`; thread the lowered value (or its `view_owners`/`lowered_root_bindings` owner set) from
`check_let`/`check_assign` through `record_copy_element_loans` so the statement temp
(`__cl_owned0` and friends) is nameable.

**Sites:** `own/place.rs:1421-1428` (the non-place branch: only `view_loans`), `:1569`
(`view_owners`), `:1603` (`lowered_root_bindings`), `own/lower.rs:3101-3184`
(`lowered_temp_owners`), the `check_let`/`check_assign` call sites.

### `DEF-433` — A call-mediated element-view result escapes

**Repro (`d433a`/`d433b`):**

```clorraine
func pick(xs: str[2], i: i32): str { return xs[i]; }
...
xs[0] = a.v;
let s: str = pick(xs, 0);       // d433a
fill(pick(xs, 0), borrow slot); // d433b
```

**Observed:** both accepted; plain exit varies; ASan/LSan `heap-use-after-free`. The direct call
form has no binding root, so `recorded_position_frame_owners` returns `None` before the binding
loans are ever consulted.

**Verdict:** 🔴 red. **Fix direction:** in `frame_owned_view_owner`, for a non-place value consult
`view_loans(expr)` and expand them with `loan_owners` under the frame filter; consult the
binding-loan owners before the provenance fallback. A caller-parameter view stays accepted.

**Sites:** `own/call_check.rs:585-616` (`frame_owned_view_owner`), `own/place.rs:975-1012`
(`recorded_position_frame_owners` reads only `element_loans`), `:1512` (`view_loans`), `:815`
(`loan_owners`), `:808` (`binding_loan_owners`).

## Phase 3 — Raw-deref move-outs

### `DEF-437` — A member projection of a raw pointee is not materialized

**Repro (`d437a`/`p437_store_only`/`p437_return`/`p437_field_only`):** `let v: String = (*p).s;`,
`return (*p).s`, `Holder { t: (*p).s }`, and `eat(take (*p).s)`; `p: raw S`, `S { s: String }`.

**Observed:** accepted; each isolated form plain exits 134 (double free); ASan/LSan `attempting
double-free`. The combined `d437a` reports a heap-use-after-free.

**Verdict:** 🔴 red. **Fix direction:** pin a `Deref` anywhere in the projection chain before the
move. The materializer alone is not enough: the three emission gates must also cover the chain —
`moves_pointee` (`codegen/function.rs:1382-1384`), `return_source_needs_emptying`
(`codegen/cleanup.rs:545-567`), and `struct_literal_needs_stores`/its field loop
(`codegen/function.rs:2071-2094`, `:1421-1432`). Add the store, return, and field cases to ST-8.

**Sites:** `own/lower.rs:3699-3721` (`materialize_owning_deref_at` top-level only),
`own/statements.rs:502` (the named-store call), the three codegen gates above. The conditional-arm
control (`d437b`, `g ? (*p).s : stringOf("zz")`) runs 0, matching the batch-M note.

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
false path (`d439b`/`q439alias_false`, `g = false`, still aliasing) runs 0, clean in the earlier
equal-length probe; with distinct lengths (`v = "vv"`, `y = "yyyy"`) it runs 2 and reads the
stored `y`, which shows the false arm stores `y` rather than reusing the old `v`. The non-aliasing
control (`q439nonalias`, `g = true`, `p = &w`) runs 0 and is clean.

**Verdict:** 🔴 red. **Fix direction:** the bool `arm_consumes_target` cannot carry the runtime
alias check. Specify the per-arm save handling directly: in the then-branch zero the save only
when the taken value's rendered address equals the target; in the else-branch drop the save
normally and store `y`. The equivalent form is the emitted condition
`take_ran && &pointee == &target`. Pin the emitted condition in ST-9's impl test, use distinct
lengths for the false-path selection, and keep the non-aliasing control.

**Sites:** `codegen/function.rs:2400-2444` (`arm_consumes_target` and the arm store),
`compiler/src/ir.rs` (the `Materialize.value_move_outs` field).

### `DEF-440` — A guarded intrinsic or borrow argument binds and releases a raw-deref pointee

**Repro (`d440a`/`q440borrow`):** `while (i < 2 && (i32(len(*alt(p, q))) == 2)) { ... }` and
`read((*alt(p, q)).s)` with a borrowing `read`.

**Observed:** accepted; plain exit 134; ASan/LSan `attempting double-free` (the borrowing-argument
form and the intrinsic loop share the cause).

**Verdict:** 🔴 red. **Fix direction:** return a `Deref`-rooted projection chain unchanged for a
borrowing intrinsic **and** a non-take borrow argument (extend the non-take rule at
`own/lower.rs:5393-5401` to walk `Member`/`Index` bases to the `Deref` root).

**Sites:** `own/lower.rs:4171-4234` (`bind_intrinsic_argument`), `:5393-5401` (the non-take rule
to extend).

### `DEF-441` — A guarded take of a member over a raw pointer leaves the pointee owned

**Repro (`d441a`/`q441sibling`):** `g && (eat(take (*alt(p, q)).name) == 1)` with `p: raw S`,
`q: raw S`, `S { name: String; other: String }`; the sibling variant reads `other` after the take.

**Observed:** accepted; plain exit 134; ASan/LSan `attempting double-free` (the sibling variant)
and `heap-use-after-free` (`d441a`). The root-wide empty loses the sibling member.

**Verdict:** 🔴 red. **Fix direction:** record the **projected member** value (the `Member`/`Index`
rooted at the `Deref`), not the root, so the emitter empties only the selected member; extend the
take-path pointer pin (`own/lower.rs:5239-5262`) to the chain. Add the two-owned-member sibling
control and an impure-pointer single-call pin.

**Sites:** `own/call.rs:804-821` (only a top-level `Deref` is matched), `own/lower.rs:5239-5262`
(the take-path pin), `own/expr.rs` (the projection chain), `compiler/src/ir.rs` (`MoveOutValue`).

## Phase 4 — Store lowering and evaluation order

### `DEF-421` — A deref-rooted member store is accepted and silently dropped

**Repro (`d421a`/`d421b`/`d421c`/`q423bump`):** `(*p).f = Inner { x: 7 };`,
`unsafe (*p).f = ...;`, `(*p).arr[i].x = 7;`, and `(*p).arr[idx].y = bump();` with `p: raw S`.

**Observed:** accepted; strict build succeeds; the stores are absent; the read-backs run 249 /
249 / 251. The `q423bump` store is also dropped (exit 249; `bump` does not run). The plain forms
are base-identical; the `unsafe` spelling is newly accepted by the batch-M `Unsafe` place arm and
still dropped.

**Verdict:** 🔴 red. **Fix direction:** route the `Deref`-rooted `Member`/`Index` target through the
raw-deref store path in `check_assign`; do **not** add a `Deref` arm to `place_target`. Pin the
target's base/index before lowering the RHS (`pin_store_target`, `own/expr.rs:2393-2421`), so
`(*p).arr[idx] = bump()` stores at the pre-call index and calls `bump` once. The `None` fallback in
`emit_indexed_store` becomes a debug assertion. Keep the structural `.len` `E10221` branch
(`own/expr.rs:1448-1461`); the conditional-value base control (`d421d`) reports one `E10221` at
10:5, unchanged (batch-M AR-44).

**Sites:** `own/place.rs:1931-1990` (no `Deref` arm), `own/expr.rs:1003-1073` (the raw-deref
target matches only a bare `*p`), `:2393-2421` (`pin_store_target`), `:2280` (`emit_assigned`),
`:2557` (`emit_indexed_store`).

### `DEF-423` — The whole-element store evaluates the RHS before the target index is pinned

**Repro (`d423a`):** `a[idx] = mkrow();` where `mkrow` sets `idx = 5`; **(`d423b`/`d423c`):**
`a[idx] = set_idx() ? a[idx] : Elem { ... };` where `set_idx` sets `idx = 1`.

**Observed:** `d423a` plain exit 139 (SEGV); ASan `stack-buffer-overflow` at index 5. `d423b`/
`d423c` plain exit 134; ASan `attempting double-free` (the store lands in `a[1]` while `a[0]` is
saved and both own the saved handle). The member-route equivalent (`d423e`) pins the target but
reads the emptied source `a[1]` and SEGVs; its move itself is legal (a[0] receives a[1], a[1] is
emptied), so the spec case reads the destination.

**Verdict:** 🔴 red. **Fix direction:** always pin the whole-element base/index before the RHS (the
member route already pins the target). The member-route control (`d423d`, `a[idx].y = set_idx()`
with an updating RHS) runs 0. The conditional self-store is a legal move after the fix; the spec
case reads `a[0]` (the destination) rather than the moved-out source (AR-18). The accepted
post-store source read (`at1`, `a[1]` after the store) is a separate use-after-move: file it as a
residual row at the phase-4 record; do not fold it into this fix.

**Sites:** `own/expr.rs:2490-2549` (`indexed_target`), `:2566` (`pin = contains_update(value)`),
`:2579-2629` (the whole-element path).

### `DEF-424` — The fixed-array literal save and clear run before the RHS

**Repro (`d424a`):** `a[i].names = [stringOf(a[i].names[0]), stringOf("xx")];` with
`names: String[2]`. **(`d424b`/`q424inner`):** `a[i].inner = *(&a[i].inner);`. **(`d424f`):** a
generic `same<T>(borrow w: T) { w.items[1] = w.items[1]; }` instantiated owning. **(`q424self`):**
`x = *(&x)`; **(`q424two`):** a two-element self-read.

**Observed:** `d424a`/`d424b`/`d424f`/`q424self`/`q424two` plain exit 139 (SEGV); ASan SEGV on a
null read. The concrete constant-index `w.items[1] = w.items[1]` (`d424c`) reports `E10026` (the
partial-move limit); the red constant-index shape is the generic owned-move acceptance. The
`d424b` shape is worse than a clear-order bug: the raw-deref RHS move-out zeroes the just-stored
target.

**Verdict:** 🔴 red. **Fix direction:** evaluate the literal's self-reading element expressions
into statement temporaries before the save/clear; the temporaries have one statement lifetime,
start zeroed, are stored once, and each is moved out once (one zeroed temp array, store,
save/clear, store, then `emit_move_zero`). Treat the raw self-alias `*(&x)` / `*(&a[i].inner)` as
a no-op with a dedicated recognizer: `is_raw_self_alias` (new beside `is_exact_self_assign` in
`own/expr.rs`) unwraps `Deref(AddressOf(place))` (through `Unsafe`) and compares the inner place's
`place_target` to the target binding/projections, or compares the rendered addresses.
`is_exact_self_assign` alone is insufficient: it matches only `PlaceTarget::Local`
(`own/expr.rs:1481-1493`), while the raw forms have no place target, and AR-17 forbids a `Deref`
arm in `place_target`. Do not let the recognizer match a distinct place: ST-14 pins `x = *(&x)` and
`a[i].inner = *(&a[i].inner)` with distinct values so a lost store is visible. Skip the
identical constant-index `Move` (extend batch-M F3's self-copy rule). Add `x = *(&x)`, the nested
literal, and the two-element self-read to ST-14.

**Sites:** `codegen/function.rs:1557-1592` (the literal `reads_target` save/clear),
`:659-690` (the `Statement::Move` emission, the runtime-only self-copy check),
`own/statements.rs:445-479` (`prepare_store` returns the `Move`), `own/expr.rs:1481-1493`
(`is_exact_self_assign`; `is_raw_self_alias` is new beside it).

### `DEF-436` — The whole-element and raw-deref-target stores bypass the owning-deref materializer

**Repro (`d436a`):** `xs[i] = *alt(p, q);` with an impure `alt`; **(`d436b`):**
`unsafe *alt(rp, rq) = *alt(p, q);`.

**Observed:** `d436a` accepted, plain 134, ASan `attempting double-free`; the emitted C calls
`cl_alt` twice. `d436b` accepted, plain 100 (the target is emptied to null instead of receiving
`bb`; `calls = 3`), ASan clean. The filed raw-target double-free is the same re-evaluation
defect; the head's pinned LHS leaves the RHS re-evaluation. The raw-target form has **two** `alt`
expressions (LHS and RHS), so the single-call pin means each pointer expression runs once, two
total for that form; the element-store case stays one.

**Verdict:** 🔴 red. **Fix direction:** run the shared `materialize_owning_deref` on the RHS in
both lowerings.

**Sites:** `own/expr.rs:2603-2622` (`emit_indexed_store` → `consume_place`), `:1055-1076`
(the raw-deref target → `consume_place`), `own/statements.rs:502`
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

**Verdict:** 🔴 red. **Fix direction:** record the conditional arm as a maybe-owning source instead
of skipping a type-parameter arm; the effect is selected in `prepare_binding_effect`
(`own/statements.rs:816-887`), so an owning instance reports one `E10034` (BR-5) matching the
concrete twin, while a duplicable instance stays accepted. Do not report at the definition. The
duplicable control must be a separate generic whole move (`let r: T = h;` at `T = P`, the
duplicable struct used by `q410dup`), because the `String?` body cannot take a duplicable `T`.

**Sites:** `own/expr.rs:175-205` (`conditional_move_sources`, the `mentions_type_param` skip at
:191), `own/statements.rs:816-887` (`prepare_binding_effect`), `own/place.rs:541-590`
(`mark_moved_if_owning_at`), `own/instance_check.rs:210-260` (`check_not_lend_requirements`).

### `DEF-425` — A generic body accepts an owned element move through a borrow parameter (**PARKED**)

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

**Verdict:** ⏸ **PARKED** (AR-22), not fixed and not closed. Frozen BR-5
(`spec/16-borrow-rules.md:191`) and the concrete/take twins say the owning body must be rejected,
but the batch-M origin cases pin the same body accepted at exit 0:
`compiler/tests/conditional_projection_residue_spec_test.rs:14116-14198`
(`rhsindexselfmember`/`rhsindexselfsamewhole`) and
`..._impl_test.rs:7365-7460` (`implselfslot`) pin the generic
`copy<T>(borrow w: T) { w.items[i].name = w.items[j].name; }` accepted. Changing an immutable
spec-oracle case needs a user ruling (AR-22 + AR-30 + AR-33). The plan records the conflict and the ruling
request; no compiler, test, spec, or registry edit is made here.

**Sites (for the ruling, not for a fix):** `own/place.rs:629-666`
(`note_borrowed_element_move`, `BindingKind::Local` only), `:541-590`
(`mark_moved_if_owning_at`), `own/instance_check.rs:210-260`.

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
`INT_MIN % -1` are the same class. The guard already substitutes `left.ty`; the recorded type is
the unresolved projection type.

**Verdict:** 🔴 red. **Fix direction:** resolve the unknown projection type through
`resolved_place_ty`/`resolved_value_ty` (`codegen/expr.rs:321`, `:397`) before the guard's
`is_integer`/scalar decision; the guard already substitutes `left.ty`.

**Sites:** `codegen/guards.rs:1444-1460` (`division_guard_condition`), `:1435`
(`emit_division_guard`), `codegen/expr.rs:321` (`resolved_place_ty`), `:397`
(`resolved_value_ty`), `own/expr.rs:2325-2341` (`combine`), `:2557-2629` (the compound element
path).

## Phase 6 — Literal destinations and conditional arrays

### `DEF-445` — The conditional-of-literals read base is accepted and emits invalid C

**Repro (`d445a`):** `i32((ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0])` with
`n1/n2: str[2]`. **(`d445b`):** the same conditional nested in an `Outer { inner: ... }` field.
**(`q445mixed`):** `(ok ? inner : Inner { row: n2() }).row[0][0]`, one place arm and one literal
arm.

**Observed:** accepted; the strict C build fails: `too few arguments to function
'cl_probe_d445a_n1'; expected 1, have 0` (the hidden `str[2]` destination is not bound). The mixed
place/literal form emits `(ok ? inner : (ClInner){ cl_probe_p445_mixed_n2() }).row[0]` and fails
the same way; the literal arm's hidden destination is unbound while the place arm is fine.

**Verdict:** 🔴 red. **Fix direction:** apply the **OR** rule in the literal read/store predicates:
if any conditional arm needs the store form, use the store form for the whole conditional. Only
`literal_read_needs_stores` returns false for a `Conditional` at its own level; the field/element
predicates already recurse or return true. The mixed place/literal read base is red and must be
included, not just the both-literal form. Name `struct_literal_needs_stores` in the codegen fix
step.

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
The `0` is the compiler's scalar-`len` fallback (`codegen/calls.rs`, the `len` `_ => "0"` arm),
not a frozen rule.

**Controls:** `d427y` (loop alias, `len(w.items[i])`) runs 0; `d427l` (`String` element) runs 0;
`d427a`/`d427e`/`d427i`/`d427j`/`d427m`/`d427q`/`d427r`/`d427t`/`d427u`/`d427v`/`d427w` (the
concrete/generic/borrow/`str`/`String` view-length spellings) all run 0; the single-index member
form `len(w.items[i].name)` runs 0.

**Verdict:** 🟡 **false positive** (AR-27). The double index selects a `u8`; the expected 2
belongs to `len(w.items[i])`, which is green. The spec case asserts only that the frozen
view-length matrix and the filed shape are accepted and ASan/LSan-clean; it does not assert the
filed run's exit value. The exact `254`/`0` observations move to an implementation pin that
documents the current output. A residual row is filed for the scalar-`len` fold (no code in this
batch), and a future scalar-`len` fix needs a user ruling because the residual row records the
current fallback. No code change here.

**Sites:** `codegen/calls.rs:339-370` (`intrinsic`, the `len` `_ => "0"` fallback),
`types/mod.rs` `index_element` (`Str`/`StaticStr` → `u8`), `own/lower.rs:550` (the fixed-array
fold). The probe ASan leg needs the array-value suite's `run_memory` helper, which that suite does
not yet have.

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
