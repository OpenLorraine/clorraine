## Preflight Report: amiga-fault-exit-cleanup

> **Status**: ✅ PREFLIGHT PASSED — all 8 findings resolved (0 critical, 3 major, 3 minor, 2 observation)
> **Iteration**: 2 (re-scan after fixes)
> **Artifact**: Implementation plan at `plans/amiga-fault-exit-cleanup/` (11 documents, committed at `5f283e71`; `99-execution-plan.md` blob `5c992c88`, `03-01-exit-registry.md` blob `a56fd763`)
> **Codebase Grounded**: 14 source/script files examined, 6 m68k/`vamos` probe builds, ~70 references verified
> **Independence**: uniform-effort (the plan was written by the same model in an earlier session; one independent challenger confirmed all majors)
> **Last Updated**: 2026-11-26

Audit target: the whole plan folder. Context documents (not audited): `plans/00-roadmap.md`,
`plans/independent-assessment/`. Modification set: none until the user authorizes fixes.
Scope mode: strict (no optional additions reported).

### Codebase Context Summary

**Tech Stack:** std-only Rust workspace (`clc`, `clorraine`, `clorraine-lsp`); emitted C99; hosted-amiga
builds with `m68k-amigaos-gcc` + libnix (`nix20`/`nix13`), `vamos`, FS-UAE.
**Architecture:** the shim `shim_hosted_amiga.c` is prepended to every hosted-amiga TU; suffix files
are appended on `hosted-amiga` only (`profile.rs:18-28,131-149`). Release builds use section GC
(`toolchain.rs:93-99`); `-Werror` applies to debug builds only (`toolchain.rs:116-120`).
**Key Files Examined:** `shim_hosted_amiga.c` (`cl_trap`/`cl_panic` :541-574, servers :588-691,
messages :2986-3094, tasks :3096-3228, devices :3270-3589, window close :1669-1729), `optional.c`
:17-31, `profile.rs`, `toolchain.rs`, libnix `startup/ncrt0.S`, `nix/stdlib/atexit.c`,
`nix/misc/__initlibraries.c`, `stubs/libbases/utility.c`, `docker/scripts/run-fault-amiga.sh`,
`a500-diskfont-smoke.sh`, `a500-gui-smoke.sh`, `a500-exec-smoke.sh`, `tools/perf/bench.py`
:660-741, `compiler/tests/amiga_exec_spec_test.rs`, `stdlib/std.amiga.*.clo`.

**Verified:** every probe API exists (`lockPath`, `pathInfo`, `createPublicPort`, `putMsg`,
`openLibrary`, `addHandler`, `newBitmap`, `openDisplay`, `displayFlip`, `openWorkbenchWindow`,
`openDiskFont`, `createTask`, `joinTask`, `openTimer`, `timerWait`, `waitSignals`, `newTagList`,
`currentDateStamp`, `createSemaphore`); the 03-02 tables cover all 52 registered kinds plus `Task`;
function line references match (within re-location by name). Baseline `vamos` runs: lock, port,
library, and utility/semaphore faults all print a line matching the leak pattern on both profiles
(red phase is real); clean `exec-self-test` and `hamurabi` print none on either profile. libnix
auto-opens `UtilityBase` through `__LIB_LIST__` (`stubs/libbases/utility.c:9-10`,
`__initlibraries.c:46-58`), so the residual about a shim-opened `UtilityBase` does not arise under
GCC. libnix `exit`/`_exit` share one entry that re-runs the current exit priority
(`ncrt0.S:58-69,86-108`), confirming AR-9 rule 1's re-run premise.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 1 (PF-006) | 🟡 |
| 2 | Implicit Assumptions | 1 (PF-003) | 🟠 |
| 3 | Logical Contradictions | 2 (PF-001, PF-004) | 🟠 |
| 4 | Completeness Gaps | 1 (PF-002) | 🟠 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 (PF-003) | 🟠 |
| 7 | Testability | 1 (PF-002) | 🟠 |
| 8 | Security Blind Spots (Exec safety) | 1 (PF-005) | 🟡 |
| 9 | Edge Cases | 2 (PF-002, PF-004) | 🟠 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 2 (PF-007, PF-008) | 🔵 |
| 13 | Codebase Alignment | 1 (PF-001) | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 3 | all resolved |
| MINOR | 3 | all resolved |
| OBSERVATION | 2 | all resolved |

---

### PF-001: Fault-path re-entry links the whole exit hook into every program 🟠 MAJOR

