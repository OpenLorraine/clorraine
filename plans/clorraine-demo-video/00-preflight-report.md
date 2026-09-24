# Preflight Report: Clorraine Demo Video Toolkit

> **Status**: ✅ PREFLIGHT PASSED — all 22 findings resolved (+1 clean observation)
> **Iteration**: 2 (re-scan after fixes)
> **Artifact**: implementation plan at `plans/clorraine-demo-video/` (10 documents)
> **Codebase Grounded**: 24 source/config files examined, ~40 references verified
> **Same-session review**: yes — the plan was authored in the current session
> **Last Updated**: 2026-09-24 22:05

## Iteration 2 — resolutions

All 22 findings were resolved in the artifact; a bounded re-scan (the whole target, plus
a consistency pass) confirmed the fixes and found no regressions.

| Finding | Resolution |
| ------- | ---------- |
| PF-001 | AR-16 amended: scene ≡ chapter; every adjacent scene crossfades (AR-16; 03-02 §Compose) |
| PF-002 | `--only` + `--out` added; `make demo` re-renders `demo/emulator.mp4` (03-01 CLI; 03-04 §Makefile) |
| PF-003 | compose trims each clip to its `duration` (03-02 §Compose) |
| PF-004 | chained `xfade` recurrence + `n=1` guard specified (03-02; ST-10) |
| PF-005 | `npm install`/lockfile + `make demo-check` move to task 1.2.1; 4.2.2 adds only `demo` (99) |
| PF-006 | ST-11 smoke moved to a separate opt-in smoke section as ST-13 (07) |
| PF-007 | handle list pinned; synthetic all-token fixture; theme-completeness dropped (03-03; ST-3) |
| PF-008 | ST-12 now asserts `make -n` text (07; 99 4.1.1) |
| PF-009 | `CHROME_BIN` resolver seam (03-01; ST-8) |
| PF-010 | `$HOME/go/bin` PATH fixup in `util/vhs.ts` (03-02; 01 §Compatibility) |
| PF-011 | unique ids + positive `duration` validated (03-01; ST-1) |
| PF-012 | gif id validation + poster fallback (03-02; ST-11) |
| PF-013 | gif list → 4 ids; example relabelled a minimal sample; import path `../` (00-index) |
| PF-014 | `duration` terminology; outro type aligned; frame title from the file stem (01; 03-02) |
| PF-015 | scaffold title example corrected (03-04) |
| PF-016 | path canonicalization + absolute-path/write-path policy (03-01; ST-7) |
| PF-017 | tape escaping + trust boundary (03-02) |
| PF-018 | PNG size assert + font availability check (03-02) |
| PF-019 | missing/unreadable file validation; overflow behavior (03-01; 03-02) |
| PF-020 | `--root` seam, coverage command, ST-2 split, per-command `Sleep`, fixtures scheduled (03-01; 03-02; 07) |
| PF-021 | card fade and inert `profile` field removed; `--force` kept (03-01; 03-02; 03-04) |
| PF-022 | citations corrected; ST-13 seconds; AC-8 task added; v0.11 prose; collision rationale; dangling clause removed |

**Iteration 2 result:** no unresolved 🔴/🟠; all 🟡 resolved by edit; PF-023 was a clean
observation. Tier = **✅ PASSED**.

---

## Iteration 1 — findings (audit trail)

> **Status (iteration 1)**: ❌ blocked — 12 critical/major unresolved


## Codebase Context Summary

**Tech Stack:** Rust workspace (`Cargo.toml:3-4` std-only, no crates); bash demo pipeline; host Node 22.23, Chrome 152, ffmpeg 6.1.1, ttyd, VHS v0.11.0 (`~/go/bin`), xwininfo.
**Architecture:** `demo/story.sh` (281 lines) hard-codes the story; `demo/record-emulator.sh` (136) captures FS-UAE via `x11grab`; four static VHS tapes; `Makefile` has `check`/`verify`/`run`/`build`, no demo target.
**Key Files Examined:** `demo/story.sh`, `demo/record-emulator.sh`, `demo/show-source.sh`, `demo/clorraine.vim`, `demo/tape/*.tape`, `Makefile`, `Cargo.toml`, `README.md`, `AGENTS.md`, `examples/hello/hello.clo`, `spec/tests/corpus/*/program.clo`, `docker/scripts/run.sh`, `.gitignore`.

