# Findings Ledger: Critical/Major DEF Closeout

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| — | — | — | Awaiting execution | — |

## Preflight Rulings

The iteration 1 preflight report (`00-preflight-report.md`) records 28 findings
(6 critical, 16 major, 6 minor) and the delegated technical resolutions. The
user ruled the reserved items on 2026-10-24:

| Ref | Ruling | Date |
|---|---|---|
| R-1 / PF-001 | Add `DEF-458` to Batch B; the active set is 34 (1 critical + 33 major), 35 DEF rows including `DEF-470` | 2026-10-24 |
| R-2 / PF-003 | Add a parse-only/stage-select mode for the `DEF-167` stage test; the full compile reports E10300 | 2026-10-24 |
| R-3 / PF-006 | Close `DEF-416` by the conservative collision report (E11101), not module-qualified identity | 2026-10-24 |
| R-4 / PF-021 | Restore the roadmap row evidence/header; phase closes go through the roadmap skill | 2026-10-24 |
| R-5 / PF-018 | Revalidate `DEF-506`/`507`; use the conservative proof; a summary surface needs a new ruling | 2026-10-24 |
| AR-22 | Apply the delegated fixes; the plan documents are corrected | 2026-10-24 |

## Iteration 2 Preflight Rulings

Preflight iteration 2 (2026-10-24) verified the iteration-1 corrections and
found residual and new defects. The report `00-preflight-report.md` carries the
full evidence. The applied corrections are:

| Ref | Ruling | Date |
|---|---|---|
| PF-002 | Add the constant-context flag; a module `const embed` reports only `E10224` | 2026-10-24 |
| PF-003 | The full compile of the filed local-i32 stage shape reports one `E10300`; keep the parse-only stage mode | 2026-10-24 |
| PF-004 | Put the source set on `PlaceState`; strong-update on assignment, join in `merge`, prune with the place; keep the destination-root, take, and type-parameter rules | 2026-10-24 |
| PF-005 | Test source-vs-destination containment; clear the taken subrange of the save copy; attach the range to the capture | 2026-10-24 |
| PF-006 | Report the `DEF-416` collision from `sema` with a pure `TypeTable` query, not the table constructor | 2026-10-24 |
| PF-008 | The nullable conditional loop head is a mandatory `E10012` case | 2026-10-24 |
| PF-010 | Complete the remaining ST sources, unsafe contexts, exit values, and strict-C assertions | 2026-10-24 |
| PF-011 | Pin the `DEF-522` arm backing and the exact `E10030` local-arm code | 2026-10-24 |
| PF-013 | Define the must-source walk for the cross-frame scan | 2026-10-24 |
| PF-017 | Thread the `DEF-472`/`473` destination through `statements.rs`, `place.rs`, and `mono.rs` | 2026-10-24 |
| PF-019 | Extend the test-impact inventory and add a retarget line to each phase close | 2026-10-24 |
| PF-023 | Require a struct declaration order that differs from the source order in ST-20 | 2026-10-24 |
| PF-029 | Allow the `DEF-506`/`DEF-507` stale green pin in R6 and the acceptance criteria | 2026-10-24 |
| PF-030 | Build the probe binary before the Phase 0 evidence probe; author the source-set pin after the field exists | 2026-10-24 |
| PF-031 | Use the pinned destination index and the run-time source shape in ST-9 | 2026-10-24 |
| PF-032 | Add the four new test pairs to the AGENTS.md test-naming map | 2026-10-24 |
| R-6 / AR-23 | User chose **follow `spec/04` §16**: a module-level `let embed` reports `E10213`; the `spec/13` §1 sentence is recorded as a frozen inconsistency; no chapter change | 2026-10-24 |

## Red-Phase Evidence

Every active row records one line per ST case before its fix:

| ST | DEF | Command | Base commit | Observed failure |
|---|---|---|---|---|
| ST-N | DEF-N | `cargo test --test def_*_integrity_spec_test <name>` | `<sha>` | `<diagnostic, ASAN/LSan, strict-C, or trace>` |

ST-12 and any AR-16 stale close record the HEAD command and the accepted result
instead of a failure.

## Phase 0 — Revalidation and Baseline

**Baseline (2026-10-25):** `cargo build --workspace` passed; `make check` passed
(spec check plus the full Rust workspace); `make verify` passed every leg after
refreshing the gitignored `docker/.env` XAUTHORITY cookie for the FS-UAE leg
(environment-only change, not a repository artifact). Phase baseline tree
`bdf231350a5e3371a40a524c0f9dc5ddc0d6ee55`.

**Roadmap check:** all 34 active rows link to this plan, keep their
severity/evidence notes, and `DEF-470` is Done. The 34 rows moved from
`Plan Preflighted` to `Executing` at phase start.