**Dimension:** Logical Contradictions / Codebase Alignment (Impact Blindness)
**Location:** `03-01-exit-registry.md` §Proposed Changes (line 29-30) and §Re-entry (lines 96-98); `03-03` §Detection
**Codebase Evidence:** `cl_trap`/`cl_panic` are `static inline` (`shim_hosted_amiga.c:544-574`) and reached from almost every program: `hello.clo` reaches them through `cl_string_*` → `cl_alloc` (`:696-700`); a release build of hello (no `-s`) links `_printf`/`_vfprintf` only through them. libnix `exit` and `_exit` share one entry that resets SP and re-runs the current exit priority, and `__exitatexit` walks without unlinking (`ncrt0.S:58-69`, `atexit.c:28-33`).
**The Problem:** §Re-entry makes `cl_trap`/`cl_panic` call `cl_exit_hook` directly. Section GC cannot drop a function referenced from code every program keeps, so the hook and all five phases (`RemIntServer`, `RemTask`, registry walk, `CloseLibrary`) link into every program. That contradicts AR-18 ("links no registry or hook code"; "at most one load and compare") and makes ST-9 (hello shows no `cl_exit_hook`) fail. It also risks the `bench-compare` 1% `.text` guard (container `.text` 24108 B → 241 B budget). The direct call is redundant: a nested `exit(20)` already re-runs the idempotent hook.

**Options:** only one is viable.
Drop the drain call and the walking-flag branch from `cl_trap`/`cl_panic`. On re-entry they keep today's `exit(20)`, and libnix re-runs the hook. The worker check reads the worker-list head (one static load and compare). If the inline worker path (`FindTask`/`Signal`/`Wait`) must also cost nothing in programs without workers, route it through one static function pointer that `cl_create_task` sets. Update 03-01 §Re-entry, the 03-01 overview sentence, and 03-03 §Detection to match.
Considered and dropped: keeping the drain call through a pointer set at hook install. It works, but it adds machinery with no behaviour the nested `exit` doesn't already give.

**Recommendation:** Apply the single option above. Confidence: High. Hardening: challenger CONFIRMED (major) and refined it so that the function pointer is optional.

**User Decision:** Resolved — User accepted recommendation: drop the drain call (03-01 §Re-entry/overview, 03-03 §Detection, 02 risks, 07 impl row, 99 tasks 1.2.2 and 4.3.1, AR-9 rule 1 refinement note)

---

### PF-002: A replied program-created message leaks on its own reply port 🟠 MAJOR

**Dimension:** Completeness Gaps / Edge Cases / Testability
**Location:** `03-01` §Messages in flight (lines 130-138); `03-02` Message row (line 39); `07` ST-3
**Codebase Evidence:** `cl_reply_msg` only calls `ReplyMsg` (`shim_hosted_amiga.c:3072-3078`). The canonical flow in `examples/amiga/exec-self-test.clo:39-85` is `putMsg(inbox)` → `getMsg(inbox)` → `replyMsg` → `waitSignals(reply_mask)` → `getMsg(reply)`.
**The Problem:** Only `cl_put_msg` adds to the in-flight set, and a message leaves it at `getMsg`. After `replyMsg`, the program-created message is queued on the program's own reply port, but neither the registry nor the in-flight set tracks it. A fault there leaks it when the reply port is deleted, and so does a Ctrl-C in `waitSignals`, which this plan makes breakable. That breaks FR-5/AR-21. The exit thunk for a received own `Message` (its drop is the reply) leaks the same way. ST-3 covers only a message that was never fetched.

**Options:** only one is viable.
`cl_reply_msg` of a program-created message puts it back into the in-flight set. The shim knows the message is program-created because it left the set at `getMsg`; record that in the `Message` record's thunk. Reverse order releases the `Message` before its port, so the port release then frees it. Add an ST case (ST-3b) on both profiles: fault between `replyMsg` and the reply `getMsg`; expected first line, exit 20, no leak line.
Considered and dropped: tracking every program-created message for its whole life. It is the same mechanism with more bookkeeping.

**Recommendation:** Apply the single option above. Confidence: High. Hardening: challenger CONFIRMED (major).

**User Decision:** Resolved — User accepted recommendation: replying a program-created message hands it back to the in-flight set; ST-3b `reply-fault.clo` added (03-01, 03-02, 03-05, 07, 99 tasks 1.1.1–1.1.3, 1.2.3)

---

### PF-003: The A500 break steps have no live capture or break-delivery mechanism 🟠 MAJOR

