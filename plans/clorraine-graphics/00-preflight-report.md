# Preflight Report: Clorraine Amiga Graphics

> **Status**: ✅ RESOLVED — Iteration 2: all 10 findings resolved (3 major, 5 minor, 2 observation), 0 critical; no new findings
> **Iteration**: 2 (re-scan after fixes)
> **Artifact**: Implementation plan at `plans/clorraine-graphics/` (10 documents)
> **Content hash**: iteration 1 `dcf5f455…4216`; iteration 2 post-fix `40c1059b…0a51a` (untracked; excludes this report)
> **Codebase Grounded**: 14 source/spec/script files examined, 12 references verified, 1 unverifiable
> **Last Updated**: 2026-09-24 14:45

> ⚠️ **SAME-SESSION REVIEW:** This artifact was created in the current session.
> Same-agent bias risk is elevated. Consider running preflight in a new session
> for maximum review independence.

### Codebase Context Summary

**Tech Stack:** Rust workspace (compiler `clc` + `clorraine` driver), embedded
Clorraine stdlib under `stdlib/`, C99 runtime shims under `compiler/runtime/`,
frozen spec under `spec/`, Docker SDK toolchain, shell test scripts.
**Architecture:** stage pipeline (lex→parse→load→resolve→sema→own→ir→codegen);
stdlib modules registered in `compiler/src/stdlib.rs`; `ffi` symbols implemented
by a per-profile shim; corpus programs under `spec/tests/corpus/`.
**Key Files Examined:** `compiler/src/stdlib.rs`, `compiler/src/profile.rs`,
`compiler/src/driver.rs`, `compiler/runtime/shim_hosted_amiga.c`,
`stdlib/std.amiga.graphics.clo` (spike, untracked), `stdlib/std.amiga.clo` peers,
`spec/appendix-amiga.md`, `spec/appendix-stdlib.md`, `spec/check.sh`,
`compiler/tests/corpus_spec_test.rs`, `docker/scripts/corpus-amiga.sh`,
`docker/scripts/corpus-fs-uae.sh`, `plans/00-roadmap.md`, `AGENTS.md`.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 1 | 🟡 |
| 3 | Logical Contradictions | 0 | — |
| 4 | Completeness Gaps | 1 | 🟠 |
| 5 | Dependency Issues | 1 | 🟠 |
| 6 | Feasibility Concerns | 0 | — |
| 7 | Testability | 2 | 🟡 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 0 | — |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 | 🟠 |
| 12 | Consistency | 2 | 🔵 |
| 13 | Codebase Alignment | 3 | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 3 | 3 pending |
| MINOR | 5 | 5 pending |
| OBSERVATION | 2 | 2 pending |

---

## Findings

### PF-001: The plan ignores the spike implementation already present in tracked files 🟠 MAJOR

**Dimension:** 13 Codebase Alignment (Impact Blindness) + 4 Completeness
**Location:** `02-current-state.md` "Relevant Files"; `99-execution-plan.md` tasks 1.2.1–1.2.3, 1.3.1–1.3.2
**Codebase Evidence:** `git status` shows `?? stdlib/std.amiga.graphics.clo`,
`M compiler/src/stdlib.rs`, `M compiler/runtime/shim_hosted_amiga.c`. That file
already declares `Screen`, `openScreen`, `setColor`, and `fill`; `stdlib.rs`
already registers `std.amiga.graphics` (lines 61-62, 152); the shim already
defines `cl_open_screen`, `cl_set_color`, `cl_fill`, and `cl_close_screen`
(lines 579, 611, 620, 630).