**Active count:** 34 active rows (1 critical + 33 major) plus the stale
`DEF-470` close = 35 DEF rows. 35 ST rows and 76 tasks.

### Evidence probes

| DEF | Filed shape | Phase 0 result |
|---|---|---|
| DEF-543 | `func fill(p: str, c: bool, borrow slot: str?) { let s: str = p; if (c) { s = "AA"; } slot = s; }` called same-frame | Accepted; strict C builds — defect reproduces |
| DEF-167 | `let call_shaped = a < b > (c);` with i32 locals | Accepted; emits `call_shaped = a(c);`; strict C fails — defect reproduces |
| DEF-171 | `let data: u8[16] = embed("assets/logo.bin");` in a function body | Accepted; emitted C fails strict C — defect reproduces |
| DEF-470 | `take h: H; let r: String? = c ? h.name : null; let s = h.name;` | One `E10020` — stale, close with the HEAD proof |
| DEF-506 | `struct H { name: String; tag: static str; }`; `viewTag(borrow h: H): str { return h.tag; }` | One `E10036` at the callee return — reproduces; not stale |
| DEF-507 | `always(xs: str[2]): str { return "ZZ"; }` then `return always(xs)` | Accepted; strict C builds — stale, close with the HEAD proof |
| DEF-458 | `v = (g \|\| eat(take v) == 2) ? x : y` with `g = true` | Iteration-1 HEAD probe: accepted; 11-byte LSan leak. The code is unchanged at the same HEAD |

The remaining 28 active rows (`DEF-411`, `412`, `413`, `415`, `416`, `454`,
`455`, `456`, `457`, `460`, `461`, `462`, `464`, `465`, `466`, `471`, `472`,
`473`, `475`, `476`, `477`, `478`, `479`, `480`, `481`, `505`, `513`, `522`)
carry the iteration-1 HEAD evidence and the roadmap's filed shapes. Each batch's
specification task re-confirms its rows red before the fix; a row that no longer
reproduces is closed stale under AR-16 with a HEAD proof.

**Stale-row decisions:** `DEF-470` and `DEF-507` close stale with the proofs
above. `DEF-506` stays active; its safe shape is not accepted today.

### Existing-test inventory

| Family | Expected change | Owning phase |
|---|---|---|
| `stage_spec_test.rs` | `comparison_and_generic_call_forms_parse` uses the new parse-only mode | 5 |
| `compile_fail_spec_test.rs` | E10213 moves from `EXCLUDED_CODES` to `ASSERTED_CODES`; E10214 stays excluded | 5 |
| `checker_conformance_spec_test.rs` (`qinitstructname`) | moves from `emit_pair` success to the `E11101` rejection | 5 |
| `borrow_generic_integrity_spec_test.rs` (`generic-mixed`/`-local`) | report counts and any accepted controls change with DEF-472/473 | 3 |
| `borrow_store_integrity_*` | out-slot controls gain the source-set judgments | 1 |
| `conditional_projection_integrity_*`, `conditional_projection_residue_*` | fresh-root, per-arm, and read/return pins may change | 3/4 |
| `store_emission_integrity_*`, `call_value_integrity_*` | conditional store and guard shapes may change | 2/3 |
| `runtime_value_integrity_*`, `eval_emission_integrity_*` | release and evaluation-order pins may change | 2/3 |
| `module_initializer_integrity_*`, `callable_release_integrity_*` | initializer and callable C pins may change | 3/4 |
| `own`/`sema` unit tests | source-set, conditional, and type-argument internals | 1/3/5 |

Each phase close retargets the tests in this inventory and records every
expected change.

## Phase 1 — Critical Flow

**Red phase (2026-10-26):** four of the six new ST-1 rejection cases failed at
HEAD — the same-frame mixed branch, the parameter-first mixed union, the
conditional assignment, and the cross-frame form. The local-root and
later-assignment cases were already green at HEAD (the declaration provenance
happened to carry the right root), so they are labeled regression controls, not
red gates. The definite-overwrite, all-static, slot-copy, take-root, and
take-view controls passed. `RDC_ASAN=1` is not run on a rejection case.

**Implementation:** the source set lives on `PlaceState` and is seeded at the
declaration, strong-updated by every assignment, joined by `merge`, and pruned
with the place. `source_atoms` follows a rebound copy to the storage it holds;
the whole-out-slot judge reads a bare rebound name, reports the foreign
parameter first, skips the destination's own parameter and a `take` root, and
defers a type-parameter pair. The callee scan walks the body with a
flow-sensitive source map (branch, loop, switch fall-through, and defer-exit
joins), withholds the caller guard only for a store shape the callee's judge
consumes, and isolates lambda-body writes. An owned `Local`/`Temp` transfer is
accepted; only a view context reports the frame escape.

