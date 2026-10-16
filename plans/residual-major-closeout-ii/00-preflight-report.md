# Preflight Report: Residual Major Closeout II (Batch L)

> **Status**: ✅ PASSED WITH NOTES — 1 major resolved (auto-design ruling, applied); 5 minors
> (4 applied, 1 accepted note); 2 observations (1 applied, 1 noted)
> **Iteration**: 1 (first scan; no prior report)
> **Artifact**: Implementation plan at `plans/residual-major-closeout-ii/` — 15 documents
> **Baseline content hash** (15 documents at scan start):
> `b0dc5f9bf37e9fb6bca8ddeb7bd9a8fc65b38cbbb06a34af4555136c04e8e6d3`
> **Corrected plan hash** (after the applied corrections, before this report):
> `33c71d14284fc3b33e3f20df2cb61fa47cf5f6914db2f6a659f428588efbea23`
> **Codebase Grounded**: ~20 compiler source files examined; ~40 cited code references mapped
> (35 exact, 5 drifted); 23 roadmap rows audited; 2 batch ledgers; 6 spec chapters + the Ch 14
> registry; ≈70 probe invocations
> **Last Updated**: 2026-10-16

**Audit target**: the 15 documents under `plans/residual-major-closeout-ii/`.
**Context documents** (read to test the target; not audited, not advanced): `plans/00-roadmap.md`
(the 23 rows), `plans/residual-major-closeout/05-findings.md` (batch K), 
`plans/conditional-projection-residue-ii/05-findings.md` (batch J), `spec/04-expressions.md`,
`spec/02-types.md`, `spec/14-diagnostics.md`, `spec/17-codegen.md`, the compiler sources, and the
recorded `build/` probe tree.
**Modification set**: the target folder only — this report plus the corrections recorded below.
No source, test, roadmap, or other-plan edits; no commits.

## Auto-Design Provenance (per `_shared/auto-design.md`)

```text
mode: auto-design
root invocation ID: codeops-dispatch agent=preflight-auditor feature=residual-major-closeout-ii
parent workflow: preflight
policy version: 1
delegated categories: eligible technical resolutions (frozen-conformance fix selection;
  plan-document corrections; reviewer-lens alignment)
reserved categories: the complete reserved set (product behavior/scope, acceptance criteria,
  priorities, risk acceptance, the six parked rows' rulings)
permission state: target-folder edits authorized by the dispatch; no commits; no source/test edits
```

**Reserved decisions**: none outstanding. The six ruling-parked rows (`DEF-167`, `DEF-169`,
`DEF-171`, `DEF-190`, `DEF-191`, `DEF-263`) stay user-owned and out of this batch (AR-2), and the
`DEF-347` (AR-7) / `DEF-357` (AR-13) phase stop-gates remain as recorded.