**The Problem:** The plan describes Phase 1 as if starting from an empty tree
("Create `stdlib/std.amiga.graphics.clo`", "Register both modules", "Implement
the six functions"). In reality the spike has already added a _single_ graphics
module that owns the `Screen` handle and four shim functions. Executing the plan
as written would either overwrite/collide or silently diverge. The plan's stated
baseline ("includes the `spikes/05-graphics/` spike") does not name these
production edits.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Update `02-current-state.md` to list the spike's tracked edits, and rewrite tasks 1.2.1–1.2.3/1.3 as "relocate `Screen` to `std.amiga.intuition`, rewrite `std.amiga.graphics` to import it, add `cl_draw`/`cl_clear`" | Accurate baseline; no wasted work; keeps the working spike | Plan edits |
| B | Revert the spike's production edits (`git checkout -- stdlib.rs shim...` and delete the module), then implement fresh | Clean baseline | Throws away working, registered code; re-does the shim |

**Recommendation:** Option A — the spike is correct and already passes
`make check`; reconcile rather than redo.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-002: Phase 1.3.3 builds a program that Phase 2 has not created yet 🟠 MAJOR

**Dimension:** 5 Dependency Issues + 11 Ordering & Sequencing
**Location:** `99-execution-plan.md` task 1.3.3 vs task 2.1.1
**Codebase Evidence:** No `spec/tests/corpus/graphics/program.clo` exists
(`ls spec/tests/corpus/` shows file-copier, string-parser, buffer-container,
interrupt-counter, freestanding).

**The Problem:** Task 1.3.3 says "Build the graphics corpus program for
hosted-amiga and confirm zero warnings", but the graphics corpus program is
created in Phase 2 (task 2.1.1). A task references an entity that does not exist
at its point in the sequence.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Build an inline probe program (or the existing spike program) in 1.3.3; leave the corpus program to Phase 2 | No reordering; keeps the warning check | One throwaway probe |
| B | Move the corpus program creation (2.1) before the shim check | Logical flow | Moves a Phase 2 deliverable into Phase 1 |

**Recommendation:** Option A — smallest change; the probe is a two-line program.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-003: "Clipped by graphics.library" is wrong for pens and colors 🟠 MAJOR

**Dimension:** 2 Implicit Assumptions + 13 Codebase Alignment
**Location:** `03-02-drawing.md` "Error Handling"; `00-ambiguity-register.md` AR-14
**Codebase Evidence:** The NDK prototypes are
`SetRGB4(struct ViewPort *, LONG index, ULONG r,g,b)` and
`SetAPen(struct RastPort *, ULONG pen)`
(`/opt/amiga/.../clib/graphics_protos.h:103,112`). Clipping applies to
coordinates (`RectFill`), not to pen/color indices.

**The Problem:** The plan states an out-of-range pen or color index is "clipped
by `graphics.library`". That is not what clipping means here; an index beyond the
screen depth or the 32 color registers is undefined by this wrapper and may be
ignored or produce wrong output. The wording hides a real caller-contract gap.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Reword to "caller contract; out-of-range indices are undefined by this wrapper" (keeps AR-14's no-check decision) | Honest; smallest | No runtime safety |
| B | Clamp the pen in the shim to the screen depth and the color index to 31 | Safer | New behavior, more shim code |

**Recommendation:** Option A — matches the AR-14 decision; the demo uses
in-range pens.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-004: `openScreen` title lifetime is asserted but not verified 🟡 MINOR

**Dimension:** 13 Codebase Alignment (Stale/Unverified) + 2 Implicit Assumptions
**Location:** `01-requirements.md` §Security; `03-03-runtime-shim.md`
**Codebase Evidence:** Unable to verify `OpenScreen`'s `DefaultTitle`
copy-vs-store behavior from the codebase; the NDK header only declares
`OpenScreen(CONST struct NewScreen *)`.

**The Problem:** The plan asserts "the shim does not retain `title`", but if
`OpenScreen` stores the `DefaultTitle` pointer instead of copying it, the
caller's `String` must outlive the screen. The corpus program happens to keep
`title` alive (declared before `screen`, so it drops after), but the contract is
unstated and unverified.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add a Phase 1 verification task and document "the `title` must outlive the screen" | Correct and explicit | Small extra task |
| B | Have the shim keep its own copy of the title | Removes the caller burden | Extra allocation in the shim |

**Recommendation:** Option A — smallest; the corpus already satisfies it.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-005: ST-5's red phase fails for the wrong reason 🟡 MINOR

**Dimension:** 7 Testability + 11 Ordering & Sequencing
**Location:** `07-testing-strategy.md` ST-5; `99-execution-plan.md` task 1.1.1/1.1.2
**Codebase Evidence:** `E10010` is an existing compiler rule (there is a
`spec/tests/compile-fail/E10010_possibly_null_use.clo` golden).

**The Problem:** ST-5 tests that an unchecked `Screen?` is rejected with
`E10010`. Written before the modules exist, the probe fails with an unresolved
import, not `E10010` — so the red phase does not demonstrate the intended
behavior. Only ST-2 has a meaningful red phase.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Treat ST-5 as a green-only regression (the rule already exists) and say so; keep the red phase for ST-2 | Accurate | None |
| B | Keep ST-5 red but assert the failure is "unresolved module", then re-point after implementation | Contrived | Confusing |

**Recommendation:** Option A — ST-5 is a regression guard, not new behavior.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-006: The plan does not update `AGENTS.md` 🟡 MINOR

**Dimension:** 13 Codebase Alignment (Convention Violations)
**Location:** `00-index.md` "Related Files"; `99-execution-plan.md` (no AGENTS task)
**Codebase Evidence:** `AGENTS.md` "Test naming map" enumerates the spec-test
files and the stdlib/shim surface conventions.

**The Problem:** The plan adds two test files
(`graphics_spec_test.rs`, `graphics_impl_test.rs`), two stdlib modules, and shim
functions, but no task updates `AGENTS.md`, which is the project's routing doc.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add an AGENTS.md update task in Phase 3 | Stays consistent | One task |
| B | Leave `AGENTS.md` unchanged | Smaller | Drift from the doc's naming map |

**Recommendation:** Option A.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-007: ST-3/AR-35 cross-import should exercise the existing slice test 🟡 MINOR

**Dimension:** 7 Testability + 13 Codebase Alignment
**Location:** `99-execution-plan.md` task 1.2.4; `00-ambiguity-register.md` AR-35
**Codebase Evidence:** `make check` contains
`stdlib_slice_compiles_with_no_diagnostics_under_hosted_amiga`, which compiles
the embedded slice. That test may be the first place the cross-import fails.

**The Problem:** AR-35's probe names a program that imports both modules, but
the most likely early failure is the existing slice test, which the task does
not name.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Name the slice test explicitly in task 1.2.4 | Catches the failure earliest | None |
| B | Leave the probe as a standalone program | Slightly narrower | May miss the slice path |

**Recommendation:** Option A.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-008: "call a method on a null-narrowed handle" wording is inaccurate 🟡 MINOR

**Dimension:** 12 Consistency
**Location:** `03-02-drawing.md` "Error Handling"
**Codebase Evidence:** The drawing API is free functions
(`std.amiga.graphics`), not methods; the nullable rule is a language rule
(Ch 02 §5), enforced as `E10010`.

**The Problem:** The wording implies methods and does not cite the rule, which
can mislead the implementer about what the compiler enforces.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Reword to reference the nullable rule and `E10010` | Accurate | None |
| B | Leave as-is | — | Slightly misleading |

**Recommendation:** Option A.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-009: Video-chapter description is inconsistent between documents 🔵 OBSERVATION

**Dimension:** 12 Consistency
**Location:** `00-index.md` "Related Files" (`demo/tape/graphics*.tape`) vs `03-04-spec-corpus-video.md` ("a short terminal chapter (optional) or none")
**The Problem:** One doc lists a tape the other calls optional/nonexistent.

**Recommendation:** State that the graphics chapter is emulator-only, and drop
the `demo/tape/graphics*.tape` reference if no terminal tape is planned.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

### PF-010: The spike evidence is untracked 🔵 OBSERVATION

**Dimension:** 13 Codebase Alignment (Migration & Compatibility)
**Location:** `02-current-state.md` references `spikes/05-graphics/results/screen.png`
**Codebase Evidence:** `git status` shows `?? spikes/05-graphics/` and
`?? stdlib/std.amiga.graphics.clo`.

**The Problem:** The plan's proof of feasibility is uncommitted, so it can be
lost and cannot be cited by a fresh session.

**Recommendation:** Commit the spike (and its `results/`) before execution.

**User Decision:** Resolved — User accepted the recommendation (applied)

---

## Iteration 2 — Re-scan and Resolution

Fixes were applied to the plan documents (the audit target) per the accepted
recommendations. Verification and the bounded 13-dimension re-scan:

| Finding | Resolution applied | Verified |
|---|---|---|
| PF-001 🟠 | `02-current-state.md` now lists the spike's three production edits; execution tasks 1.2.1–1.2.3 and 1.3.2 rewritten as reconcile/relocate/rewrite; a Precondition note added | ✅ `grep` finds no "Create stdlib/std.amiga.graphics.clo" |
| PF-002 🟠 | Task 1.3.3 now builds an inline probe (the corpus program is Phase 2) | ✅ |
| PF-003 🟠 | `03-02` error table and AR-14 reworded (clipping is for coordinates, not pens) | ✅ no "clipped by graphics.library" |
| PF-004 🟡 | Task 1.3.4 verifies `OpenScreen`'s title behavior; `01`/`03-03` state the title must outlive the screen | ✅ |
| PF-005 🟡 | ST-5 marked a green-only regression; red phase scoped to ST-2 | ✅ |
| PF-006 🟡 | Task 3.1.4 updates `AGENTS.md`; listed in index/current-state | ✅ |
| PF-007 🟡 | Task 1.2.4 names `stdlib_slice_compiles_with_no_diagnostics_under_hosted_amiga` | ✅ |
| PF-008 🟡 | `03-02` null-screen row reworded and cites the nullable rule / `E10010` | ✅ |
| PF-009 🔵 | Video chapter stated emulator-only; the `demo/tape/graphics*.tape` reference removed | ✅ |
| PF-010 🔵 | Precondition records that the spike evidence should be committed with the plan | ✅ |

**Regression check:** the fixes introduced no new references, ownership changes,
dependency ordering changes, or contract changes. The task counts were also
reconciled (13 + 6 + 4 + 4 = 27); the earlier header/table totals were internally
inconsistent, which the fresh scan caught and corrected.

**Bounded 13-dimension re-scan:** no new findings. No critical, major, or minor
items remain; both observations are resolved.

## Verdict

✅ **PREFLIGHT PASSED** — all 10 findings resolved (3 major, 5 minor, 2
observation); 0 critical; no new findings in the iteration-2 re-scan.

**Same-session review note:** the plan and this audit share one session, so
same-agent bias applies. The major findings were re-verified against `git status`
and the NDK headers rather than trusting the plan's prose. For maximum
independence before a large execution, a fresh-session re-scan remains advisable.