**Runtime ruling AR-24:** the generic `T = str?` whole-slot copy reports
`E10030` while the concrete twin is accepted. Batch A stays concrete-first; the
pre-existing false rejection is recorded here for a future plan and is not part
of this batch.

### Phase review (2026-10-26)

| Finding | Severity | Area | Ruling |
|---|---|---|---|
| SA-001 / SR-001 | CRITICAL | The scan withheld the caller guard for a conditional/projection/call store or a lambda-body write while the callee accepted, leaving an accepted use-after-free | Fixed: the set is consulted only for a bare rebound name; lambda bodies walk a cloned map; regression tests added |
| SR-002 | MAJOR | A legal owned whole-slot transfer was reported `E10030` | Fixed: an owned `Local`/`Temp` transfer is accepted; only a view context reports the frame escape; regression test added |
| SR-003 / SA-004 | MAJOR | Duplicate reports for a defer, do-while, or switch fall-through store | Fixed: defer blocks walk at the scope exit, loops get a second carried pass, switch clauses carry the fall-through state; regression tests added |
| RV-001 / RV-002 / SR-004 | MINOR | The red-phase record overclaimed two base-green controls; the mixed-union fixture used a static local; the impl pin was absent | Fixed: the record is corrected, the fixture uses a local owner, and the cross-frame impl pin is added |
| RV-003 | MINOR | The scan's name-keyed map does not restore a shadowed local's previous source | Recorded residual: a shadowed inner local can add one duplicate report; it does not accept an unsafe store |
| RV-004 | MINOR | The scan cannot distinguish a proven-static callee store, so a cross-frame call keeps the caller guard | Recorded residual: conservative false rejection, no unsafe acceptance |
| SA-002 | CRITICAL (pre-existing) | A same-frame composite store value (`slot = c ? s : "BB"`) was accepted and UAF'd | Fixed under AR-25: the set judge resolves conditional, projection, call, and literal values; regression test added |
| SA-003 | CRITICAL (pre-existing) | An element store into a local view array did not update the source state | Fixed under AR-25: the element store joins the base binding's set; regression test added; a sibling element read is conservatively judged from the union |
| PE-1…PE-4 | MINOR (pre-existing) | Captured writes, `for`-step stores, the generic instance duty, and temporary views keep their base behavior | Recorded residuals |

**Roadmap:** `DEF-543` is Done at `f02e0e1`; the roadmap row links the phase
commit and the verification evidence.

**Verification:** `make check` and `make verify` pass after the review fixes;
`borrow_store_integrity_spec_test` 58 passed, `borrow_store_integrity_impl_test`
14 passed, `own` unit tests 62 passed. The accepted controls are ASAN clean. The
AR-25 fixes add the composite-store and element-store UAF regressions.

## Phase 2 — Raw Store Integrity

**Red phase (2026-10-26):** 19 specification cases. Fifteen fail at HEAD with the
filed failure mode: `guardedarg` double-free, `arraytake` double-free,
`nestedarg` double-free, `selfalias`/`selfaliasmember` SEGV, `shortcircuit`
11-byte LSan leak, `containsup` double-free, `containfirst` leak,
`arrayparam`/`arrayaddr` use-after-free, `selfread`/`selffill` wrong values,
`postsrc` accepted, and `elementalias`/`elementmove` SEGV. The controls pass:
`guardedargskip`, `fieldpin`, `postdst`, and `shortcircuittaken`.

`fieldpin` is a green runtime control because the host C compiler happens to
evaluate the store at the same field address; the defect is the missing pin in
the emitted C. The implementation test in 2.3.2 is the red oracle for that
row. The address-of fixed-array spelling could not be expressed as a source at
HEAD (`raw String[2]` conversion), so the concrete parameter alias is the
specification case and the adjusted-address emission is pinned in the
implementation test.

**Green and full verification (2026-10-26):** all eleven DEFs are fixed, the
19-case specification suite passes under `RDC_ASAN=1`, and the eleven
implementation cases plus the determinism pin pass. `make check` and
`make verify: all legs passed`.

