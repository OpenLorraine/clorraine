# Preflight Report — Residual Major Completion (Batch M)

> **Audit target**: `plans/residual-major-completion/` — 14 documents
> **Audited revision**: commit `f43570b` (tree `3b8e75a57bd816c2413ffcefadf84e93a258910d`),
> 2,518 lines
> **Revised revision**: `9f1ed28` (iteration-1 fixes), then the iteration-2 fixes in the working
> tree (3,093 lines)
> **Mode**: strict scope; auto-design under the standing defect mandate; independent five-cluster
> fan-out, two iteration-2 verifiers, one iteration-3 fix-scoped verifier
> **Date**: 2026-10-17
> **CodeOps Artifact Schema**: 1

## Verdict

**✅ PREFLIGHT PASSED — all findings resolved.**

| Round | Findings | Status |
|-------|----------|--------|
| Iteration 1 (five clusters) | 64 raw → 32 merged (`PF-01`–`PF-32`: 3 critical, 15 major, 12 minor, 2 observations) | All resolved (`R1`–`R21`) |
| Iteration 2 (two verifiers) | 32 raw → 12 fix groups (`M1`–`M12`) | All applied |
| Iteration 3 (fix-scoped) | 12 (1 critical, 4 major, 7 minor) | All applied (`C1`–`C7`) |
| Iteration 4 (fresh session) | 10 (3 major, 6 minor, 1 observation; no critical) | All applied (`D1`–`D10`) |
| Iteration 5 (bounded confirmation) | 0 | **CLEAN** |

The iteration-3 critical was a regression introduced by an iteration-2 fix (`Unsafe(_) => true`); it
was corrected to the recursing arm and independently proven on a scratch build (the five
conditional-rooted forms and both unsafe-wrapped conditional forms report one `E10221`; the raw
prefixes emit the store and run 0; the workspace suite green). No critical or major finding
remains; the two pinned controls and the two parked majors are scope decisions, not findings.

## Scope

The audit target is the batch-M plan set: the 20 remaining un-parked major/critical defect rows
(`DEF-360`, `361` control, `363`, `364`, `365`, `367` critical, `368`, `371`, `372`, `373`,
`374` control, `375`, `376`, `377`, `378`, `379`, `384`, `385`, `386`, `390`). The confirmed
baseline excludes the two ruling-parked majors (`DEF-167`, `DEF-171`) and the minors. Context
documents: `plans/00-roadmap.md` (the row notes), `AGENTS.md`, the frozen `spec/`, the compiler
source, and the probing tree under `build/batchM-plan/`.

## Method

| Round | Dispatches | Raw findings | Result |
|-------|------------|-------------|--------|
| Iteration 1 | 5 clustered auditors (① soundness · ② grounding · ③ delivery · ④ risk · ⑤ fit) | 64 | Merged to `PF-01`–`PF-32`; 21 resolutions applied (`R1`–`R21`) |
| Iteration 2 | 2 verifiers (soundness/fit; grounding/risk/delivery) | 32 | Merged to 12 fix groups (`M1`–`M12`); all applied |
| Iteration 3 | 1 fix-scoped verifier | appended below | Fix-scoped to `M1`–`M12` and their dependency surface |

Every auditor read the artifact directly, probed the compiler at the planning head, and cited the
frozen text; no auditor fixed anything. The lead merged, deduplicated, and resolved.

## Iteration 1 — merged findings (PF-01…PF-32)

