# Preflight Report — Memory and Value Integrity (Batch N)

> **Audit target**: `plans/memory-value-integrity/` at `100180c` (14 documents, 3,087 lines)
> **Revised**: `ab4f611` (iteration-1 fixes), `506a78e` (iteration-2 fixes)
> **Mode**: strict scope; auto-design under the standing defect mandate; independent five-cluster
> fan-out, one fix-scoped re-scan, one bounded confirmation
> **Date**: 2026-10-18
> **CodeOps Artifact Schema**: 1

## Verdict

**✅ PREFLIGHT PASSED — all findings resolved.**

| Round | Findings | Status |
|-------|----------|--------|
| Iteration 1 (five clusters) | 57 raw → 21 resolution groups (`A1`–`A3`, `B1`–`B14`, `C1`–`C4`) | All resolved |
| Iteration 2 (fix-scoped) | 9 (4 major, 5 minor) | All applied (`I2-1`–`I2-9`) |
| Confirmation (bounded) | 1 (the `DEF-439` run-value residual) | Fixed in place (the five stale lines corrected to the `g = true` 0 / distinct-length `g = false` 2 reading) |

No critical or major finding remains. `DEF-425` is a **scope decision** (parked pending the user's
ruling), not an open finding; the two false positives are evidenced controls.

## Scope

The audit target is the batch-N plan set: the 23 rows named by the dispatch — the nine remaining
criticals (`DEF-428`, `429`, `430`, `431`, `432`, `433`, `437`, `440`, `441`) and the fourteen
memory/store majors (`DEF-410`, `421`, `423`–`427`, `435`, `436`, `439`, `445`, `446`, `447`,
`448`). The two parked majors (`DEF-167`, `DEF-171`), the other majors, and the minors are out of
scope. Context: the batch-M ledgers (the origin evidence), `plans/00-roadmap.md`, `AGENTS.md`, the
frozen `spec/`, and the compiler source.

## Method

| Round | Dispatches | Raw findings | Result |
|-------|------------|-------------|--------|
| Iteration 1 | 5 clustered auditors (① soundness · ② grounding · ③ delivery · ④ risk · ⑤ fit) | 57 (7 + 17 + 10 + 18 + 5) | Merged to 21 resolution groups (`A1`–`A3`, `B1`–`B14`, `C1`–`C4`) |
| Iteration 2 | 1 fix-scoped verifier (the revised plan) | 9 (4 major, 5 minor) | All applied (`I2-1`–`I2-9`) |
| Confirmation | 1 bounded verifier | appended | — |

The auditors read the artifact directly, re-ran the filed probes plus independent edge probes, and
cited the frozen text; no auditor fixed anything. The lead merged and resolved.

## Iteration 1 — merged resolutions