| DEF | Commit | Fix |
|---|---|---|
| 454 | `bdc043d` | Guarded literal fills a temporary and moves the raw-deref field |
| 455 | `0339d16` | Fixed-array conditional take capture and saved-subrange clear |
| 456 | `0da2cf4` | Both containment directions over integer addresses; taken-subrange clear |
| 457 | `ca2b735` | Take-capture stack preserves every live outer save |
| 458 | `0ec766d` + `07f1e3e` | Place move-out records the take-ran flag on the evaluated path |
| 460 | `07c213a` | Raw self-alias restores the save and clears the duplicate |
| 461 | `b220003` | Adjusted array source address guards the saved target |
| 462 | `e387879` | Intermediate raw field pinned before the side-effecting RHS |
| 464 | review R1 | The conditional element source reports one `E10026` (the frozen v1 rule, AR-26); the same-place stable self-arm is a self-copy; the destination read control uses a non-element source |
| 465 | `46e8634` | Duplicable self-read fills a temporary before the copy, including the fill form |
| 466 | `039785a` | Element store compares source/target addresses; distinct source emptied |

Expected retargets: the conditional-projection `p439`/`p421` and root-binding
pins moved to the containment/range spelling; the call-value consumed-save pins
accept the take-flag memset; the raw-store implementation pins are new.

Residuals: a plain whole-binding raw-deref aliased store (`v = *p` where `p`
names the destination outside a conditional store) keeps the pre-existing
drop-before-RHS order; this batch covers the conditional and element forms. The
address-of fixed-array spelling cannot be expressed as a source at HEAD, so the
concrete parameter alias is the specification case. The `a[0]` fill twin keeps
the per-element fill expression; both elements read the old destination, so
`[3, 3]` holds.

## Phase 2 Review — 2026-10-26

Four lenses (correctness, memory safety, emission integrity, semantics) reviewed
the phase diff against baseline `9cbe86990b8d601a10a99e098a5a720d7d0382c4`.
The filed eleven DEFs' own specification and implementation cases are green, but
the review found new-code escapes and pre-existing sibling spellings. The phase
cannot proceed to Phase 3 until the rulings below.

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-001 | 2 | CRITICAL | take range: a C-adjusted array parameter's source extent was one element (`function.rs` / `guards.rs` extent helpers) | resolved (R4): the source extent uses the whole array; impl pin + ASAN probe |
| RV-002 | 2 | MAJOR | `constant_index` resolved a shadowing local as the module global (`own/expr.rs`) | resolved by AR-26 (the index-pin machinery was reverted) |
| RV-003 | 2 | MAJOR | wildcard element self-skip on distinct run-time indices (`own/expr.rs`) | resolved by AR-26 (same-place expression check; distinct indices E10026) |
| RV-004 | 2 | MAJOR | indirect callee and module-initializer global writes were not tracked (`own/call.rs`, `own/expr.rs`) | resolved by AR-26 (no global index pin remains) |
| RV-005 | 2 | MAJOR | a wildcard element move did not invalidate `initialized_elements` (`own/place.rs`) | resolved by AR-26 (the element initialized state was removed) |
| RV-006 | 2 | MAJOR | `mentions_call` missed a call nested in an index/member (`own/place.rs`) | resolved (R4): the walk follows index/member bases; the indexed RHS pin is pinned |
| RV-007 | 2 | MAJOR | six store-before-move-out ordering pins weakened to presence (`conditional_projection_residue_impl_test.rs`) | resolved (R4): the branch-aware ordering helper restores the pins (`5ae64df`) |
| RV-008 | 2 | MAJOR | the unplanned null-safe String shim made a moved source readable and was the ST-11 oracle (`runtime/shim_*.c`) | resolved (R3): the shim was reverted; the ST-11 non-alias control checks the sibling instead of reading the moved source |
| SR-001 | 2 | MAJOR | the conditional element move was accepted although `spec/03` §4 / `spec/07` STR-12 require `E10026`; ST-9 encoded the acceptance | resolved (AR-26): follow the frozen spec; ST-9 retargeted; the acceptance machinery reverted |
| SR-M-001 | 2 | CRITICAL | named-place fixed-array destination take creates no capture (`codegen/function.rs` fixed-array core); pre-existing, inside the changed function | resolved (R2): the capture accepts the named-place take and clears after the branch; ASAN spec + impl |
| SR-M-002 | 2 | CRITICAL | named-place take of a destination subobject creates no capture (`arm_consumes_target` exact spelling); pre-existing | resolved (R2): containment-based capture; the subobject range clears only the taken bytes; ASAN spec |
| SR-M-003 | 2 | CRITICAL | `take (cond ? named-dest : other)` loses the selected arm; the plan's literal ST-5 shape; pre-existing | resolved (R2): the taken conditional's arm move records the capture; ASAN spec |
| SR-M-004 | 2 | CRITICAL | fixed-array conditional `take (cond ? …)` tests the materialization temp, not the selected arm; pre-existing | resolved (R2): the fixed-array arm source move records the alias; ASAN spec |
| SR-M-006 | 2 | CRITICAL | a plain (borrow) parameter moved into an owned conditional store double-frees; outside the batch | filed as `DEF-544` (Backlog) |
| SR-E-003 | 2 | MAJOR | the `__cl_rhs` self-read temporary does not reach conditional arms or raw-deref destinations; pre-existing sibling of DEF-465 | resolved (R2): the temporary reaches conditional arms and raw-deref destinations; impl pins |
| SR-S-002/SR-S-003 | 2 | MINOR | callable-field index guard; computed destination index false rejection | resolved by AR-26 (the index-pin machinery was reverted) |
| RV-010/RV-013/SR-006 | 2 | MINOR | loosened pins; untested receiver/address-of spellings; latent array extent | report-only |