| ID | Severity | Finding | Resolution (applied) |
|----|----------|---------|----------------------|
| PF-01 | 🔴 | `DEF-375`'s store-only gate leaves the return, aggregate-field, array-literal-element, and conditional-arm deref positions double-freeing (`own/statements.rs:490`) | One shared owning-deref materializer across every consuming position; field/aggregate source moved (pointee zeroed); position probes + controls (`R1`, `M4`) |
| PF-02 | 🔴 | `DEF-367`'s generic-borrow spelling cannot resolve the target type (`element_ty = self.ty(target)` at declaration), and the compound/update/deep-chain/nested/multi-index stores are silently dropped; the target-before-RHS order is unpinned | Per-instance store-side type resolution (`resolved_value_ty` on the `AddressOf` operand), generalized target builder for every store form, always-materialized base/index, generic/nested/multi-index/bounds/order cases (`R2`, `M5`) |
| PF-03 | 🔴 | `DEF-373`'s per-arm `move_element_loans` replaces the destination record, losing the first arm's loans; the conditional-element-inside-an-array-literal composition is also unsafe | Snapshot both arms, write the loan union once; recurse into conditional elements; mixed-position/different-count/fill cases (`R3`, `M7`) |
| PF-04 | 🟠 | `DEF-361`'s "module-own" control is the `DEF-363` import/declaration collision, which P1's own fix must reject | Remove the shape from the control matrix; keep the six verified direct-name fixtures; the collision stays a `DEF-363` red (`R4`, `M12`) |
| PF-05 | 🟠 | `DEF-384`'s "one `E10011`" oracle contradicts the every-node check and duplicates the existing declaration reports; the call-site type argument and nested positions are uncovered | Scope the check to nested type-argument positions plus an explicit call-site argument check; split the probes; five-site duplicate-count; nested and `Array<String?>` key pins (`R5`, `M6`) |
| PF-06 | 🟠 | `DEF-386`'s fit unit is unresolved: the operand mechanism rejects `i8 = -128`, and the `u8 = -5` = 251 pin is an open reading | Fold the negated literal first, one `int_fits` check gated by `is_integer_scalar`; `-128` accepted, `-129`/`u8 -5` one `E10202`; the 251 pin removed; eight-position matrix (`R6`, `M3`) |
| PF-07 | 🟠 | `DEF-364`'s direct null-arm conditional is accepted; the claimed expected-type plumbing does not exist in `check_conditional` | Pass the expected type into `check_conditional` and both arms; four-context pins; concrete disagreeing control (`R7`, `M9`) |
| PF-08 | 🟠 | `DEF-360`'s deferred-conversion duty binds the enclosing generic function's type arguments, rejecting a valid dot call; the generic-caller control was invalid | Infer the callee's type arguments at the call site from the receiver and written args; rebuild the control with `c: Box`; keep the invalid shapes as rejection cases (`R8`, `M2`) |
| PF-09 | 🟠 | `DEF-390`'s named qualified arm is the func-field path; the free-function name is computed in `lower_call` via `callee_name`; the identity needs the modes/params/return lookups | Correct the site map; thread the identity across `codegen/calls.rs`, `expr.rs`, `function.rs`; add the qualified func-global control (`R9`, `M12`) |
| PF-10 | 🟠 | `DEF-379`'s diagnosis names the guard-literal carrier, but the eager fill is the by-value argument binding (`materialize_owned_argument`, `own/lower.rs:5109`), emitted by `codegen/guards.rs`; no loop fixture | Name the real carrier and sites; defer the binding under `defer_move_outs`; drop/re-fill/release loop case + eager-order control (`R10`, `M11`) |
| PF-11 | 🟠 | `DEF-377`'s `is_place` recipe omits the Member-arm recursion and leaves the Index `|| Unsafe` bypass; the claimed green controls are red at head | Add `Member => is_place(base)` and `Unsafe(inner) => is_place(inner)`; remove the bypass; five `E10221` forms; true block controls; prefix-form outcome stated; `(*p).f` residual recorded (`R11`, `M1`) |
| PF-12 | 🟠 | `DEF-376`'s take-deref materialization runs at statement entry even under `defer_move_outs`, so `g = false` still calls an impure pointer; the `Materialize` channel's exhaustive matches are unnamed | Gate on `defer_move_outs == 0`, carry the pointer fill deferred, name the IR/codegen/own sites; impure both-selection cases (`R12`) |
| PF-13 | 🟠 | `DEF-367`'s bounds case is routed to the frozen, spec-data-driven `run_fault_spec_test` | Move to the residue suite with the inline `assert_traps` helper; the Amiga fault list untouched (`R13`) |
| PF-14 | 🟠 | The written-argument hook has two owners (P1 vs P3) and the dependency diagram contradicts its legend/text | Split by call path (P1 dot-call, P2 conditional, P3 plain literal); redraw the diagram consistently; add the P2→P3 arm dependency (`R14`, `M9`) |
| PF-15 | 🟠 | The P2 hosted-amiga/ASan legs, the P3 nullable-handle ASan run, and the `DEF-364` profile control are missing or ineligible | Add them; use the no-import control for the freestanding leg; record the freestanding exclusions (`R15`, `M10`) |
| PF-16 | 🟠 | The base/head emission differential is not an operational gate; the ASan/strict command spellings do not cross the container boundary | Per-phase differential with named probe sets; corrected `CMD="…"` spellings (`R16`) |
| PF-17 | 🟠 | The close-out syncs only the 20 planned rows; rows filed during execution and the likely incidentals (`DEF-380`, `DEF-382`) are unaccounted | Extend the close-out to every filed row with evidence, probe the incidentals, record incidental vs open, move the controls to Done (`R17`, `M12`) |
| PF-18 | 🟠 | The P1 lens set omits the memory-safety auditor for a symbol/IR change; the index lens table contradicts AR-27 | Add the P1 memory lens; align all lens tables (`R19`) |
| PF-19 | 🟡 | `DEF-363`'s "every declaration kind" is untested (only `let`) | Declaration-kind matrix (func/struct/const at minimum), both orders, one `E11103` each (`R18`) |
| PF-20 | 🟡 | Missing pins: the R4 shadow control, the `ClArray__StringQ` key, the nested `DEF-384` shapes | Add all (`R18`, `M6`) |
| PF-21 | 🟡 | Several ST controls lack a concrete fixture or base observation | Name the fixture and the observed value for each (`R18`, `M12`) |
| PF-22 | 🟡 | The testing strategy allows re-pinning a committed spec case as a "legitimate emission delta" | Re-pinning restricted to `_impl_test.rs` byte-identity pins; spec cases never edited (`R20`) |
| PF-23 | 🟡 | `DEF-378`'s causal narrative is wrong (the predicate already returns true; the unguarded builder never consults it) | Correct the narrative; name the codegen value path and its predicate (`R21`, `M8`) |
| PF-24 | 🟡 | Task 3.2.2 says "park" on a legal-nullable false positive; AR-34(b) says narrow | Change to "narrow the check" (`R21`) |
| PF-25 | 🟡 | "Severity-first (the critical first)" contradicts the P4 placement | Reword: severity-first scope; P4 fourth by dependency order (`R21`, `M12`) |
| PF-26 | 🟡 | The controls' no-code-change instruction lives only in the designs | Add it to the implement tasks (`R21`) |
| PF-27 | 🟡 | The phase-record task `n.4.1` sits inside `Step n.3` | Add `Step n.4` headers (`R21`) |
| PF-28 | 🟡 | The index Specialist Agents table contradicts AR-27 (P4 semantics, P5 emission) | Align the table (`R19`) |
| PF-29 | 🟡 | Reference and citation drifts across the docs (wrong lines/sections) | Correct every citation and site line (`R21`) |
| PF-30 | 🔵 | The two control rows' roadmap status is unspecified | Move to Done with control-pin evidence at close-out (`R17`) |
| PF-31 | 🔵 | AR-26 pre-authorizes a new suite pair despite the no-new-suite scope | Delete the contingency; a case with no home needs a register entry + approval (`R20`) |
| PF-32 | 🔵 | AR-34(a) permits parking the critical `DEF-367` | Convert to a phase blocker + user ruling (`R21`) |

