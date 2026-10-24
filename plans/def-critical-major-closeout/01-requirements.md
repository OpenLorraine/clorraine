# Requirements: Critical/Major DEF Closeout

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Close every open critical and major defect in `plans/00-roadmap.md` so the
repository can resume feature work on a clean safety and correctness baseline.
HEAD has 136 open rows: 35 critical/major (1 critical + 34 major) and 101 minor.
The program covers 34 active rows; `DEF-470` is revalidated and closed as stale,
so the plan tracks 35 DEF rows. The 101 minor rows are outside this plan
(AR-1, AR-2, AR-18).

## Functional Requirements

### Must Have

- [ ] R1 — Batch A: `DEF-543` no longer accepts a same-frame conditional write
  that launders a local view into an out-slot.
- [ ] R2 — Batch B: the eleven raw/conditional store and alias defects
  (`DEF-454`, `455`, `456`, `457`, `458`, `460`, `461`, `462`, `464`, `465`,
  `466`) no longer produce a use-after-free, double-free, lost store, or
  silently wrong value.
- [ ] R3 — Batch C: the conditional materialization/read-base rows
  (`DEF-470` revalidation, `471`, `472`, `473`, `475`, `476`, `477`, `478`,
  `479`, `480`, `481`) emit valid C, evaluate in source order, and release or
  retain every owned root exactly once.
- [ ] R4 — Batch D: the generic/null/conditional parity rows (`DEF-411`,
  `412`, `413`, `415`, `505`, `506`, `507`, `513`, `522`) match their concrete
  twins: safe programs are accepted, unsafe programs are rejected with the
  concrete twin's count (one report per site, or one per nullable arm for a
  same-arm nullable conditional), and accepted programs emit valid C.
- [ ] R5 — Batch E: `DEF-167` and `DEF-171`'s in-scope forms no longer accept a
  program whose C build fails; `DEF-416` reports the cross-module same-simple-name type
  collision with one `E11101` at the second declaration instead of collapsing
  the two types into one table entry. `DEF-171` covers the function-body,
  local-`const`, and module-level `let` forms; a module `const` keeps the staged
  `E10224`. R-6 resolved the `spec/04` §16 / `spec/13` §1 conflict in favor of
  `spec/04`: a module-level `let embed` reports `E10213`, and the `spec/13`
  sentence is recorded as a frozen inconsistency.
- [ ] R6 — Every active DEF has a specification regression derived from its
  concrete twin or frozen spec, written before the fix and verified red.
  `DEF-470` is the stale green pin (ST-12). A Phase 0 revalidation that closes
  `DEF-506` or `DEF-507` as stale under AR-16 gets a green HEAD pin instead of a
  red one; the `05-findings.md` proof records why. These green pins are the only
  exemptions from the red rule.
- [ ] R7 — Each fix has an implementation/ASAN/strict-build control where its
  class needs one; accepted outputs that change are re-emitted deterministically.
- [ ] R8 — Each roadmap row is marked Done through the roadmap skill with its
  commit and evidence at phase close; no row is closed without a reproducing
  test or a current-HEAD proof. The phase record is added to `05-findings.md`.
- [ ] R9 — `make check` and `make verify` pass at the end of every phase and at
  the end of the program.

### Should Have

- [ ] R10 — Fixes reuse the existing ownership, instance-audit, and codegen
  helpers; no generalized summary subsystem (AR-12).
- [ ] R11 — Each batch stays reviewable as its own commit series, with the
  phase and task markers kept current.

### Won't Have (Out of Scope)

- The 101 minor roadmap defects (AR-1).
- New language features, new diagnostics without an existing spec owner, or
  optional refactors. The `DEF-416` collision uses the existing `E11101` code.
- A general callee-summary or provenance-lattice framework.
- Constant-context data inclusion (lowering a module-level `let embed`); under
  R-6 the form is rejected with `E10213` instead. The `spec/13` §1 sentence
  stays a recorded frozen inconsistency.
- Any spec or golden change without explicit approval (AR-11).
- A rewrite of the conditional emission or ownership stages.

## Technical Requirements

### Performance

No new whole-program pass. The new correctness work must stay inside the
existing function/instance walks; a direct callee-body scan is bounded by body
size and does not run over the corpus twice.

### Compatibility

- Rust workspace stays std-only; no new crates or build scripts.
- C99 output stays valid for `host`, `hosted-amiga`, and `freestanding`.
- The frozen spec chapters and golden data stay the oracle; see AR-11. The
  `DEF-167` parse-only mode preserves the stage parse, and `DEF-416` is a
  conservative collision report, not a spec amendment.
- Accepted emitted C must remain byte-identical unless a defect's fix explicitly
  changes the accepted output, in which case a determinism pin is updated.
  `DEF-416` rejects the collision, so no C naming change is planned.

### Security

- No accepted program may retain a view of storage that dies before the read.
- ASAN/LSan probes cover every memory-class row; no leak, double-free, or UAF is
  accepted as “pre-existing” inside the active set.
- Invalid C is treated as a correctness failure, not a cosmetic one.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|---|---|---|---|---|
| Defect set | all 136 / critical+major / residuals | 35 critical/major at HEAD; 34 active after DEF-470 | Blocks feature work; minors can wait | AR-1, AR-2, AR-18 |
| Plan shape | umbrella / per-batch / per-DEF | umbrella, five batches across seven phases | Shared root causes and one verification policy | AR-3 |
| Batch order | risk-first / numeric / strict-build first | A–E risk-first | Memory safety before precision and parser gaps | AR-4, AR-6 |
| Fix policy | precise / pause / conservative | concrete twin + frozen spec | Preserves safety while recovering precision | AR-5 |
| Precision mechanism | summary subsystem / direct scan / conservative | direct existing-pattern scan | Avoids new architecture; see complexity stop | AR-12 |
| DEF-167 integration | reparse / amend stage test / parse-only mode | parse-only/stage-select mode | Keeps the frozen parse and the E10300 rejection | AR-14, AR-19 |
| DEF-416 behavior | qualified identity / collision report / defer | collision report (E11101) | Smallest path; no type-identity layer | AR-20 |
| Roadmap evidence | restore / move to ledger / leave | restore and append the plan link | Keeps the cross-session severity/evidence | AR-17, AR-21 |
| Spec amendments | free / never / approval | approval before application | Frozen spec remains authoritative | AR-11 |

> **Traceability:** Every scope decision references the confirmed register. See
> `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] All 34 active DEF rows are Done, and `DEF-470` is closed as stale with
   current-HEAD evidence; the plan tracks 35 DEF rows including the stale close.
2. [ ] Each active DEF has a failing-before/passing-after specification test.
   ST-12 is the green stale pin for `DEF-470`; a `DEF-506`/`DEF-507` stale close
   under AR-16 is a green HEAD pin with its `05-findings.md` proof. No other row
   is exempt from the red rule.
3. [ ] Memory rows are ASAN/LSan clean on their accepted controls and reject the
   unsafe reproduction.
4. [ ] Emission rows build under the strict C leg and preserve corpus output.
5. [ ] No open critical or major DEF remains in `plans/00-roadmap.md`; the
   roadmap rows are synced through the roadmap skill.
6. [ ] `make check` and `make verify` pass; review findings are closed or
   explicitly recorded.
7. [ ] No new warnings, dead code, or spec change without approval; `DEF-416` is
   a documented conservative collision limit, not a spec amendment.