The user's rulings (2026-10-26): follow the frozen spec (R1, AR-26); fix the
pre-existing sibling spellings now and file SR-M-006 separately (R2); revert the
runtime shim (R3); apply the eligible new-code fixes (R4). All four batches are
implemented; each fix was red at the prior head and green with its tests. SR-M-006
is filed as `DEF-544`. The one-pass re-review ran; its criticals were ruled and
fixed or filed (`DEF-544`, `DEF-545`), and Phase 2 is closed.

## Phase 2 Review Fix Batch — 2026-10-26

| Batch | Finding | Fix | Evidence |
|---|---|---|---|
| R1 (`e062cec`) | SR-001, RV-002…005 | follow the frozen v1 element-move rule: a different or unstable destination index reports `E10026`; a same-place self-arm with a literal index stays a self-copy (refined by AR-27 in the re-review rulings) | raw-store ST-9 retargeted; conditional-projection spec 229 / impl 224 |
| R3 (`80e0986`) | RV-008 | revert the unplanned null-safe String accessors; observe the emptied source without reading it | raw-store suite 19/19 under ASAN |
| R4 (`1651cce`) | RV-001, RV-006 | array-parameter take extent is the whole array; `mentions_call` follows index/member bases | implementation suite 13/13; extent probe ASAN-clean |
| R4 (`5ae64df`) | RV-007 | restore the six store-before-zero ordering pins with a branch-aware helper | conditional-projection implementation 224/224 |
| R2 (`fix batch`) | SR-M-001…004 | named fixed-array/subobject/conditional takes create a capture; the fixed-array arm source and the taken conditional arm record the alias; the fixed-array path clears after the branch | four ASAN spec cases plus the implementation pins; each fails at the prior head |
| R2 (`fix batch`) | SR-E-003 | the `__cl_rhs` self-read temporary reaches conditional arms and raw-deref destinations | two spec cases and two implementation pins; both red at the prior head |
| R2 (filed) | SR-M-006 | plain borrow parameter moved into an owned conditional store | `DEF-544` (Backlog, critical) |
| Re-review corrections | RR-001/004/005/006/008/009/010 | recursive index stability, total call walk, storage-arm self-alias guard, raw-read temporary, docs/counts | each red at `2a40ff6`; raw-store 29/29 ASAN, implementation 17/17, `make verify` all legs |
| Re-review rulings | RR-002/003, P1, P2 | literal-index self-copies; generic per-instance element duty; conservative take capture; multi-range DEF | raw-store 29/29 ASAN, conditional-projection 230/224, `make check`; `DEF-545` filed |

## Phase 2 Re-review — 2026-10-26

One-pass re-review over the fix batch (`d270045..2a40ff6`) with the memory-safety,
emission-integrity, and correctness/semantics lenses. The batch's own tests are
green and every corpus emission diff is explained, but the re-review found one
regression introduced by the batch and same-class escapes that keep the Phase 2
gate closed.

