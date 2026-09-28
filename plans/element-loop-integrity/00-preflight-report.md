# Preflight Report: Element Loop Integrity

> **Status**: ✅ PREFLIGHT PASSED — all 6 findings resolved (0 critical, 1 major, 3 minor, 2 observations)
> **Iteration**: 1 (first scan; all accepted fixes applied and verified)
> **Artifact**: Implementation plan at `plans/element-loop-integrity/` (8 documents)
> **Codebase Grounded**: 14 source files + 3 spec chapters + 5 test/harness files examined; 62 references mapped, 60 verified, 2 approximate
> **Last Updated**: 2026-09-28 09:50

SAME-SESSION REVIEW: This artifact was created in the current session. Same-agent bias risk
is elevated. Consider running preflight in a new session for maximum review independence.
Counter-measures applied: every behavioral claim was re-verified by rebuilding `clc` in the
pinned Docker SDK and running the emitted C; one independent challenger reviewed the MAJOR
finding batch.

## Audit Scope

| Item | Value |
| ---- | ----- |
| Audit target | `plans/element-loop-integrity/00-ambiguity-register.md`, `00-index.md`, `01-requirements.md`, `02-current-state.md`, `03-01-element-read-ownership.md`, `03-02-nested-loop-step.md`, `07-testing-strategy.md`, `99-execution-plan.md` |
| Context documents (read, not audited) | `compiler/src/own/lower.rs`, `own/call.rs`, `own/loops.rs`, `own/place.rs`, `ir.rs`, `ir/control.rs`, `codegen/control.rs`, `codegen/function.rs`, `codegen/cleanup.rs`, `codegen/calls.rs`, `codegen/expr.rs`, `codegen/guards.rs`, `runtime/shim_host.c`, `compiler/tests/codegen_ownership_{spec,impl}_test.rs`, `spec/03-ownership.md`, `spec/05-statements.md`, `plans/00-roadmap.md`, `AGENTS.md` |
| Modification set | Not yet authorized — plan documents only, and only if fixes are requested |
| Scope mode | strict |

## Codebase Context Summary

**Tech Stack:** Rust workspace (std-only), emitting C99; pinned toolchain in `docker/`.
**Architecture:** staged compiler (`lex`→`parse`→`resolve`→`sema`→`own` (ownership/lowering to IR)→`codegen`); `own/loops.rs` builds loop IR, `codegen/control.rs` prints structured C.
**Key files examined:** see context table above; harnesses build emitted C with `cc -std=c99 -O0 -g -Wall -Wextra -Werror` (no sanitizer).
**Key observations:**
- The two proposed designs match the real code: `materialize_owned_argument` at `own/lower.rs:659` skips only `ValueKind::Place` (`:664`); `BranchShape::Loop { kind, body, exit }` at `ir/control.rs:26-34`; `for_step_block` at `codegen/control.rs:407`.
- `closure_from` stays used elsewhere (`codegen/control.rs:137`, `codegen/cleanup.rs:420`), so deleting `for_step_block` leaves no dead code.
- The emitter's runtime-index guard is emitted by `emit_return` (`codegen/cleanup.rs:264`) on the return path and by `emit_assign` (`codegen/function.rs:577`) on the assignment path; `E11211` text comes from `cl_trap` (`runtime/shim_host.c:39`) and `E10026` from `own/place.rs:224`.
- Verified pre-fix behavior by running the emitted C: ST-1–ST-8 abort with glibc `double free detected in tcache 2` (exit 134); ST-14–ST-18 hang; ST-9/ST-10 reject `E10026`; ST-11 exits 3 (ASan-clean); ST-12 exits 2; ST-13 traps `E11211` (exit 134); ST-19/ST-20 exit correctly.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
| --- | --- | --- | --- |
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 0 | — |
| 3 | Logical Contradictions | 1 | 🟠 |
| 4 | Completeness Gaps | 1 | 🟡 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 1 | 🟠 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 2 | 🟡 |
| 13 | Codebase Alignment | 2 | 🔵 |

Compiler-and-language lens (grammar/IR/type-system evidence in the repo; other domain lenses
not applicable): ownership semantics against `spec/03` §2/§4, loop step and `continue` semantics
against `spec/05` §8/§9/§12, diagnostics unchanged against `spec/14` — no conformance defects
found beyond the findings below.

### Summary by Severity

| Severity | Count | Status |
| -------- | ----- | ------ |
| CRITICAL | 0 | — |
| MAJOR | 1 | resolved |
| MINOR | 3 | resolved |
| OBSERVATION | 2 | resolved |

---

### PF-001: Red-phase expectations contradict the verified pre-fix behavior 🟠 MAJOR