## Summary by Dimension

| # | Dimension | Findings | Highest |
|---|-----------|----------|---------|
| 1 | Ambiguities | 2 | 🟠 |
| 2 | Implicit Assumptions | 1 | 🟠 |
| 3 | Logical Contradictions | 3 | 🟠 |
| 4 | Completeness Gaps | 4 | 🟠 |
| 5 | Dependency Issues | 2 | 🟠 |
| 6 | Feasibility Concerns | 3 | 🟠 |
| 7 | Testability | 5 | 🟠 |
| 8 | Security Blind Spots | 2 | 🟡 |
| 9 | Edge Cases | 3 | 🟠 |
| 10 | Scope Creep | 1 | 🟡 |
| 11 | Ordering & Sequencing | 3 | 🟠 |
| 12 | Consistency | 3 | 🟠 |
| 13 | Codebase Alignment | 3 | 🟠 |

## Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 12 | 12 pending |
| MINOR | 10 | 10 pending |
| OBSERVATION | 1 | 1 pending |

---

## MAJOR findings

### PF-001: "Chapter" is not a modeled concept, so AR-16's "within a chapter" is unimplementable 🟠 MAJOR
**Dimension:** Ambiguities / Consistency · **Location:** `00-ambiguity-register.md` AR-16; `03-01` §Types; `03-02` §Compose; `07` ST-10
**Codebase Evidence:** n/a (plan-internal) — `DemoConfig`/`SceneBase` have no group field.
**The Problem:** AR-16 says "0.5s crossfade between chapters; hard cuts within a chapter", but nothing groups scenes; the formula `Σdurations − (n−1)·transition` and ST-10 treat every scene as a chapter boundary.
**Options:** A — declare scene ≡ chapter and crossfade every adjacent scene; amend AR-16. B — add a `group`/`chapter` field and crossfade only at group boundaries.
**Recommendation:** A — the model is flat and the formula already assumes it; smallest change.

### PF-002: `demo/emulator.mp4` has no producing mechanism 🟠 MAJOR
**Dimension:** Completeness · **Location:** `01` AC-1; `99` 4.2.5; `03-04` §outputs; `03-01` `DemoConfig`
**Codebase Evidence:** `demo/story.sh:215` writes `demo/emulator.mp4` today.
**The Problem:** The schema has only `out/poster/gif` and `make demo` has no target for a scene-only export, so AC-1 is unreachable.
**Options:** A — add `extraOutputs: { out, sceneIds }[]`; B — add `--only <id> --out <path>` to `render` and have `make demo` run a second render; C — drop it from AC-1.
**Recommendation:** B — reuses one CLI flag, sources the otherwise-unsourced `--only`, and adds no schema field.

### PF-003: Terminal clips exceed their declared `duration`, breaking the composition math 🟠 MAJOR
**Dimension:** Edge Cases / Feasibility · **Location:** `03-02` §terminal, §compose
**Codebase Evidence:** `demo/story.sh:120-125` (`normalize`) does not trim; a VHS clip = typing + `Sleep`.
**The Problem:** `Σdurations`, the `xfade` offsets, and ST-10 are wrong for every terminal scene.
**Recommendation:** compose trims each clip to its declared `duration` (`ffmpeg -t`).