| Group | Sources | Finding | Resolution (applied) |
|-------|---------|---------|----------------------|
| A1 | c1 PA-001 | `DEF-425`'s `E10034` fix rejects the same generic borrow-element move that the batch-M spec/impl cases (`rhsindexselfmember`, `implselfslot`) pin accepted; an immutable spec-oracle change needs a user ruling | **Park `DEF-425`** with the conflict evidence (frozen BR-5 + the concrete/take twins vs the pinned acceptance); P5 re-scoped; the ruling requested; counts reconciled |
| A2 | c3 PA-001/002, c4 PA-001/002 | the `DEF-429` gate alone greens `DEF-431`/`DEF-435`; `DEF-435`'s named route is dead for its case; the runtime-destination shape (`grid[k] = …; fill(grid[1][0])`) is an accepted UAF | `DEF-429` primary; the runtime-destination cases + the constant-sibling acceptance control; `DEF-431`/`DEF-435` as a phase-1 verification branch (incidental closure or the prefix fallback/collector); the root-cause rewrite |
| A3 | c5 PA-001/002, c1 PA-005, c4 PA-016 | the `DEF-427` control pins an unfrozen `len(u8) == 0`; the P7 ASan command is a no-op | `len(u8)==0` moved to an impl pin; the spec case keeps the frozen view matrix + acceptance; a scalar-`len` residual row; the array-value `run_memory` helper + the P7 ASan scope |
| B1 | c2 PA-01, c4 PA-003 | `DEF-437`'s materializer alone leaves the store/return/field double-free | the three emission gates extended to the Deref-rooted chain |
| B2 | c2 PA-02, c4 PA-004 | `DEF-441`'s "root Deref" zeroes the whole pointee | record the projected member; extend the take-path pin; the sibling/single-call pins |
| B3 | c2 PA-03, c4 PA-005 | `DEF-439`'s bool predicate cannot carry the runtime alias check | the per-arm save handling (`take_ran && alias` or the branch move); the aliasing-false/non-aliasing controls |
| B4 | c2 PA-16, c4 PA-006 | `DEF-432`'s collector never receives the lowered value | thread `view_owners`/`lowered_root_bindings`; the `__cl_owned0` pin |
| B5 | c2 PA-06, c4 PA-007 | `DEF-433`'s direct call form has no binding root | `view_loans` + `loan_owners` with the frame filter; the direct-call pin |
| B6 | c2 PA-04, c4 PA-008 | `DEF-430`'s runtime union has no distinguishable record and a naive merge false-rejects the sibling copy | the `Element(None)` wildcard marker; the sibling acceptance and `k=0` outcomes |
| B7 | c2 PA-05, c4 PA-009/010 | `DEF-424` includes the raw self-alias shape and the temp lifetime | the raw self-alias recognizer; the temp lifetime/zero; the controls |
| B8 | c4 PA-011 | `DEF-445`'s predicate rule contradicts itself; the mixed arm is red | the OR rule; the mixed cases; `struct_literal_needs_stores` named |
| B9 | c4 PA-012 | `DEF-440`'s borrow-argument half is not fixed | the non-take chain extension; the borrow-argument case |
| B10 | c4 PA-013, c2 PA-09 | `DEF-421`'s raw target is not pinned; the `.len` fallback shares the `None` arm | `pin_store_target` before the RHS; the structural `.len` branch kept |
| B11 | c4, c1 | the `DEF-423` `at1` source read is an accepted use-after-move | filed as a residual row at the phase-4 record; the destination read in ST-13 |
| B12 | c1 PA-002, c2 PA-07 | `DEF-410`'s duplicable control is not constructible; the effect site is `prepare_binding_effect` | the `T = P` control; the effect site named; the no-report-at-definition pin |
| B13 | c2 PA-08 | `DEF-426`'s hook is the resolved type, not `element_ty` | the root-cause rewrite (`resolved_place_ty`/`resolved_value_ty`) |
| B14 | c4 PA-018 | `DEF-436`'s raw-target form has two `alt` expressions | each expression once (two total); the element case one |
| C1 | c1/c2/c4 minors | the site-line drifts; the `is_exact_self_assign` reuse; the `.len` branch | corrected |
| C2 | c3 PA-003..005, c1 PA-006 | the task wording (the self-assignment alternative; the `Deref` arm; the `RequiresNotLend` duty) | single-choice wording; the raw-deref route; `RequiresNotLend` named; P5 scoped to 410/426 |
| C3 | c3 PA-006..010, c4 PA-017 | the dependency graph, the index suite summary, the oracle names, the freestanding legs, the probe preservation, the citations | corrected; the `E11200` record steps; `build/batchN-plan/probes/` |
| C4 | the specialist note | `analyze-agents` after the new plan | task 1.0.1 added; the state refreshed (outcome None) |

## Iteration 2 — the re-scan findings