**Dimension:** Feasibility Concerns / Implicit Assumptions
**Location:** `03-05` §A500 gate (lines 53-70); `07` ST-15, ST-16; `99` task 5.1.1
**Codebase Evidence:** `a500-diskfont-smoke.sh:131-137` reads the Workbench HDF only after FS-UAE stops ("The HDF is locked by FS-UAE, so poll only after the run"). Live polling elsewhere uses `grep -a` on a raw floppy ADF (`a500-gui-smoke.sh:185-197`, `a500-exec-smoke.sh:~130-167`).
**The Problem:** The gate copies the HDF (diskfont pattern) and sends `ctrl+c` "once the probe's `waiting` line appears in the capture". The plan never says:
- where probe and walker output go;
- how `waiting` is seen while the HDF is locked;
- how a console Ctrl-C reaches a probe whose output is redirected.

On 1.3 the CLI runs commands on its own process, so delivery is likely, but it is unproven, and xdotool needs window focus. Unable to verify the 1.3 CON: break target from a cited source; flag for red-phase proof. ST-15 and ST-16 cannot be built as written.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Keep xdotool (AR-17). Boot the WB 1.3.5 HDF with a second, blank capture ADF in DF0. The startup-sequence redirects walker and probe output (including `***Break` and the `rc fail` echo) to `DF0:` files. The gate polls the raw ADF with `grep -a` for `waiting`, focuses the window, and sends `ctrl+c`. It reads the files after the run. Task 5.1.2's red phase proves that a console Ctrl-C reaches the redirected probe. | Uses only the existing patterns; keeps AR-17 and ST-15's `rc fail` | One more FS-UAE drive; delivery proof deferred to the red phase |
| B | In-guest `Run` + `C:Break` instead of xdotool | Deterministic, no host input | Reopens AR-17; a `Run` probe's rc is unobservable, so ST-15 loses `rc fail` |

**Recommendation:** Option A. It fills the gap inside the user's AR-17 decision. Confidence: Medium-High (delivery unproven until the red phase). Hardening: challenger CONFIRMED (major) and picked A.

**User Decision:** Resolved — User chose Option A: capture ADF plus xdotool (03-05 §A500 gate; 99 tasks 2.1.1 and 5.1.2)

---

### PF-004: Exit phase 2 prints before it pops, so a nested exit reprints the worker line 🟡 MINOR

**Dimension:** Logical Contradictions / Edge Cases
**Location:** `03-03` §Exit removal (lines 56-62); `03-01` §Re-entry ("every phase pops before it acts"); `03-04` §During the exit (lines 65-67)
**Codebase Evidence:** libnix re-runs the hook on a nested `exit` (`ncrt0.S:58-69`).
**The Problem:** Phase 2 prints an unreported worker fault first and unlinks the record last. 03-04 itself expects libnix's stdio check to raise SIGINT while that line prints. The resulting nested `exit(20)` re-runs phase 2, which sees the record still unreported and prints the line twice.
**Options:** only one is viable. Mark the record reported (or unlink it into a local) before printing; this keeps 03-01's pop-before-act rule.
**Recommendation:** Apply the single option above.
**User Decision:** Resolved — User accepted recommendation: mark reported before printing in join and phase 2 (03-03)

---

### PF-005: The worker's fault-path list walk is unprotected against the parent freeing a record 🟡 MINOR

**Dimension:** Security Blind Spots (Exec safety)
**Location:** `03-03` §Proposed Changes (Worker list, Detection)
**Codebase Evidence:** workers run at priority -1 and the parent at ≥ 0 (`shim_hosted_amiga.c:3174-3195`). The join `FreeMem`s the record (`:3156-3157`).
**The Problem:** Only link and unlink run under `Forbid`. A faulting worker walks the list without it. With two workers, the parent can preempt that walk, join the other worker, and `FreeMem` the record the walker is about to read: a use-after-free on the very path this plan makes safe.
**Options:** only one is viable. Run the worker-path walk under `Forbid`/`Permit` (legal in a plain task). Take `Permit` before `Signal` and `Wait(0)`.
**Recommendation:** Apply the single option above.
**User Decision:** Resolved — User accepted recommendation: worker-path walk under Forbid (03-03, 07, 99 task 4.3.1)

---

### PF-006: The break rule for a non-waiting window-event poll is undefined 🟡 MINOR