## Iteration 2 — merged fix groups (M1…M12)

| Group | Findings | Finding | Resolution (applied) |
|-------|----------|---------|----------------------|
| M1 | I2S-002, I2G-001, I2G-012 | `DEF-377`'s fix list omits the Member-arm recursion (the five red forms stay places); the `Unsafe` transparency flips the prefix forms to accepted without a stated expectation | Add the Member recursion to AR-7/R13/task 4.2.1; pin the five forms; state the prefix acceptance and add the cases |
| M2 | I2S-001, I2G-002 | The rejecting direct twins were listed as "run 0"; the generic-caller control passed `c: i32` (invalid) | Correct the expectations; rebuild the control with `c: Box`; keep the i32 shapes as rejection cases |
| M3 | I2G-003, I2S-005 | The operand-context mechanism rejects `i8 = -128` and `int_fits` over-accepts f32/bool; the matrix counts disagree (seven/eight) and `i8 = -5` is uncased | Fold-first mechanism with the integer-scalar gate; eight positions; `-128`/`-129`/f32/bool pins |
| M4 | I2G-004, I2G-005, I2S-010 | `DEF-375` misses array-literal elements and conditional arms; the field source needs a real pointee move-out; values were not concrete | Extend the materializer; field/aggregate move + empty; `b375_arr`/`b375_cond` + controls; run 12 |
| M5 | I2G-006, I2G-014, I2G-015, I2S-012 | `DEF-367` misses nested generic member and multi-index stores; the wrong type helper is named; the update path still drops silently; the `None` fallback is an either/or; no profile compile | Codegen-side resolution; nested/multi-index probes; no-silent-drop on both paths; debug assertion/pinned unreachable; P4 hosted-amiga compile |
| M6 | I2G-007, I2G-016, I2S-008, I2S-011 | `DEF-384`'s site cannot cover explicit call type arguments; the count case has five sites, not four; ST-4 lacks the shadow fixture | Call-site argument check; five-site count; shadow fixture; citation fixes |
| M7 | I2G-008 | `DEF-373` misses a conditional element inside an array literal | Recurse per element; `b373_mix`/`diff`/`fill` |
| M8 | I2G-009, I2S-016 | `DEF-378`'s read-position fix is in `codegen/function.rs` (`struct_literal_needs_stores`), not the named lowerer predicates; the `ir.rs` hedge | Add the codegen path + predicate; state the `ValueKind::Materialize` carrier |
| M9 | I2G-010 | The P2→P3 conditional-arm dependency is unstated | `check_conditional` passes the expected type into both arms; P2→P3 recorded |
| M10 | I2G-011, I2S-003 | The freestanding `DEF-364` control imports `std.text` (`E11200`) | Add the no-import `m364_control_fs`; fix the AR-28 wording |
| M11 | I2G-013 | `DEF-379`'s re-executed guard has no loop fixture | Add `b379_loop` (run 2/calls 6 at head) + the eager control |
| M12 | I2S-004/006/007/009/013/015, I2G-016 | Bookkeeping and precision: the sixth fixture, the `DEF-382` causal claim, kind file names, quotes, the duplicated paragraph, command spellings, ST-2/10/14/21 precision, `m367_gen_elemset`, the `DEF-380` gridel boundary, the severity wording | All corrected; `DEF-382` recorded open; the `DEF-380` boundary recorded as a close-out measurement |