### PF-004: Chained `xfade` for 15 clips and the single-clip case are unspecified 🟠 MAJOR
**Dimension:** Feasibility · **Location:** `03-02` §compose step 2; `07` ST-10
**Codebase Evidence:** ffmpeg 6.1.1 `xfade` is binary (`VV->V`).
**The Problem:** 15 clips need 14 chained xfades with cumulative offsets; no `n=1` guard is stated.
**Recommendation:** specify the filtergraph recurrence and the `n<2` guard; add a spec case.

### PF-005: The lockfile/`npm ci` and `make demo-check` ordering has gaps 🟠 MAJOR
**Dimension:** Dependency / Ordering · **Location:** `99` 1.1, 1.2, 1.3.1, 4.2.2; AR-5
**The Problem:** Steps 1.1/1.2 verify with `make demo-check`, created only in 1.3.1; `package-lock.json` is required by AR-5 but no task installs deps or commits it; the target is scheduled twice.
**Recommendation:** install deps + commit the lockfile and create `demo-check` in 1.2.1; keep 1.3.1; 4.2.2 adds only the `demo` target.

### PF-006: ST-11 (smoke) is classified as a mandatory spec test but can never be red 🟠 MAJOR
**Dimension:** Testability · **Location:** `07` §ST cases, §Specification Tests; `99` 3.3.1
**The Problem:** ST-11 needs the full Chrome/VHS/ffmpeg pipeline and sits in a hardening step, violating the red-first rule.
**Recommendation:** move ST-11 out of the mandatory ST table into the opt-in smoke section.

### PF-007: ST-3 is unsatisfiable as written 🟠 MAJOR
**Dimension:** Testability · **Location:** `07` ST-3; `03-03` §Grammar
**The Problem:** the handle list is open-ended ("for example `Screen`") and "every scope has a theme color" is not a finite set; the three fixtures contain a small token subset.
**Recommendation:** pin the handle list, tokenize a synthetic all-token fixture, and drop the theme-completeness assertion.

### PF-008: ST-12 recurses and needs the emulator 🟠 MAJOR
**Dimension:** Testability · **Location:** `07` ST-12; `03-04` §Makefile; `99` 4.2.2
**The Problem:** `make demo-check` runs `node --test`, which would run ST-12 again; `make demo` needs Docker/FS-UAE/X.
**Recommendation:** assert `make -n demo`/`make -n demo-check` text; keep the real `make demo` as manual acceptance.

### PF-009: ST-8 cannot be tested deterministically (no Chrome seam) 🟠 MAJOR
**Dimension:** Testability · **Location:** `07` ST-8; `03-02` §Chrome
**The Problem:** "Chrome absent" is nondeterministic and no injection point exists.
**Recommendation:** add a `CHROME_BIN`/resolver seam and unit-test the resolver.

### PF-010: VHS is not on the default PATH, and the plan omits the fixup 🟠 MAJOR
**Dimension:** Implicit Assumptions / Codebase Alignment · **Location:** `01` §Compatibility; `03-02` §terminal
**Codebase Evidence:** `which vhs` fails; `~/go/bin/vhs` exists; `demo/story.sh:90` exports `$HOME/go/bin`.
**Recommendation:** carry the `$HOME/go/bin` PATH fixup into `util/vhs.ts`.

### PF-011: `id` uniqueness and `duration > 0` are unvalidated 🟠 MAJOR
**Dimension:** Edge Cases · **Location:** `03-01` §Types; `07` ST-1
**The Problem:** duplicate ids collide on `build/demo/raw/<id>.mp4` and make `gif.sceneIds` ambiguous.
**Recommendation:** validate unique non-empty ids and positive durations; add ST cases.

### PF-012: `gif.sceneIds` and the poster have no validation/fallback 🟠 MAJOR
**Dimension:** Edge Cases · **Location:** `03-02` §compose steps 4-5; `03-01` `DemoConfig`
**The Problem:** an unknown gif id is never validated and "first card scene" is undefined when there is no card.
**Recommendation:** validate gif ids; error (or fall back to the first scene) when no card exists.

## MINOR findings

