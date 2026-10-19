# Findings

## Phase 1 — View escapes through carriers and calls

**Reviewed implementation**: `07036c9` (carrier loans, aggregate out-slot, call/global view roots), `80f8ae0` (red cases and pins), `5741d5c` (literal-arm union, narrowed call roots, take/view split, owned-storage gate), `90ae07c`/`b996610` (known-empty positions, field gate, plain literal gate), `4e0dad6` (static-str, shadow, alias, run-time parameter, call-returned carriers), `dd0eab5` (view-like destination route, generated-only leaf loans). **Lenses**: all four.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS — 0 in-diff blockers after correction | The four filed shapes reject once with no TU; SR-001..SR-007 closed; the stdlib view unit restored; `make check` exit 0. |
| semantics | PASS — 0 in-diff blockers | Both-literal, unrelated-argument, static-bound, take aggregate/view, static/shadow/alias/call-returned carrier, mixed-field, and filed shapes all match the concrete twins. |
| memory-safety | CONFIRMED — no base-clean → head-red memory defect | The temporary/call-carrier and lost-loan shapes reject; accepted controls ASan/LSan clean; no new UAF acceptance. One safe false rejection (`DEF-484`) and pre-existing classes filed. |
| emission-integrity | CONFIRMED | 2,218 base-accepted programs byte-identical; no new strict-build failure; re-emission and both profiles clean; filed shapes emit no C. |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `8e15ed1`, `07036c9`, `80f8ae0` | Red cases, carrier loan transfer, aggregate/call escape judgment, pins |
| C1 | `5741d5c` | Struct-literal arm, narrowed call root walk, take/view split, owned-storage gate, alias/static controls |
| C2 | `90ae07c`, `b996610` | Known-empty positions, field gate, plain literal gate |
| C3 | `4e0dad6` | Static-str, shadow, alias, run-time parameter, call-returned carriers |
| C4 | `dd0eab5` | View-like destination stays on the binding-effect route; generated-only leaf loans |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (`80f8ae0`) | **NOT CONFIRMED**: both-literal arms lost loans (UAF), unrelated call arguments loaned, take double report, static aggregates rejected |
| B2 (`5741d5c`) | **NOT CONFIRMED**: view-only early return accepted temporary-backed carriers; literal fields loaned scalars |
| B3 (`90ae07c` committed) | **NOT CONFIRMED**: the plain literal gate was uncommitted; static/alias/call carriers regressed |
| B4 (`dd0eab5`) | **CONFIRMED**: all five SR classes and the four filed shapes pass; close recommended |

### Filed rows

`DEF-484` (static local alias false rejection), `DEF-485`/`DEF-486` (const/global conditional carrier copy), `DEF-487` (function-returned call-mediated view), `DEF-488` (inline call-result carrier, local backing), `DEF-489` (static/call carrier mix), `DEF-490` (inline static aggregate literal), `DEF-491` (frame-backed literal arm extra report), `DEF-492` (zero-argument static factory), `DEF-493` (plain carrier copy from a carrier call), `DEF-494` (taken-view double report), `DEF-495` (direct call view: closure/wrapper/generic), `DEF-496` (dot receiver), `DEF-497` (mixed aggregate carrier), `DEF-498` (conditional shadow copy), `DEF-499` (alias element return), `DEF-500` (strict-build residuals).

### Record notes

- The carrier copy now records known-empty view positions for static, module, and parameter-backed storage, so a view-only carrier is not mistaken for its own backing; a call-temporary or lost-loan position still falls through to the frame judgment.
- The call root walk follows only the single lend source; a named source container is not a loan owner.
- A take aggregate's caller-side report is suppressed (the callee owns it); a taken view keeps both sites.
- No `spec/` oracle or registry edit.

## Phase 2 — Take alias capture

**Reviewed implementation**: `f4de6ae` (indexed pointer capture), `d01484e` (conditional, member, computed-address, module-global, writable-parameter, and alias captures; stable bare local/by-value parameter). **Lenses**: correctness, semantics, memory-safety.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS — the filed and sibling pointer shapes are captured | The index, member, conditional, global, and computed-address cases are ASan-clean; the bare local and by-value parameter stay direct; the six weakened assertions are restored; the impl pin checks the captured temp, call, alias test, and move-out. |
| semantics | PASS | The callee, alias test, and move-out read one captured pointer; guarded operands defer the fill; a skipped path never evaluates the pointer. |
| memory-safety | CONFIRMED | The base double-free shapes are clean at head; no base-clean → head-red memory defect; accepted controls are LSan-clean. |

### Residuals

None for `DEF-459`; the broader pointer spellings are fixed. The filed row is Done.

## Phase 3 — Generic conditional duties

**Reviewed implementation**: `0b5acd5` (type-parameter conditional sources at a return and a binding), `d01484e` (selected member/element resolution in the duty, raw pointee included). **Lenses**: correctness, semantics, memory-safety.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS for the filed rows; base-identical sibling shapes filed | The generic return and field-let report one `E10034` at the instance; a duplicable member is accepted; a view/raw element reports. The take, struct-literal, projected-assignment, and projection-over-conditional forms remain base-identical and are filed (`DEF-501`..`DEF-505`). |
| semantics | PASS for the filed rows | The concrete twins and the generic instances agree on the selected member; the `T = raw u8`/view controls stay accepted. |
| memory-safety | CONFIRMED for the filed rows; the pre-existing take/projection classes are filed | The filed repros no longer double-free; no base-clean → head-red defect. |

### Incidental closure

`DEF-470` (a generic field-projected take later move) now reports the concrete twin's `E10020` at the later whole use; closed incidentally by the per-instance duty.

## Phase 4 — Ruling, controls, and close-out

**Ruling**: a generated owner (`__cl_owned<N>`) is a compiler binding that holds the value captured by a local or a literal, not a `__cl_tmp<N>` source temporary. Ch 03 D5 governs source temporaries; a generated owner lives in the binding's frame and is released at that scope's exit. The body-scope binding is conformant, and a same-scope read stays accepted; a later fill that stores a temporary-derived view still reports the escape.

**Control**: `should_accept_a_next_statement_read_of_a_generated_owner` (residue spec) is green plain and under `RDC_ASAN=1`. No code change. `DEF-452` is Done with the ruling.

## Phase 4 — Ruling, controls, and close-out

_(to be recorded)_
