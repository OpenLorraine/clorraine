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

_(to be recorded)_

## Phase 3 — Generic conditional duties

_(to be recorded)_

## Phase 4 — Ruling, controls, and close-out

_(to be recorded)_