**Dimension:** 7 — Testability (with dimension 3 contradiction and dimension 13 evidence)
**Location:** `99-execution-plan.md` task 1.1.2 (line 60); `07-testing-strategy.md` note lines 62-63 and checklist lines 114-115
**Codebase Evidence:** Rebuilt `clc` in the pinned Docker SDK and ran every case's program under the harness flags (`cc -std=c99 -O0 -g -Wall -Wextra -Werror`): ST-1–ST-8 abort (`free(): double free detected in tcache 2`, exit 134), ST-14–ST-18 hang; ST-9/ST-10 reject `E10026` (`own/place.rs:224`), ST-11 exits 3 (AddressSanitizer-clean), ST-12 exits 2, ST-13 traps `error E11211: index out of range` (`codegen/guards.rs:200`), ST-19/ST-20 exit correctly.
**The Problem:** Task 1.1.2 demands that "ST-1 – ST-11 and ST-13 fail … and ST-12 passes", and the 07 note says only "ST-12 and ST-20 pass before the fix". Both are wrong: ST-9, ST-10, ST-11, ST-13 also pass pre-fix, and ST-19 passes too. The correct split is: FAIL = ST-1–ST-8, ST-14–ST-18 (13 cases); PASS = ST-9–ST-13, ST-19, ST-20 (7 cases). The error contradicts the plan's own component spec, which calls these paths "unchanged" (`03-01` §Error Handling), and the red-phase gate is non-negotiable specification-first evidence.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Correct 99:60, 07:62-63, and 07:114-115 to the verified split; add a sentence that any deviation is an oracle/test defect to stop and record, not to "adjust" the case | Docs match verified reality; per-case falsification preserved; two prose edits, no test changes | Pins the gate to the current verified snapshot |
| B | Rewrite 1.1.2 and the note to "record the observed state of every case; the suite must fail overall" without a split | Less brittle wording | Weakens the gate: 13 red cases make "suite fails" trivially true, hiding a silently-passing runtime case |

**Recommendation:** Option A — the split is already verified, matches `03-01`'s "unchanged" declarations, and preserves per-case evidence. (The only other viable option, making every case red, is not possible without deleting a correct rejection or a working bounds guard.)
**Hardening:** Challenger: converged — independently recommended Option A and added two riders now folded into the options: also fix the 07 checklist lines 114-115, and add the deviation escape-valve sentence.
**Confidence:** High — would change only if the plain harness produced different per-case outcomes than verified.
**User Decision:** Resolved — User accepted recommendation (applied)

### PF-002: Spec coverage is narrower than the promised "every nested `for` form" 🟡 MINOR

**Dimension:** 4 — Completeness Gaps (test coverage)
**Location:** `07-testing-strategy.md` ST-14 – ST-20 (lines 54-60) and coverage goal line 13; `01-requirements.md` R4 (lines 42-45)
**Codebase Evidence:** The grounded repro `build/defs/nest2.clo` (three-clause `for` inside an element loop) hangs pre-fix — verified. `spec`-confirmed element/three-clause forms exist; the plan's own AR #3 accepted "cover every nested `for` form" and R4 promises "element loops and three-clause loops inside `while`, `for`, and element loops".
**The Problem:** The ST set covers element-in-element (ST-14), element-in-while (ST-15) and three-clause-in-while (ST-16), but has no case for a three-clause `for` inside an element loop (the grounded nest2 form), and no explicit `const`-parameter lend case although R1 promises "any non-take parameter". The coverage goal claims "every `for` form nested in every loop kind", which the cases do not realize.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add one ST case for a three-clause `for` inside an element loop (e.g. xs=[1,2], j=0..2, `s + x + j` → exit 15) and extend ST-6's program with a `const` callee; update the 99 task ranges | Closes the exact accepted-scope gap; cheap | Touches two docs |
| B | Add only the element-loop/three-clause case, and drop the "every form" wording to match | Cheaper | Leaves the `const` mode untested despite R1 |

**Recommendation:** Option A — both additions are grounded, cheap, and restore alignment between R1/R4/AR #3 and the oracle.
**User Decision:** Resolved — User accepted recommendation (applied)

### PF-003: Ambiguity Register row 12 is malformed and carries two statuses 🟡 MINOR

**Dimension:** 12 — Consistency
**Location:** `00-ambiguity-register.md` line 20 (row `12 | Naming | …`)
**Codebase Evidence:** Not code — document defect. The row contains 12 cells for an 6-column table, duplicates "New internal names…" and both `❌ Open` and `✅ Resolved`, while the header (line 3) claims "all 15 items resolved".
**The Problem:** The register is the gate audit trail and the plan’s user-decision authority for AR #12. A contradictory row undermines the "GATE PASSED" claim mechanically (a reader or tool cannot parse one decision/status) even though the substantive decision (`projects_from_place` + `step`) is clear and used consistently by the plan documents.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Repair the row to the table schema: options `projects_from_place` + `step` / other; decision `User accepted recommendation: …`; status `✅ Resolved` | Restores a parseable audit trail; no semantic change | None material |
| B | Leave it; rely on the header + resolution note | No edits | Keeps a broken, contradictory row in the authority document |

**Recommendation:** Option A.
**User Decision:** Resolved — User accepted recommendation (applied)

### PF-004: Close-out task says "file DEF-21", but DEF-21 is already filed 🟡 MINOR