### PF-013: 00-index Quick Reference is stale 🟡 MINOR
`00-index.md` §Usage Examples: `gif.sceneIds` has 3 ids (AR-17/03-04 say 4); the `demo/story.ts` example shows 4 scenes (the story is 15); and `03-01` §Example imports `../../tools/demo/...` where `../tools/demo/...` is correct.
**Recommendation:** fix the gif list to 4, relabel the 4-scene block as a minimal sample, and use `../tools/demo/...`.

### PF-014: Model/terminology mismatches 🟡 MINOR
`01` says card/outro take an "optional subtitle, optional lines" but the outro type has no subtitle and requires lines; `01` says "dwell time" while the model uses `duration`; the branded frame's "title" has no source field.
**Recommendation:** align `01` to the types; use `duration`; derive the frame title from the file stem.

### PF-015: The scaffold title example is wrong 🟡 MINOR
`03-04` claims `examples/hello/hello.clo` yields `The smallest Clorraine program`; the actual line is `The smallest Clorraine program: print one templated line.`
**Recommendation:** show the actual derived value (no new punctuation rule).

### PF-016: The path guard's mechanism is undefined for write paths 🟡 MINOR
`03-01` §Config Loading declares a repo-root escape guard without a canonicalization rule; `out`/`poster` are writes.
**Recommendation:** define canonicalization (realpath of the nearest existing ancestor), reject absolute paths, and add a spec case.

### PF-017: Tape generation can be broken or injected by scene text 🟡 MINOR
`03-02` §terminal interpolates `commands`/`id` verbatim into the tape.
**Recommendation:** reject or escape `"`/newlines and state the trust boundary at the writer.

### PF-018: Chrome frame size and font availability are unchecked 🟡 MINOR
`03-02` §Chrome does not assert the screenshot is `width×height`, and AR-27 covers only missing Chrome, not missing DejaVu fonts.
**Recommendation:** assert/scale the PNG dimensions; fail fast if the fonts are absent.

### PF-019: A missing code file or vertical overflow is undefined 🟡 MINOR
`03-01` §Error Handling has no "code file missing"; `03-02` §code silently clips with `scroll:false`.
**Recommendation:** validate file existence/readability at load; define overflow behavior.

### PF-020: Test seams and test-spec gaps 🟡 MINOR
No injectable repo root (ST-9 writes into the tree); no coverage command for the 90/80/70 targets; ST-2 blends frame count with encoded duration; ST-4 does not pin the `Sleep` placement; fixtures are unscheduled.
**Recommendation:** inject the root/target dir; name `node --test --experimental-test-coverage`; split ST-2 (fake-renderer loop count vs smoke duration); pin one trailing `Sleep` per command; fold fixtures into the spec-author tasks.

### PF-021: Unsourced extras in the specs 🟡 MINOR
`--force`, the card fade-in animation, and the inert single-valued `profile?: "hosted-amiga"` field appear in `03-01`/`03-02`/`03-04` with no requirement/AR.
**Recommendation:** keep `--force` (ST-9 needs it); drop the fade-in and the inert `profile` field.

### PF-022: Traceability and prose nits 🟡 MINOR
Weak ST `Source` citations (ST-1→AR-21, ST-7→AR-1); ST-11 has no numeric duration; AC-8 ("security and edge-case review complete") has no task; `03-02` §Error Handling says "v0.12.0 warning" where the pin is v0.11.0; `03-02` §emulator's collision-fix rationale is inaccurate; `99` 3.1.2 has a dangling "before Phase 2 exists" clause.
**Recommendation:** correct the citations, give ST-11 explicit seconds, tie AC-8 to the strict phase reviews, fix the version prose and the collision rationale, and remove the dangling clause.

## OBSERVATION

### PF-023: Complexity Escalation Gate — clean 🔵
Audited: `tools/demo/` + `shiki` + Chrome = AR-15; branded frame = AR-28; grammar = AR-2/AR-9; scaffold = AR-6. No unapproved extra machinery found.