**Same-agent bias**: the plan was authored in a sibling planning session, not this one; this scan
ran in a fresh subagent context. Residual same-model-family bias is possible. Every finding below
is grounded in a probe result or a cited frozen-text line, and the two most consequential
analyses (AR-6 and AR-7) were re-derived from independent evidence (the scalar member twin's
rejection; `types::view_member_ty`'s typing), not from the plan's own reasoning.

## Codebase Context Summary

- **Stack**: Rust workspace (`clc` compiler, `clorraine` driver), C99 emitted translation unit +
  hand-written shims, embedded Clorraine stdlib; **std-only**; pinned Amiga SDK container.
- **Architecture**: lex → parse → load → resolve → sema → own → ir → codegen. The
  monomorphization walk (`own/generics.rs`) keys the instances the emitter (`codegen/expr.rs`)
  spells; the own stage judges places, loans, and drops; `consts` folds intrinsics; the
  assignment-place rule is `sema/ops.rs::check_assign` / `is_place` (`E10221`).
- **Key observations the audit relied on**:
  - `types::view_member_ty` (`types/mod.rs:759`) types `str.ptr` as `raw u8`; the C `ClStr`
    member is `const u8 *` (a qualifier artifact, `spec/17-codegen.md:146`).
  - `sema/expr.rs::member_type` (:692) dispatches members; the unhandled-base fallthrough is at
    :748; the enum/raw arms are :711-730. The walk's `resolved_ir_value_ty` already carries the
    batch-K F-2 conditional arm (:1093); the remaining decline is the `Place` arm's `Local`-only
    base (:1109).
  - The container build is clean at the head (`make -C docker exec CMD="cargo build --workspace"`,
  Finished, up to date); the local `target/debug/clc` postdates every source file. Probes ran on
  host GCC 15.2 with the recorded strict flags (`-std=c99 -O0 -g -Wall -Wextra -Werror`); every
  reproduced diagnostic matched the recorded container evidence, so the host/container
  distinction is immaterial for the sampled builds.
- **Reference verification**: ~40 document→code references mapped — 35 exact; 5 drifted (2
  wrong-function anchors corrected, 1 wrong-arm anchor corrected, 2 cosmetic ±8); all cited paths
  exist.

## Summary by Dimension

| # | Dimension | Findings | Highest severity |
|---|-----------|----------|------------------|
| 1 | Ambiguities | 1 (PF-004, ST-11/ST-12 wording) | 🟡 |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 1 (PF-001) | 🟠 |
| 4 | Completeness Gaps | 0 | — |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 2 (PF-002, PF-004) | 🟡 |
| 8 | Security Blind Spots | 0 (input validation unchanged; N/A as recorded) | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 1 (PF-001 — the parked route would have added surface) | 🟠 |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 3 (PF-005, PF-006, plus PF-002) | 🟡 |
| 13 | Codebase Alignment | 4 (PF-003, PF-007, PF-008, plus PF-001 evidence) | 🟠 |

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 1 | Resolved — auto-design ruling applied (PF-001) |
| MINOR | 5 | 4 applied (PF-002, PF-003, PF-005, PF-006); 1 accepted note (PF-004) |
| OBSERVATION | 2 | 1 applied (PF-007); 1 noted (PF-008) |

## Row-Status Verification (all 23 rows vs the plan's claims)

| Phase | Rows | Probed at the head (this audit) | Claim status |
|-------|------|---------------------------------|--------------|
| P1 | 335/336/337 | `k1d10` run 139; crafted place arms run 139; `k1d15` run 139; `k1e6` run 139; controls `k1d9` 2, `k1m1` 0 | ❌ red confirmed |
| P2 | 338/340/342 | `d03` strict fail (`cl_pick__Box`); `b8` strict fail (`cl_keep__String`); `p2b` strict fail (`cl_id__i32`); controls `p2d2` 3, `p2f` 7 | ❌ red confirmed |
| P3 | 339/345/353 (+341) | `s2` LSan leak; `e04/e07/e09/e11` strict fail; `p5dup` run 249; `b10` **E10401**, crafted concrete twin **E10401**; controls `dn4` 7, `dn5` 24, `d05c` 2, `d06` 1 | ❌ red / ✅ control confirmed |
| P4 | 315/329 | `hp_alias_copy` ASan heap-use-after-free; `rawreadown` ASan double-free | ❌ red confirmed |
| P5 | 322/323 | `def322` run 251 (no store); `read322` run 0 (read routes); `scalar322` E10221; crafted plain 323 runs the skipped call (calls=3); crafted take 323 strict fails (`too few arguments`) | ❌ red confirmed |
| P6 | 344/351 (+350) | `dn1` strict fail; `dn2` one `E10012`; `dn3` two; `reverse1/gencallee/genmixed/bs01` strict fail; `genlit` E10200; `h8/h9/fr17c/fr17f` E10200; `fr17b/fr17e` build+run 0; `fr17d/fr17h` compile; `fr17l` E10500; `fr17j` strict-fail residual (other rows' scope) | ❌ red / ✅ control confirmed |
| P7 | 346/347/348 | `def346a` strict fail (`member 'ptr'`); `def346b` strict fail (`'ClStr' no 'foo'`); `def347` strict fail (`discarded-qualifiers`); `ctl347` run 0; `def348` strict fail (`member 'A'`); `ptrconv` cstring destination accepted+cast | ❌ red / ✅ control confirmed |
| P8 | 356/357/359 | `alias_fn` strict fail (alias symbol); self-qualified family `fnself`/`fnself3` run 0; crafted `q.holder.g(5)` **E10219**; `plain_fn` 0; `extent` 253, `extent_view` 248; `structpad`/`view` 0; `sigmain2` strict fail; `sigmain` 0 | ❌ red (partial) confirmed |

---

## Findings

### PF-001: `DEF-322`'s chosen per-arm store route is not grounded in the frozen place rule [🟠 MAJOR]

**Dimension:** 3 (Logical Contradictions) / 10 (Scope Creep) / 13 (Codebase Alignment)
**Location:** `00-ambiguity-register.md` AR-6 (row and note); `01-requirements.md` R5 and Scope
Decisions; `03-05-store-emission-conditional-literals.md` §1; `07-testing-strategy.md` ST-13;
`99-execution-plan.md` phase 5; `02-current-state.md` P5 row.
**Codebase Evidence:**
- `spec/04-expressions.md:88` (§4): "The target must be a place: a local, a parameter, a struct
  field, an array element, or a dereferenced raw pointer, otherwise E10221"; `:387` (EX-14). No
  conditional appears in the list.
- `spec/14-diagnostics.md:107`: `E10221` = "Assignment to something that is not a place".
- Probe `scalar322.clo` (crafted): `(c ? b1 : b2).x = 7;` → **E10221** at the head — the same
  conditional base under a scalar member is already rejected.
- Probe `read322.clo`: `let v = (c ? b1 : b2).ys[i];` → builds, runs 0 — reads need no place, so
  the read's acceptance does not sanction a store.
- Probe `def322` (`build/memory-audit/j4/def322_probe.clo`): run 251 — the store is silently
  dropped with the RHS.

**The Problem:** AR-6 chose the per-arm route as primary ("the read spelling already routes…
keeps the acceptance"), with the frozen `E10221` rejection as the fallback "if the route cannot
be expressed without new surface". The route *is* expressible, but it would accept
`(c ? b1 : b2).ys[i] = 7` while the same base stays rejected for a scalar member — an acceptance
the frozen place list does not define. Executed as planned, the phase would add language surface,
which AR-12 forbids; the fallback's trigger condition (expressibility) is mis-specified.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Report the frozen `E10221` at the assignment-place judgment (the scalar twin's rule and site); park the route | Frozen-conformant; consistent with the scalar twin; no new surface; smaller fix | Rejects a program the head accepted (silently broken anyway) |
| B | Keep the per-arm route as primary | Keeps the acceptance | Adds an acceptance rule the frozen list does not define; inconsistent with the scalar twin; needs a spec ruling (AR-12 park) |

**Recommendation:** Option A — the frozen place rule's closed list and the scalar twin's
verification reject the route; the rejection is the conformance fix.

**User Decision:** Resolved — auto-design delegated (preflight-auditor dispatch under the
standing batch mandate): adopt Option A; the route is parked with this evidence. Applied (see
Corrections C1).

```text
Authority: AI — delegated by --auto-design
Eligibility: conformance-fix selection inside the frozen rules (implementation mechanism; no
  reserved consequence — no product scope, no acceptance-criteria change)
Objective: close DEF-322 so the store is never silently dropped, conforming to the frozen
  place rule (the batch-L mandate)
Decision: report the frozen E10221 at the assignment-place judgment (the scalar member twin's
  rule and site); park the per-arm route
Evidence: spec/04-expressions.md:88 §4 and :387 EX-14 (closed place list); spec/14-diagnostics.md:107;
  probe scalar322 (E10221); probe read322 (run 0 — reads need no place); probe def322 (run 251,
  silent drop)
Rejected alternatives: the per-arm route — it accepts what the frozen place list does not
  define and what the scalar member twin rejects, and would need a spec ruling (AR-12)
Strongest counterargument: the head accepts the spelling, so a rejection can look like a
  regression — but the acceptance is a silent wrong-code bug, and the same base is already
  rejected under a scalar member; the row's recorded fix direction permits the rejection
Confidence: High — two independent probes plus the closed spec list; would change if a
  frozen-text basis for conditionals as places appears
Hardening: required independent challenger unavailable in this dispatch; the two independent
  evidence layers stand in; no divergence observed
Policy version: 1
Root invocation ID: codeops-dispatch agent=preflight-auditor feature=residual-major-closeout-ii
Reopen triggers: a spec/decision-ledger ruling adds a conditional-of-places form; or the
  phase's semantics review grounds the route in the frozen text
```

---

### PF-002: `e01_scalar_member_min` is named a control although it is red at the head [🟡 MINOR]

**Dimension:** 7 (Testability) / 12 (Consistency)
**Location:** `03-03-element-instance-substitution.md` §2 and Integration Points;
`07-testing-strategy.md` ST-8.
**Codebase Evidence:** probe `e01` (`build/emission-audit/k2/probes/e01_scalar_member_min.clo`,
`func f(): i32 … return f().y;`) → accepted, strict build fails (`request for member 'y' …`) — it
is the pre-existing "member on a scalar" family, not a generic-substitution case. The true
controls are green: `d05c` run 2, `d06` run 1, `dn4` run 7, `dn5` run 24, all strict-build clean.
**The Problem:** AR-5/AR-11 require controls to be verified green first; a red "control" would
fail the phase's green-first gate or, worse, pin wrong behavior.
**Recommendation:** Option A — strike `e01` from the control lists and record it as a
pre-existing out-of-scope red; name the concrete twins as the controls.
**User Decision:** Resolved — auto-design delegated: Option A applied (Correction C3).

---

### PF-003: Stale/wrong line anchors in the phase design documents [🟡 MINOR]

**Dimension:** 13 (Codebase Alignment — phantom references)
**Location:** `03-02-generic-walk-resolver.md:11,:63` (`codegen/expr.rs:~728` → actually `:887`);
`03-07-member-typing-concrete-paths.md:21,:54` (`sema/expr.rs:598` → actually the dispatch at
`:692` with the fallthrough at `:748`; `:569-579` → actually `:711-730`); `02-current-state.md`
`:27,:62,:119,:157` (same two anchors).
**Codebase Evidence:** `grep -n "fn resolved_call_type_args" compiler/src/codegen/expr.rs` → 887;
`resolved IR value ty` sites; `member_type` at `sema/expr.rs:692`, `_ => Ty::unknown()` at :748,
enum/raw arms at :711-730.
**The Problem:** The plan's own note says the executor re-resolves line numbers, but these three
anchors point at unrelated functions (`:~728` is a binary-expression renderer; `:598` is inside
`check_qualified`; `:569-579` is `check_int`/`check_float`), which can mislead the executor.
**Recommendation:** Option A — correct the anchors in place.
**User Decision:** Resolved — auto-design delegated: Option A applied (Correction C2). The
cosmetic drift (`consts/mod.rs:495` → fn at :497; `own/place.rs:1112` → fn at :1120, doc block
at :1112) is left as-is and noted here.

---

### PF-004: Disjunctively worded spec-case oracles at the stop-gated rows [🟡 MINOR]

**Dimension:** 1 (Ambiguities) / 7 (Testability)
**Location:** `07-testing-strategy.md` ST-11 ("accepted with the loans intact **or** reports the
frozen conflict"), ST-12 ("moves … **or** reports the frozen move rule"); ST-13 and ST-19
originally ("**or** the frozen `E10221` (AR-6)", "**or** the frozen `E10200` (AR-7)").
**Codebase Evidence:** The head reds are unambiguous (probes: `hp_alias_copy` ASan
heap-use-after-free; `rawreadown` ASan double-free), so every case still fails before the fix;
the concern is the *green* oracle — a case that accepts either outcome is not an immutable
oracle.
**The Problem:** A stop-gated fallback is a phase contingency, not a test expectation. Left as
written, the test could pass on either outcome and cannot detect a fix that took the wrong path.
**Recommendation:** Option A — write one pinned expectation per spelling at the phase's red-ref
measurement (AR-5 already requires it); if a stop-gate fires, record an AR amendment and rewrite
the case before the review gate.
**User Decision:** Resolved — auto-design delegated: ST-13 and ST-19 tightened (Correction C4);
ST-11/ST-12 accepted note — the expectation is pinned by the phase's red-ref measurement, and the
report records this requirement for the phase-4 review.

---

### PF-005: P2/P6 lens sets omit the memory-safety auditor their sibling phases carried [🟡 MINOR]

**Dimension:** 12 (Consistency) / 13 (Codebase Alignment)
**Location:** `00-ambiguity-register.md` AR-10; `00-index.md` Key Decisions + Specialist Agents;
`05-findings.md` phases 2/6; `99-execution-plan.md` phases 2/6.
**Codebase Evidence:** `plans/residual-major-closeout/05-findings.md` — batch-K phase 2 ("Lenses:
correctness (K2R), semantics (K2S), emission-integrity …, memory-safety (K2M)") and phase 4
("correctness (K4R), semantics (K4S), emission-integrity (F1), and memory-safety (0 findings)")
both carried the read-only memory-safety auditor; the plan's P2 (walk resolver) and P6 (call
conversions/`ffi`) are those phases' direct siblings and make previously invalid-C programs run.
`AGENTS.md`'s dispatch matrix routes IR/codegen changes to correctness + memory-safety (+
emission-integrity).
**The Problem:** Newly running accepted programs are exactly the surface the ASan probe covers;
the plan's trigger clause ("any ownership/drop diff") does not fire for P2/P6, and no rationale
was recorded. P7/P8 follow their own sibling phases (K3/K6), which did not carry the auditor.
**Recommendation:** Option A — add the memory-safety auditor to P2/P6 (read-only, cheap;
sanitizer runs over the resolved instances and the legal conversion instances); keep P7/P8 as the
K3/K6 sets.
**User Decision:** Resolved — auto-design delegated: Option A applied (Correction C5).

---

### PF-006: AR-9 says P3 → `checker_conformance`, but the `DEF-341` control is scheduled in the residue suite [🟡 MINOR]

**Dimension:** 12 (Consistency)
**Location:** `00-ambiguity-register.md` AR-9 vs `07-testing-strategy.md` Test Categories table
and `99-execution-plan.md` 3.1.1 / verification commands.
**Codebase Evidence:** The testing table and the phase task place ST-10 (the `DEF-341` capture
pair) in `conditional_projection_residue_{spec,impl}_test.rs`; AR-9 lists P3 under
`checker_conformance`.
**The Problem:** Two placement rules for one case; the executor must guess.
**Recommendation:** Option A — record the exception in AR-9 (the capture-pair control may ride
the P4/P5 suite).
**User Decision:** Resolved — auto-design delegated: Option A applied (Correction C6).

---

### PF-007: P3/P4 execution task file lists are narrower than their design documents [🔵 OBSERVATION]

**Dimension:** 13 (Codebase Alignment)
**Location:** `99-execution-plan.md` 3.2.1 (lacks `sema/call.rs`, `own/call_check.rs` — the duty
record sites named by `03-03`'s integration points) and 4.2.1 (lacks `own/statements.rs:365`,
`own/place.rs` — the `!is_view_like` guard and `move_element_loans` named by `03-04` and
`02-current-state`).
**The Problem:** The AR-15 deviation protocol covers the extra files, but naming them avoids a
deviation record.
**Recommendation:** Option A — add the missing files to the two task lines.
**User Decision:** Resolved — auto-design delegated: Option A applied (Correction C7).

---

### PF-008: AR-7's fallback premise is doubtful — `.ptr` already types `raw u8` [🔵 OBSERVATION]

**Dimension:** 13 (Codebase Alignment)
**Location:** `00-ambiguity-register.md` AR-7; `03-07-member-typing-concrete-paths.md` §2;
`07-testing-strategy.md` ST-19.
**Codebase Evidence:** `types/mod.rs:759` `view_member_ty` types `str.ptr` as `raw u8`; the C
`const u8 *` is the `ClStr` ABI shape (`spec/17-codegen.md:146`). Probe `ptrconv`: a `cstring`
destination accepts `s.ptr` and renders the cast; a `raw u8` destination is type-identical and
fails only on the C qualifier. There is no language-level conversion for a semantics review to
rule illegal, so the AR-7 fallback (`E10200`) would report a conversion that does not exist, and
it would turn an accepted spelling into a rejection.
**The Problem:** A stop-gate whose premise cannot fire coherently; if taken, the phase would
produce a mis-coded diagnostic.
**Recommendation:** Option A — keep the destination-cast fix as the only oracle (ST-19 tightened
in C4); leave AR-7 in the register as a formal stop-gate but record that the E10200 fallback is
not expected to apply.
**User Decision:** Noted — auto-design: no further plan edit (ST-19 tightening applied in C4);
the phase-7 review can close AR-7 on the cast evidence.

---

## Verification Evidence (probe summary; sources under `build/`, scratch under `/tmp/opencode/preflight-l/`)

| Area | Probe (source) | Result at the head |
|------|----------------|--------------------|
| P1 | `k1d10`, crafted `def335f`/`def335g`, `k1d15`, `k1e6` | run 139 each (NULL deref); controls `k1d9` 2, `k1m1` 0 |
| P2 | `d03`, `b8`, `p2b` strict fails; `p2d2` 3, `p2f` 7 | red / green controls |
| P3 | `s2` + LSan leak; `e04/e07/e09/e11` strict fails; `p5dup` 249; `b10` + crafted concrete twin E10401 | red / control confirmed |
| P4 | `hp_alias_copy` ASan UAF; `rawreadown` ASan double-free | red (sanitizer) |
| P5 | `def322` 251; crafted read twin 0; crafted scalar twin E10221; crafted 323 plain calls=3, take strict fail | red / controls |
| P6 | `dn1` strict fail; `dn2` 1×E10012; `dn3` 2×E10012; `reverse1/gencallee/genmixed/bs01` strict fail; `genlit` E10200; `h8/h9/fr17c/fr17f` E10200; `fr17b/fr17e` run 0; `fr17d/fr17h` compile; `fr17l` E10500; `fr17j` strict-fail residual | red / control table |
| P7 | `def346a/def346b/def347/def348` strict fails; `ctl347` 0; `ptrconv` cstring dest cast | red / control |
| P8 | `alias_fn` strict fail; crafted `q.holder.g(5)` E10219; `plain_fn`/`fnself`/`fnself3` 0; `extent` 253, `extent_view` 248; `structpad`/`view` 0; `sigmain2` strict fail; `sigmain` 0 | red (partial) / controls |
| Tests strategy | red-before-green: every red ST case fails at the head (above); read-backs present for `s2` (value 26 + LSan), `p5dup` (249→0), `def322` (251); sanitizer gates named for P1/P3/P4/P5 — no silent-memory defect gated only by exit 0 | ✅ |
| Build | `make -C docker exec CMD="cargo build --workspace"` | clean, up to date |

## Plan Corrections Applied

| # | Correction | Files | Finding |
|---|------------|-------|---------|
| C1 | AR-6 flipped: the frozen `E10221` place rejection is the phase-5 fix; the per-arm route parked (frozen place list + scalar-twin evidence); R5, quick reference, key decisions, current state, 03-05 (§1, error table, tests), ST-13, 99 phase-5 tasks/deliverable/dispatch, 05 ledger note, risk row, unrelated AR-6 mentions re-worded | `00-ambiguity-register.md`, `00-index.md`, `01-requirements.md`, `02-current-state.md`, `03-05-…`, `03-06-…`, `07-testing-strategy.md`, `99-execution-plan.md`, `05-findings.md` | PF-001 |
| C2 | Anchors corrected: `codegen/expr.rs:~728` → `:887`; `sema/expr.rs:598` → `:748`; `:569-579` → `:711-730` | `03-02-…`, `03-07-…`, `02-current-state.md` | PF-003 |
| C3 | `e01` struck from the P3 control lists; the concrete twins named | `03-03-…`, `07-testing-strategy.md` | PF-002 |
| C4 | ST-13 rewritten to the single rejection oracle; ST-19's AR-7 alternative removed; store-emission coverage row re-worded | `07-testing-strategy.md` | PF-001, PF-004, PF-008 |
| C5 | Memory-safety auditor added to P2/P6 (register AR-10 row + note, index key decisions + specialist table, 05 phases 2/6, 99 phases 2/6) | `00-ambiguity-register.md`, `00-index.md`, `05-findings.md`, `99-execution-plan.md` | PF-005 |
| C6 | AR-9 records the `DEF-341` capture-pair placement exception | `00-ambiguity-register.md` | PF-006 |
| C7 | P3/P4 task file lists extended (`sema/call.rs`, `own/call_check.rs`; `own/statements.rs`, `own/place.rs`) | `99-execution-plan.md` | PF-007 |

## Could Not Verify

| Item | Reason |
|------|--------|
| The exact crafted probe sources removed at planning close (`def335f/g`, `def323a/b`, `ctl322`, `ctl323`, `ctl347`, `def346a/b`, `def347`, `def348`, `def356d`, `def342c`) | Removed per AR-3; equivalents were re-crafted from the row descriptions and reproduced every claimed symptom (E10221 scalar twin; run 139 place arms; strict fails; E10219; the 323 take/plain signatures) |
| The self-qualified `b.holder.g(5)` control and the wider `k18` matrix (`amb`, `pdiff`, `pimp`) | Probes not preserved; the `fnself`/`fnself3` current-module controls run 0 and the batch-K K6C-1 close-out records the four-probe confirmation |
| `DEF-344`'s explicit twin (`pair<i32>(1, x)`) | Not re-run; the recorded count controls (`dn2` 1, `dn3` 2) reproduced exactly |
| `make check` / `make verify` (full suites, Amiga legs) | Out of scope for this audit; the container workspace build and the focused probes ran instead |
| Pinned-container compiler for the probes (GCC 12.2) | Probes ran on host GCC 15.2 with identical strict flags; every reproduced diagnostic and run value matched the recorded container evidence on the shared probes |

## Verdict

`✅ PREFLIGHT PASSED WITH NOTES` — all critical/major findings resolved (1 major, applied under
an auto-design delegated ruling); the remaining minor findings are applied or accepted as notes;
the two observations are applied/noted. The plan's scope, sequencing, row accounting (23 = 21 red
+ 2 controls), frozen-conformance directions (with the AR-6 amendment), and test/verification
strategy are grounded in the codebase and the frozen oracle.

**Required plan corrections**: none outstanding — C1–C7 are applied and recorded above. The
corrected plan carries hash `33c71d14…`; any workflow consuming this plan should re-read it in
its corrected form.