**Dimension:** 12 — Consistency
**Location:** `99-execution-plan.md` task 3.1.3 (line 158); `01-requirements.md` acceptance criterion 6 (line 111)
**Codebase Evidence:** `plans/00-roadmap.md:40` already carries the `DEF-21` row filed during planning; AR #4 records the filing as the resolution of that scope question.
**The Problem:** Task 3.1.3 instructs the executor to file DEF-21 and acceptance criterion 6 says "`DEF-21` is filed". Following the task literally would duplicate an existing roadmap row or stall; the task should verify, not file.
**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Reword 3.1.3 to "confirm `DEF-21` is filed (already filed during planning)" | Matches reality; no duplicate work | None material |
| B | Remove the DEF-21 mention from 3.1.3 | Simpler | Loses the close-out verification link |

**Recommendation:** Option A.
**User Decision:** Resolved — User accepted recommendation (applied)

### PF-005: Guard-source citation points at the assignment path only 🔵 OBSERVATION

**Dimension:** 13 — Codebase Alignment
**Location:** `02-current-state.md` risk table (line 127); `03-01-element-read-ownership.md` §Integration Points (line 95)
**Codebase Evidence:** `emit_assign` (`codegen/function.rs:577`) emits guards for assignments, but the ST-2/ST-13 shape `return vlen(arr[i]);` is emitted by `emit_return`, which emits the guards at `codegen/cleanup.rs:264`. The substance ("runtime-index reads remain bounded") is verified true; only the citation is incomplete. Minor anchor drift: `own/loops.rs:268` names the shape decision line while the `BranchShape::Loop` literal is at `:269-273`.
**The Problem:** A reader checking the cited function for the return path finds no guard call; precision matters for a safety-critical claim.
**Recommendation:** Cite both `emit_return` (`cleanup.rs:264`) and `emit_assign` (`function.rs:577`) for guard emission; optionally tighten the loop line anchors.
**User Decision:** Resolved — User accepted recommendation (applied)

### PF-006: Register option names a "global base" case that cannot exist 🔵 OBSERVATION

**Dimension:** 13 — Codebase Alignment
**Location:** `00-ambiguity-register.md` AR #13 options cell
**Codebase Evidence:** Compiling `build/defs/read13.clo` reports `E11106: module-level 'let' must be duplicable or 'const'` and `E10038: cannot borrow global 'g'` — module-level owned arrays are rejected, so no global-base ST can exist. The plan documents correctly omit it.
**The Problem:** The accepted "full matrix" wording could be read as promising a global-base case; it can only mean every legal form.
**Recommendation:** Add a parenthetical to the AR #13 decision ("globals of owned arrays are rejected, so the matrix is every legal form") — or leave as harmless wording.
**User Decision:** Resolved — User accepted recommendation (applied)

---

## Fix Verification (Iteration 1 close)

All accepted resolutions were applied to the plan documents and verified directly (the edits are
prose and test-case descriptions; no behavior, ownership, dependency-order, security, or
compatibility contract changed, so a bounded verification is used per the convergence rules).

| Finding | Applied edit | Verification |
| ------- | ------------ | ------------ |
| PF-001 | `99` task 1.1.2 now expects ST-1–ST-8 to fail and ST-9–ST-13 to pass, with a stop-and-record deviation rule; `07` note lines 62-65 and checklist line 114-116 use the verified split | Grep shows no stale split remains; the split matches the reruns recorded in Codebase Evidence |
| PF-002 | Added ST-21 (three-clause `for` inside an element loop, expected exit 15, arithmetic re-checked: 6 + 9); extended ST-6's acceptance to a `const` callee in `01` criterion 1/2; updated `07` suite range and `99` tasks 2.1.1/2.1.2/2.2.2 and `03-02` reference | Grep confirms no `ST-1 – ST-20` or `ST-14 – ST-20` ranges remain in the plan docs |
| PF-003 | Register row 12 restored to the 6-column schema with one `✅ Resolved` decision | Column-count check: every register row now has exactly 6 fields |
| PF-004 | `99` task 3.1.3 now says "confirm `DEF-21` is filed (as recorded during planning)" | `plans/00-roadmap.md:40` already carries the row |
| PF-005 | `02` risk table and `03-01` integration point now cite `emit_return` (`cleanup.rs:264`) for the return path alongside `emit_assign` (`function.rs:577`) | Both citations resolve to the guard-emitting lines |
| PF-006 | AR #13 decision notes "every legal form; module-level owned arrays are rejected" | Matches the `E11106`/`E10038` behavior of `read13` |

## Iteration Log

| Iteration | Action | Result |
| --------- | ------ | ------ |
| 1 | Full 13-dimension scan with codebase grounding and one independent challenger on the MAJOR finding | 6 findings recorded; user accepted all recommendations |
| 1 (close) | Applied all accepted resolutions; direct verification of each edit and a consistency sweep of the affected cross-references | All 6 findings resolved; no new issues; pass verdict |