## Decisions (auto-design)

- Every finding was resolved with the auditor's recommended option; no reserved decision was
  required. None of the resolutions adds a language surface, a diagnostic code, a registry row, or
  a spec edit; the two spec-adjacent rulings were verified against the frozen place list and the
  batch-L precedent.
- `DEF-386`'s reading is recorded in AR-16: the fit is judged on the folded negated value
  (`int_literal_value` + one `int_fits`, gated by `is_integer_scalar`); `u8 = -5` reports `E10202`;
  the earlier 251 wrapper pin is withdrawn.
- `DEF-382` is recorded open: the `DEF-371` fix is scoped to concrete parameters and does not
  substitute explicit type arguments, so `identity<u8>(5)` is not incidentally closed.
- `DEF-380` (the multi-extent `str[2][2]` call-row field) is probed at close-out; the `gridel`
  case either closes it incidentally or records it open, and the next-batch count is adjusted.
- The two controls (`DEF-361`, `DEF-374`) remain pinned only; their roadmap rows move to Done with
  the control-pin evidence at close-out.

## Residual notes

- The plan stays at 0/60 tasks and `Status: Planning` until execution starts; this report records
  the pre-execution audit, not any row closure.
- The five clusters' per-finding evidence is retained in the phase dispatches and the probe tree
  under `build/batchM-plan/`; the merged tables above carry the decisive evidence.
- The iteration-3 result is appended below with the final verdict.

## Iteration 3 — fix-scoped verification

One verifier, scoped to `M1`–`M12`. **12 findings: 1 critical, 4 major, 7 minor.** The critical was a
regression introduced by the `M1` fix itself.

| ID | Severity | Finding | Resolution (applied) |
|----|----------|---------|----------------------|
| I3-001 | 🔴 | `M1`'s `Unsafe(_) => true` is not place-transparency: it accepts conditional-rooted stores the closed place list rejects, and the store is silently dropped; the "accepted/run 0" oracle cannot fail on the drop | Restore `Unsafe(inner) => is_place(inner)`; the raw prefixes (`unsafe p.f`, `unsafe p.f.x`) are accepted with a store-emitting read-back; the conditional-rooted spellings report one `E10221` (`C1`) |
| I3-002 | 🟠 | Five dependent docs still said rejection (or acceptance) inconsistently with the `M1` oracle | Align `03-04`, the index, the findings row, the P4 deliverable, and `02`; drop the stale control counts (`C1`) |
| I3-003 | 🟠 | Task 3.2.1 still instructed the superseded operand-context mechanism | Rewrite to the fold-first mechanism (`C2`) |
| I3-004 | 🟠 | Four "three"-shape summaries contradicted the five `DEF-375` positions | Update all four to the five shapes; add the array/conditional read-back (`C3`) |
| I3-005 | 🟠 | The `DEF-367` error table still permitted the forbidden `E10221` fallback | Delete the option; debug assertion or pinned-unreachable proof (`C4`) |
| I3-006 | 🟡 | The error table described only the pointer pin, not the required pointee move | State the pin **and** the pointee zeroing (`C3`) |
| I3-007 | 🟠 | `DEF-378`'s causal narrative still blamed the lowerer predicate; the fix is the unguarded codegen builder | Rewrite AR-22/`03-07`; add the `struct_literal_needs_stores` read path to task 7.2.1 (`C5`) |
| I3-008 | 🟠 | `DEF-376`'s carrier cannot hold the consumed non-place source (`Materialize.move_outs` is `Vec<Place>`) | Add `value_move_outs: Vec<Value>`; emit `MoveOutValue` in both guard forms; keep `move_outs` for places (`C6`) |
| I3-009 | 🟡 | AR-23 and the findings row omitted the loop expectation (`b379_loop` calls = 4) | Add it plus the eager control (`C7`) |
| I3-010 | 🟡 | Two dependency summaries omitted P2→P3 | Add it to `01` and the `99` overview (`C7`) |
| I3-011 | 🟡 | Design bullets, findings rows, and index surfaces lagged the updated ST cases | Align all (`C7`) |
| I3-012 | 🟡 | The legal nullable `raw`/`cstring`/view controls had no fixture | Narrow the claim to the verified `Array<String?>` (allowed option; `C7`) |
| I3-013 | 🟡 | The recorded UAF run values are not reproducible; the sanitizer is the stable oracle | Replace with "varies; ASan heap-use-after-free" (`C7`) |