| Finding | Severity | Finding | Status |
|---|---|---|---|
| F1 / RR-005 | MAJOR (new) | a fixed-array conditional arm that aliases the destination through a raw pointer or array parameter recorded a plain move as a take; the save was zeroed and the handles leaked (read form: SEGV) | resolved: every storage arm gets the runtime self-alias guard; `rawwholealias`/`paramwholealias` ASAN cases, red at `2a40ff6` |
| RR-001 | CRITICAL (new) | the destination-index stability check inspected only the outer index of a multi-index target | resolved: the chain is checked recursively; `nestedidx` E10026 case, red at `2a40ff6` |
| RR-004 | MAJOR (partial RV-006) | `mentions_call` missed calls under unary, binary, assignment, struct/array literal, intrinsic, and template wrappers | resolved: total call walk; `binarypin` implementation case, red at `2a40ff6` |
| RR-006 | MAJOR (partial SR-E-003) | the raw self-read temporary only recognized the same pointer root; alias and call-rooted pointers overwrote the partly written array | resolved: a literal containing any raw read uses the temporary; `rawaliasrhs` spec + impl, red at `2a40ff6` |
| RR-002 | CRITICAL (pre-existing, AR-26 soundness) | a local index mutated by the store's value or condition is treated as stable; the accepted store moves a different element, so a later read can dereference freed storage | resolved (AR-27): only a literal index proves a self-copy; a local, parameter, or computed index reports `E10026`; the local-index self-arm tests are retargeted, the literal self-arm stays accepted, and `selflocal` pins the rejection |
| RR-003 | CRITICAL (pre-existing, AR-26 gap) | the generic twin accepted a conditional element move and the later read, while the concrete twin reports `E10026`; the instance audit recorded no conditional-element duty | resolved (AR-28): the maybe-owning arms are paired with their expressions, the precise self-copy rule applies, and one per-store element-move duty reports `E10026` for an owning substitution; the duplicable instance is accepted (`genericcondmove`) |
| P1 | CRITICAL (pre-existing) | a named fixed-array parameter take created no capture when no raw pointee appeared in the condition; the saved destination was released after the callee freed it. The RV-001 implementation test masked this through its `(*p).flag` read | resolved (AR-28): any owning materialization move-out creates the capture; the runtime range decides; `arrayparamtake` covers the no-pointee case and the extent test no longer relies on the raw guard |
| P2 | CRITICAL (pre-existing) | one `(offset,length)` slot cannot clear two consumed subranges of one save; two takes in one condition double-free (the last range is cleared, earlier ranges are freed again) | filed as `DEF-545` (Backlog, critical, AR-28) |
| RR-007 | MINOR | two element-source arms of one conditional store report `E10026` twice at the same span | report-only: the concrete twin still reports per arm; the generic duty dedupes per store. A store-site dedupe is cosmetic |
| RR-008 | MINOR | the `p423cond` doc paragraph still described the old acceptance | resolved |
| RR-009 | MINOR | the new index-stability doc was mis-indented and overstated stability | resolved (reindented; the claim matches the implemented literal/local rule; RR-002 tracks the local case) |
| RR-010 | MINOR | the roadmap header undercounted the Executing severities; the fix-batch commits recorded only `make check` | resolved (header corrected; `make verify` runs on the corrected batch) |

Rulings applied (2026-10-27): RR-002 literal-index self-copies only (AR-27);
RR-003 and P1 fixed within the same AR-26/take-capture mechanisms (AR-28); P2
filed as `DEF-545` because a single range slot is a structural design limit and
the immediate-clear or multi-range redesign has a broad emission blast radius.

Phase 2 is closed: every critical/major re-review finding is fixed, or filed as
`DEF-544`/`DEF-545` with the user's ruling; RR-007 and the earlier minors remain
report-only. The phase gate is green for Phase 3.

## Phase Records

Execution adds one section per phase with the revalidation results, review
findings, and the final verification evidence. No row is marked Done without a
current-HEAD reproduction or a documented stale proof.

## Phase 3 — Specification Cases and Red Confirmation (2026-10-27)

The batch's specification suite is `def_conditional_value_integrity_spec_test.rs`
(13 cases covering ST-12…ST-22, with the extra green controls). The suite runs
with `cargo test --test def_conditional_value_integrity_spec_test`; the memory
cases run under `RDC_ASAN=1` as well. At HEAD `11140b3` the observed results are:

| ST | DEF | HEAD result | Decision |
|---|---|---|---|
| ST-12 | 470 | one `E10020` at the later `h.name`, no C | green stale pin (already closed) |
| ST-13 | 471 | SIGFPE (exit 136), no `E11210` and no stderr line | red |
| ST-14 | 472 | the owning `T = String` instance is accepted; the `T = str` control exits 0 and is ASAN clean | red (control green) |
| ST-15 | 473 | the owning `T = String` instance is accepted; the view control exits 0 | red (control green) |
| ST-16 | 475 | the mixed place/literal read base exits 3, strict C builds, ASAN clean | **stale close (AR-16)**: no longer reproduces at HEAD |
| ST-17 | 476 | strict C fails: the literal arm calls `n1`/`n2` without the hidden array destination, and the comma arm has no effect | red |
| ST-18 | 477 | the committed nested-else source fails strict C at base: the row calls have no hidden destination (the earlier then-arm shape exited 8) | red |
| ST-19 | 478 | `g = true` selects `n1` only (exit 2); `g = false` calls nothing (exit 0) | **stale close (AR-16)**: no longer reproduces at HEAD |
| ST-20 | 479 | the original short-circuit call argument and the array fill exit 9: the trace follows declaration order (`21`/`31`, fill `2121`); the fixed trace is `12`/`13`/`12` | red |
| ST-21 | 480 | the literal and call-projected arms exit 3/4, ASAN/LSan clean | **stale close (AR-16)**: no longer reproduces at HEAD |
| ST-22 | 481 | the all-call read base exits 0, ASAN/LSan clean | **stale close (AR-16)**: no longer reproduces at HEAD |