| ID | Severity | Finding | Resolution (applied) |
|----|----------|---------|----------------------|
| I2-001 | 🟠 | `DEF-424`'s `is_exact_self_assign` reuse cannot see `*(&x)` (a `Local`-only predicate) | the `is_raw_self_alias` recognizer (unwrap `Deref(AddressOf(place))`); distinct-value pins |
| I2-002 | 🟠 | the `Element(None)` wildcard has no record/match hook | the wildcard match semantics for the three readers; the non-constant branch records it; the helpers in the blast radius |
| I2-003 | 🟠 | `DEF-427`'s spec case still pins the filed `254` | the spec asserts acceptance + sanitizer cleanliness; the exact values in the impl pin; the residual ruling note |
| I2-004 | 🟠 | `DEF-439`'s "false path leaves `v` intact" contradicts the store (the false arm stores `y`) | the corrected expectation; distinct lengths (`"vv"` vs `"yyyy"`); the non-aliasing control |
| I2-005 | 🟡 | the `DEF-435` constant/runtime direct reads conflated | separated; the intended delta/residual stated |
| I2-006 | 🟡 | the `DEF-410` control shape (`i32` vs the recorded `P`) | standardized on `T = P` |
| I2-007 | 🟡 | the AR-22 citation (`AR-33/AR-37`) | `AR-22 + AR-30 + AR-33` |
| I2-008 | 🟡 | the index line still lists `DEF-425` to close | `(DEF-410, DEF-426; DEF-425 parked)` |
| I2-009 | 🟡 | the `P4 → P5` dependency arrow is unsupported | dropped; the shared resolvers already exist at head |

## Decisions

- **`DEF-425` is parked** pending the user's ruling: the frozen BR-5 (`spec/16-borrow-rules.md:191`)
  and the concrete/take twins require `E10034`/`E10026`, while the batch-M spec/impl cases pin the
  generic borrow-element spelling accepted. An immutable spec-oracle change needs an explicit ruling;
  the park evidence is recorded in the register and the phase-5 record.
- **The two false positives** (`DEF-427`, `DEF-447`): the filed expectations do not reproduce (the
  double index selects a `u8`; the inline literal shape is LSan-clean). The controls pin the real
  behavior; a scalar-`len` residual row records the compiler fallback (no code this batch).
- **The scope**: 22 active rows (20 red + 2 controls) + the parked `DEF-425`; the remaining
  majors/minors are the later batches.
- No finding expanded the scope: no new diagnostic, no spec/registry edit, no new suite.

## Residual notes

- The probe sources are preserved under the gitignored `build/batchN-plan/probes/` tree.
- The phase-1 verification branch decides whether `DEF-431`/`DEF-435` close incidentally under the
  `DEF-429` gate or need their fallbacks; the close-out measures it.
- The specialist check was refreshed before phase 1 (outcome None; the four project agents cover the
  recurring areas).

## Confirmation

One bounded verifier checked the nine iteration-2 fixes, the cross-document consistency, and the
counts. Eight closed; one residual:

| ID | Status | Evidence |
|----|--------|----------|
| I2-001 | closed | the `is_raw_self_alias` recognizer (the AST `Deref(AddressOf(place))`, optional `Unsafe`; `place_target` stays Deref-free); the distinct-value pins; `q424self`/`q424inner` still red as planned |
| I2-002 | closed | the `Element(None)` wildcard match defined for the three readers (the `Ord`-keyed `Projection::Element(Option<usize>)`); the non-constant branch records it; the helpers in the blast radius |
| I2-003 | closed | ST-21 asserts acceptance + sanitizer cleanliness only; the exact `254`/`0` are implementation pins; the residual-ruling note |
| I2-004 | **residual → fixed** | the five stale "both aliasing selections run 0" lines corrected: the `g = true` selection runs 0, the distinct-length `g = false` selection runs 2 (it reads the stored `y`), the non-aliasing control runs 0 |
| I2-005–009 | closed | the `DEF-435` constant/runtime split, the `DEF-410` control (`T = P`), the AR-22 citation, the index line, the dropped `P4 → P5` arrow |

Counts: 23 dispatch rows = 22 active (20 red + 2 controls) + 1 parked; 22 requirements (R1–R22),
22 specification cases (ST-1–ST-22), 62 tasks (9+8+8+8+9+8+8+4), Progress `0/62`.

**BATCH N PREFLIGHT: PASSED.** The plan is ready for execution.