`M1`–`M12` status at iteration 3: `M2`, `M6`, `M7`, `M9`, `M10`, `M12` applied; `M1`, `M3`, `M4`,
`M5`, `M8`, `M11` partially applied, each with the specific corrections above. All `C1`–`C7`
corrections were then applied (the plan text now carries the corrected mechanism, shapes, carriers,
oracles, and bookkeeping).

## Iteration 4 — fresh-session verification (scoped to `C1`–`C7`)

One fresh verifier, which also **independently applied the planned `is_place` change to a scratch
copy and rebuilt it**: the five conditional-rooted forms and both unsafe-wrapped conditional forms
each report one `E10221`; `unsafe p.f` / `unsafe p.f.x` compile, emit the store, and run 0; the
full workspace suite is green on the patched copy. **10 findings: 0 critical, 3 major, 6 minor, 1
observation** — all stale dependent-document surfaces.

| ID | Severity | Finding | Resolution (applied) |
|----|----------|---------|----------------------|
| I4-001 | 🟠 | The testing-strategy coverage goal says the unsafe-wrapped forms are accepted (the conditional forms must reject) | Reword to the conditional-rejection / raw-prefix-acceptance split (`D1`) |
| I4-002 | 🟠 | `02`'s `DEF-378` fix direction still names the lowerer predicates | Name `struct_literal_needs_stores` at the unguarded codegen read path (`D2`) |
| I4-003 | 🟠 | `02`'s `DEF-376` fix direction still says `move_outs`; the index omits `value_move_outs` | Correct both to `Materialize.value` + `value_move_outs: Vec<Value>` (`D3`) |
| I4-004 | 🟡 | Three surfaces still say three `DEF-375` shapes | Add the array-literal element and conditional arm (`D4`) |
| I4-005 | 🟡 | Two UAF run values remain recorded as reproducible | Replace with "varies; ASan heap-use-after-free" (`D5`) |
| I4-006 | 🟡 | The `DEF-377` mitigation still says the `unsafe p.f` prefix "is not claimed" | Reword to the post-fix acceptance + read-back (`D6`) |
| I4-007 | 🟡 | The P5 heading is missing from the testing strategy | Restore it (`D7`) |
| I4-008 | 🟡 | The P4 impl-pin task no longer names the unsafe-wrapped conditional spans | Restore the phrase (`D8`) |
| I4-009 | 🟡 | The `m384_count` case is grouped with the accepted cases | Split it out (clc 1, three `E10011`) (`D9`) |
| I4-010 | 🔵 | The `Unsafe` transparency also accepts `unsafe (*p).f`, which still drops the store | Name both spellings in the residual row and probe both when filed (`D10`) |

## Iteration 5 — bounded confirmation (scoped to `D1`–`D10`)

One fresh verifier checked the ten corrections, grepped the plan set for the stale patterns, verified
the counts (20 R, 38 AR, 21 ST, 60 tasks, `Progress: 0/60`), and re-ran three probes.

| Check | Result |
|-------|--------|
| `D1`–`D10` | All PASS (the corrected text is present and consistent) |
| Stale patterns (`Unsafe(_) => true`, "all three", a `move_outs` pointer carrier, the lowerer-predicate `DEF-378` fix, accepted unsafe-wrapped conditionals) | 0 in the planning docs (only the historical quote in this report) |
| Counts and the six `DEF-361` fixtures | Consistent |
| Probe re-runs (`m377` five forms, `m384_count`, `b379_loop`) | Match the recorded head values |

**ITERATION 5 (confirmation): CLEAN.** The plan is ready for execution.