The four stale rows keep their green spec pins as the current-HEAD evidence; the
roadmap close in `3.3.3` records the proof for each. The active fixes are
`DEF-471`, `472`, `473`, `476`, `477`, and `479`.


## Phase 3 — Conditional Materialization Close (2026-10-27)

- **Baseline tree:** `11140b3` (the Phase 2 review gate).
- **Fixes:** `DEF-471` (`ee9fea8`), `DEF-472`/`473` (`89c3760`), `DEF-476`/`477`
  (`6c74563`), `DEF-479` (`3213dfb`).
- **Stale closes (AR-16):** `DEF-475`, `DEF-478`, `DEF-480`, and `DEF-481` no
  longer reproduce at HEAD; their ST-16/19/21/22 cases are the green pins.
  `DEF-470` keeps its earlier stale close.
- **Specification suite:** 18 cases (after the re-review fixes), 18/18 in the
  plain and `RDC_ASAN` runs; every memory case uses `run_memory`, and the ST-13
  fault case stays a strict fault probe.
- **Implementation pins:** 12/12 in `def_conditional_value_integrity_impl_test.rs`.
- **Retargets:** the Phase 0 inventory expected the conditional-projection/
  residue and fresh-root pins to change; the focused suites stayed green with no
  emission pin changes outside the fixed paths. The spec-source adjustments are
  the filed nested-conditional shape (ST-18), the restored original ST-20
  short-circuit call argument, and the array-fill twin.
- **Verification:** after the environment-only XAUTHORITY refresh, the full
  `make verify` reports **all legs passed**, including the FS-UAE leg;
  `make check` and the focused ASAN runs are clean.
- **Roadmap:** the ten active Batch C rows move to Done; the re-review siblings
  are `DEF-551`…`DEF-555`; the progress is 419/544 with twelve executing rows
  left.

## Phase 3 Review — 2026-10-27

Three lenses (correctness/semantics, memory safety, emission integrity) reviewed
the phase diff `11140b3..a893a7e`. The fixed rows' own cases are green and the
corpus emissions are explained, but the review found two new regressions, a
test-coverage weakening of the filed `DEF-479` shape, and incomplete
destination-aware paths. The phase gate is blocked pending the rulings below.

| Finding | Severity | Finding | Status |
|---|---|---|---|
| RV-001 | CRITICAL (new) | a duplicable struct conditional that needs stores is routed through the expression-form owned move; a **place arm** is copied into the temporary and then `cl_zero`d, so the source is silently zeroed. The concrete twin copies. The ST-17 case uses literal arms only. | resolved (`22722af`): only an owning arm is emptied; `placearm475b` and its implementation pin are red at the prior head |
| RV-002 | CRITICAL (new/test) | the source-order predicate reaches the store/guard/return/initializer positions but not call arguments or array fills; the **original ST-20** short-circuit call argument still exits 9 at HEAD, and the committed ST-20 was retargeted to a conditional store. The live `spec/04` §3 order rule is not covered; the `DEF-479` close is too broad. | resolved (`0f7dd70`): the lowering materializes order-mismatched literals in call/receiver positions and the fill evaluates once into a local; the original ST-20 and the fill are green, and `DEF-479` keeps its close |
| RV-003 | CRITICAL (new) | the destination map and the duplicability duty are function-wide, not flow-sensitive: `write_place` did not prune the map and the duty deduped by `(binding, paths)`. A view-destination duty could suppress a later owning move (`two_store_min`/`two_store_generic` accepted, ASAN SIGSEGV; base reports one `E10020`), and a stale destination could false-reject a re-initialized source (`stale_dest`). | resolved (`22722af`): the map and strict set are pruned on writes, a destination-less move marks the path strict, and the duty is recorded per use; the four destination-generation cases are red at the prior head |
| RV-004 | MAJOR (phase-relevant) | the destination judgment is wired only through `BindingEffect` (let/const/whole assignment). A concrete lend returned through a maybe-owning conditional is accepted at `T = String` and double-frees; literal-element, conditional take-argument, and read paths false-reject valid view instances. The return shape is pre-existing (base also accepts and double-frees). | filed as `DEF-546` (return double-free, critical) and `DEF-547` (element/take/read view false rejects, major); the 472/473 close is scoped to the let/whole-assignment forms |
| RV-005 | MAJOR (pre-existing) | at a view destination the store copies but records no view loan/escape duty; a source mutation, returned view, or caller-slot store is accepted while the concrete twin reports `E10031`/`E10035`/`E10033`; ASAN shows heap-use-after-free. | filed as `DEF-550` (critical, view-destination loan/escape) |
| RV-006 | MINOR | two concrete lend arms in one conditional report two `E10034` at one site in the generic body; the concrete twin dedupes by span. | report-only |
| RV-007 | MINOR | `record_requires_duplicable` and `maybe_move_destinations` docs still describe only parameter-rooted duties and overstate the conservative map. | resolved (`0f7dd70`): the docs describe the destination-aware duty and the pruning/strict set |
| RV-008 | MINOR (standards) | the new spec/impl section comments reference `DEF-…` identifiers, against the repository's no-plan-identifiers rule. | resolved (`0f7dd70`): the section comments are plain labels |
| RV-009 | MINOR | the red table records the pre-retarget ST-18 observation; the committed ST-18 fails base with a strict-C error, and the filed then-arm fixed-array shape is not pinned. | resolved (`0f7dd70`): the red table and the ST-20 source record are corrected; the restored original ST-20 and the fill are pinned |
| EIA-1 | MINOR | `function.rs:2313-2336` comments still say a nested conditional keeps the ternary fallback after the recursion fix. | resolved (`0f7dd70`): the comments describe the per-arm recursion |
| EIA-2..4 | MAJOR (pre-existing) | runtime-index `StorePointer` bypasses the struct-store path (accepted invalid C); value-position literal order is the RV-002 class; a literal zero divisor is emitted and fails strict C. | `DEF-548` (runtime StorePointer, major); the value-position order is resolved with RV-002; `DEF-549` (literal zero divisor, major) |