**Dimension:** Ambiguities
**Location:** `03-04` §Breakable waits, `cl_window_next_event` row
**Codebase Evidence:** `cl_window_next_event(handle, wait)` waits only when `wait` is true (`shim_hosted_amiga.c:2911-2941`).
**The Problem:** The new form "loop: check break, GetMsg, Wait(...)" doesn't say whether a poll (`wait == false`) checks the break. If it does, every polling game loop gains a break exit that AR-12 did not list. If it doesn't, the executor has to guess.
**Options:** only one is viable. Check the break only when `wait` is true; a poll is not a wait (AR-12). State this in the row.
**Recommendation:** Apply the single option above.
**User Decision:** Resolved — User accepted recommendation: a poll never checks the break (03-04)

---

### PF-007: Handle-kind counts are misstated 🔵 OBSERVATION

**Dimension:** Consistency
**Location:** `02-current-state.md` line 32; `03-01` line 153; `07` line 13
**Codebase Evidence:** `grep "handle .* freedBy" stdlib/std.*.clo` finds 55 handle kinds: 53 in `std.amiga.*`, plus `File` (`std.io`) and `Buffer` (`std.mem`).
**The Problem:** The plan says "53 kinds across `std.amiga.*` and `std.io.clo`" and "52 non-libc kinds". The 52 includes `Task`, which goes through the worker phase rather than the registry. The tables themselves are complete.
**Options:** only one is viable. Change the text to "53 `std.amiga` kinds: 52 registered plus `Task` (worker phase)".
**Recommendation:** Apply the single option above.
**User Decision:** Resolved — User accepted recommendation: counts corrected (02, 03-01, 07)

---

### PF-008: The ST-18 probe row omits the timer it uses 🔵 OBSERVATION

**Dimension:** Consistency
**Location:** `03-05` probe table, `worker-fault-unjoined.clo` row; `07` ST-18
**The Problem:** ST-18 has the parent call `timerWait(timer, 50)` before its fault. The 03-05 "Acquires, then" column doesn't list `openTimer`. The ST case owns the expected behaviour, so this is only a wording gap.
**Options:** only one is viable. Add `openTimer`/`timerWait(50)` to the 03-05 row.
**Recommendation:** Apply the single option above.
**User Decision:** Resolved — User accepted recommendation: timer added to the ST-18 probe row (03-05)

---

### Adversarial checklist

- Confirmation risk: the plan says "verified in planning" for libnix exit semantics. I re-derived them from `ncrt0.S`/`atexit.c` and confirmed them, so they are not just restated.
- External standard: ISO C leaves a second `exit` from an `atexit` handler undefined. The plan relies on libnix-specific behaviour, which is verified from source; vbcc stays build-only (existing residual). The 1.3 CON: break target could not be cited; see PF-003.
- Domain expert view: removing an unjoined worker with `RemTask` without waiting is an accepted risk under AR-9 rule 6 and matches the existing join; not re-litigated.

---

## Iteration 2 — verification

> **Status**: ✅ PREFLIGHT PASSED — all 8 findings resolved
> **Previous Iteration**: 8 findings — all resolved
> **This Iteration**: 0 new findings
> **Carried Forward**: none

- **Fixes verified:** each resolution appears in the documents listed in its decision. A grep for
  stale text (`walking`, `_exit(20)`, drain wording, `ST-1..ST-6`/`ST-1..ST-9` ranges, the old
  counts) leaves only the register's original AR-9 rule 1 text, which the new refinement note
  qualifies, and accurate uses.
- **Regression check:** one consequence of the PF-003 fix was corrected under the same
  identifier. With a disk in DF0, Kickstart 1.3 would try to boot from it before the HDF, so the
  capture ADF moved to DF1 (`--floppy-drive-1`). The break key is sent once, without retries,
  so a stray second break cannot reach the next command. Task 5.1.2 now names ST-16's
  `break bit` as the delivery proof, with a stop-and-ask on failure.
- **Bounded fresh scan (13 dimensions, changed sections):**
  - 03-01 §Re-entry now matches 03-04 §During the exit, AR-18, and ST-9.
  - The PF-002 hand-off stays consistent with reverse-order release: the received `Message`
    is released before its port.
  - The PF-005 `Permit` comes before the field stores. That is safe because the parent frees
    the matched record only after this worker's signal, or after `RemTask`.
  - No new task was added: 55 tasks remain, and ST-3b folds into tasks 1.1.1–1.1.3.
- **Codebase re-check:** the new references (`a500-diskfont-smoke.sh:131-137`,
  `a500-gui-smoke.sh:185-197`, `ncrt0.S:58-69,86-108`, `atexit.c:28-33`) were read during this run.

Residual risk, accepted in the plan itself: on Kickstart 1.3, delivering a console Ctrl-C to a
probe with redirected output is unproven until the Phase 5 red phase. Consider a human review of
the A500 gate script by someone with 1.3 CON: handler experience.