Rulings applied (2026-10-27): fix RV-001 and RV-003 now (both are new
regressions from this phase); fix RV-002 and restore the original ST-20, keeping
`DEF-479` closed once the value-position path is green; file RV-004's
return/element/argument/read paths as `DEF-546`/`DEF-547`, RV-005's view-loan
escape as `DEF-550`, and the pre-existing emission gaps as `DEF-548`/`DEF-549`;
apply the minor documentation and test-identifier fixes in the same batch.
RV-001/RV-003 are fixed at `22722af`; RV-002 and the minors are fixed at
`0f7dd70`.


## Phase 3 Re-review — 2026-10-27

A focused re-review over the review-fix diff (`034ce42..5a561e2` plus the
re-review corrections) confirmed RV-001/RV-002/RV-003 and found two new
regressions in those fixes. The re-review corrections are part of the phase
close.

| Finding | Severity | Finding | Status |
|---|---|---|---|
| RR-01 | CRITICAL (new) | the destination/strict prune was a function-wide strong update while `moved_if_owning` merges by union; a re-initialization on one branch erased the other path's destination, so a later use of a moved source was accepted (ASAN SEGV; the base reports `E10020`). | fixed: the destinations and strict set moved into `PlaceState` and join by union; `branchdest` is red at `5a561e2` and green |
| RR-02 | MAJOR (new) | a fill that covers no slot (`prefix == extent`) emitted a set-but-unused `__cl_fill` local, so the strict C build failed. | fixed: the full-prefix fill keeps its single evaluation and emits a `sizeof` use marker; `fullprefixfill` is red at `5a561e2` and green |
| RR-03 | MINOR (new) | the duty update ORed `partial` and moved the span to the latest use, so a whole-then-projected use reported `E10025` at the projected site. | fixed: the span/partial update only when the destination judgment changes |
| RR-04 | MINOR | stale ledger counts (`13`/`11`) and the retarget sentence after the ST-20 restore. | fixed in this record |
| RR-05 | MINOR (scope) | the expression-form and nested-row fills still evaluate once per element/row; the statement fill is fixed. | filed as `DEF-551` |
| — | MAJOR (pre-existing) | a later order-mismatched argument is hoisted before an earlier argument's effects; the new hoist makes it deterministic. | filed as `DEF-552` |
| P1 | MAJOR (pre-existing) | a module-initializer store-needing literal leaks its owning fields (init local not registered). | filed as `DEF-553` |
| P2 | MAJOR (pre-existing) | an owned `String` widened into a view slot is not released. | filed as `DEF-554` |
| P3 | MAJOR (pre-existing) | a module-initializer nested owning call field evaluates before earlier source fields. | filed as `DEF-555` |

The re-review found no new memory-safety defect in the corrected paths; the new
critical RR-01 and major RR-02 are fixed and red/green pinned. The phase gate is
green for Phase 4, with `DEF-551`…`DEF-555` filed as backlog rows.
