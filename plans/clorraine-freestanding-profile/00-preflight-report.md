# Preflight Report: Freestanding profile

> **Status**: ✅ PASSED — all 11 findings resolved (5 major, 6 minor), 0 remaining
> **Iteration**: 2 (re-scan after fixes; iteration 1 found 11)
> **Artifact**: Implementation plan at `plans/clorraine-freestanding-profile/` (9 documents)
> **Codebase Grounded**: 246 source/spec files enumerated; ~30 files read in depth; every plan `file:line` reference re-checked
> **Artifact content hash**: `sha256:a7f59ea8…10bf090e` (iteration-2 revision; folder is untracked, hash over the 9 `.md` files)
> **Last Updated**: 2026-09-23 19:25

> **SAME-SESSION REVIEW**: This plan was created in the current session. Same-agent
> bias risk is elevated. Consider a fresh-session re-scan after fixes for maximum
> independence.

### Codebase Context Summary

**Tech Stack:** Rust (std-only workspace), C99 emitted translation unit and runtime shims, Clorraine standard library (`.clo`), GNU Make, Docker SDK (`m68k-amigaos-gcc` 16.2, libnix, `vamos`).
**Architecture:** `clc` is emit-only; the `clorraine` driver (`compiler/src/clorraine/`) emits C in process, compiles it with a per-profile `Tool`, then runs it. `Profile` (`compiler/src/profile.rs`) bundles the shim and target facts. The frozen `spec/` is an immutable oracle except where the plan is authorized to edit it.
**Key Files Examined:** `compiler/src/profile.rs`, `driver.rs`, `codegen.rs`, `codegen/program.rs`, `codegen/function.rs`, `stdlib.rs`, `clorraine/{mod,artifacts,build,run,toolchain,cli}.rs`, `runtime/shim_host.c`, `spec/14-diagnostics.md`, `spec/15-profiles.md`, `spec/17-codegen.md`, `spec/check.sh`, `compiler/tests/{stage,compile_fail,run_fault,corpus}_spec_test.rs`, root `Makefile`.
**Live toolchain probes (SDK container):** host no-libc `_start` links and runs at `-O0`/`-O2`; m68k `-nostdlib -lg` links floats, and without `-lg` the link fails on `__muldf3`/`__gtdf2`; bare `nm -u` prints `no symbols` on the linked hunk.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 1 | 🟡 |
| 2 | Implicit Assumptions | 1 | 🟠 |
| 4 | Completeness Gaps | 2 | 🟠 |
| 6 | Feasibility Concerns | 1 | 🟡 |
| 7 | Testability | 1 | 🟠 |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 1 | 🟡 |
| 13 | Codebase Alignment (incl. Test Impact) | 4 | 🟠 |
| others (3,5,8,9,10) | — | 0 | — |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 5 | all resolved |
| MINOR | 6 | all resolved |
| OBSERVATION | 0 | — |

---

### PF-001: `E11213` is already the "assertion failed" runtime fault 🟠 MAJOR

**Dimension:** Codebase Alignment (stale assumption)
**Location:** `00-ambiguity-register.md:44` (AR-28), `03-01-profile-runtime.md:109`, `03-03-spec-goldens.md` §Diagnostics, `01-requirements.md:54` (R9)
**Codebase Evidence:** `spec/14-diagnostics.md:172` registers `E11213 = "assertion failed"`; it is emitted at `compiler/src/codegen/function.rs:294`, tested at `compiler/tests/run_fault_spec_test.rs:173-174`, and has a run golden `spec/tests/run/E11213_assertion_failed.clo`.
**The Problem:** The plan calls `E11213` "the new runtime code" and uses it for the freestanding file-I/O trap. The code already exists with a different meaning. Reusing it would give one code two incompatible first lines and contradict Ch 14; `spec/check.sh` only checks registration, so it will not catch the collision.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Use the next free code `E11214` for the file-I/O trap; register it in Ch 14 §11 and add it to `check.sh`'s runtime-fault list | No collision; keeps E11213's meaning; one-line change to the plan's code list | Touches one more registry line |
| B | Keep `E11213` and move `assert` to a new code | — | Reassigns a shipped, tested code; breaks existing goldens |

**Recommendation:** Option A — the next free code is `E11214`; E11213 is taken and tested.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-002: Removing the only `E11200` compile-fail golden breaks `spec/check.sh` 🟠 MAJOR

**Dimension:** Test Impact / Codebase Alignment
**Location:** `00-ambiguity-register.md:46` (AR-30), `03-03-spec-goldens.md` §Golden and corpus data, `99-execution-plan.md` task 3.1.4
**Codebase Evidence:** `spec/tests/compile-fail/E11200_unimplemented_profile.clo` is the **only** golden declaring E11200 (`grep -rln "expect-.*E11200" spec/tests/compile-fail/` → 1 hit). `spec/check.sh:165-177` requires every registered compile-time code not on the excluded list to have a compile-fail test; E11200 is registered and not excluded. A simulated removal in a scratch copy prints `ERROR: no compile-fail test for registered diagnostic E11200`.
**The Problem:** The plan deletes the golden and expects `E11201_unknown_profile.clo` to "keep that coverage", but E11201 covers a different code. Without a replacement, the spec check fails. (The Rust `ASSERTED_CODES` length check can be satisfied by removing E11200 from the list, but then E11200 loses all golden coverage.)

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Replace with a new E11200 golden that triggers the still-live profile-surface rule (a `host` program importing `std.amiga.dos`) | Keeps E11200's rule covered; mirrors the existing stage test | One new file |
| B | Remove E11200 from the registry/asserted set | — | E11200 is still emitted by the code; dropping its only directive hides the rule |

**Recommendation:** Option A — add `E11200_unavailable_module.clo` (host + `std.amiga.dos`), which the host gate rejects with `E11200` today.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-003: The root `Makefile` does not forward `PROFILE=freestanding` 🟠 MAJOR

**Dimension:** Codebase Alignment (stale assumption) / Dependency
**Location:** `03-02-driver.md:98-102` ("No new target is needed … already forwards `FILE`, `PROFILE` …"), `01-requirements.md:76` (R17), `00-ambiguity-register.md:22`
**Codebase Evidence:** `Makefile:34` `PROFILE_ARG = $(if $(filter hosted-amiga,$(PROFILE)),--profile hosted-amiga,)`; used at `Makefile:68,74`.
**The Problem:** Only `hosted-amiga` is forwarded. `make run FILE=x PROFILE=freestanding` passes an empty `--profile`, so the driver runs the **host** profile silently. The plan's claim that the Makefile needs no change is wrong, and acceptance criterion 9 / R17 would not hold.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Change `PROFILE_ARG` to forward any non-host profile: `$(if $(filter-out host,$(PROFILE)),--profile $(PROFILE),)`; correct the 03-02 text | One line; keeps `host` default | None material |
| B | Add a dedicated freestanding Make target | — | Duplicates the passthrough; more surface |

**Recommendation:** Option A — it is the smallest change that makes the existing passthrough true.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-004: `compiler/src/codegen/program.rs` is required but omitted from the authorized set 🟠 MAJOR

**Dimension:** Completeness Gaps / Codebase Alignment
**Location:** `00-ambiguity-register.md:38` (AR-22), `00-index.md:106-113` ("Read-only dependencies … program.rs"), `02-current-state.md:26` ("No change")
**Codebase Evidence:** `compiler/src/codegen/program.rs:83` emits the AmigaOS ABI typedefs only `if self.profile == Profile::Host`; `program.rs:112-113` unconditionally emits `APTR cl_string_ptr(...)` and `ULONG cl_string_len(...)`. A freestanding TU therefore references undeclared `APTR`/`ULONG`. `03-01-profile-runtime.md:83-87` and `99-execution-plan.md:71` task 2.1.4 require editing this file, but AR-22 and `02-current-state.md` say otherwise.
**The Problem:** The plan contradicts itself, and the authorized modification set does not cover a file that must change for the emitted C to compile. The "Read-only dependencies" line is also wrong.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add `program.rs` to the set and emit the ABI typedefs for `host` **and** `freestanding`; fix `02-current-state.md` to "change the typedef condition" | Matches the plan's 03-01 choice; consistent with host | None material |
| B | Define `APTR`/`ULONG` (and the rest) inside the freestanding shim | Keeps program.rs read-only | The shim is pushed before the typedefs; duplicates ABI names and mixes concerns |

**Recommendation:** Option A — the fix is already written in 03-01; only the authorized set and current-state doc lag.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-005: `compiler/src/clorraine/mod.rs` is required but omitted from the authorized set 🟠 MAJOR

**Dimension:** Completeness Gaps / Codebase Alignment
**Location:** `00-ambiguity-register.md:38` (AR-22), `03-02-driver.md:63-90`, `99-execution-plan.md` task 3.1.2
**Codebase Evidence:** `run_command` (`compiler/src/clorraine/mod.rs:190-220`) builds via `build::build` and runs `built.bin`; `required_tools` (`mod.rs:98-111`) adds only `tool(profile).program` and (for hosted-amiga) `vamos`. `run.rs` holds only `run_program`. The freestanding run must execute a **separate host artifact** and must require `cc`.
**The Problem:** The behavior the plan describes lives in `mod.rs`, which is not in AR-22; without editing it, `run --profile freestanding` would try to execute the m68k hunk natively and would not check for `cc`.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add `mod.rs` (and, if needed, `artifacts.rs`) to AR-22 and specify the run_command/required_tools changes | Covers the real edit site | Slightly larger set |
| B | Move the orchestration into `run.rs` first | — | A refactor with no functional gain; more churn |

**Recommendation:** Option A — edit `mod.rs` in place and record it in the set.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-006: The m68k "zero undefined symbols" check is vacuous with bare `nm` 🟡 MINOR

**Dimension:** Testability
**Location:** `00-ambiguity-register.md:43` (AR-27), `07-testing-strategy.md:47` (ST-11), `99-execution-plan.md:53,132`
**Codebase Evidence:** Probe (SDK container): the linked GCC hunk prints `nm: build/pf_amiga: no symbols` for bare `nm`; without `-lg` the link fails on `__muldf3`/`__gtdf2` — so **link exit 0** is the enforced gate. The prefixed `m68k-amigaos-nm -u` reads the hunk (independent check).
**The Problem:** As written ("parses `nm -u`, asserts 0 undefined"), the harness would pass vacuously on the bare `nm` the plan names.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Assert that the m68k link command exits 0 (the link fails on any unresolved symbol) | Real, simple, already measured | Relies on the linker's enforcement |
| B | Parse `m68k-amigaos-nm -u` output instead of bare `nm` | Also names the symbols | Tool-name coupling; redundant with A |

**Recommendation:** Option A as the primary assertion; mention B as an optional diagnostic. Update AR-27/ST-11 wording.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-007: The freestanding shim's FFI bindings are not covered by the existing unit test 🟡 MINOR

**Dimension:** Test Impact
**Location:** `00-ambiguity-register.md:38` (AR-22 omits `compiler/src/stdlib.rs`)
**Codebase Evidence:** `compiler/src/stdlib.rs:397-422` `each_profile_shim_defines_every_ffi_symbol_its_modules_bind` iterates `[Profile::Host, Profile::HostedAmiga]`. The freestanding surface binds `Open`, `Close`, `Read`, `Write`, `cl_write`, `cl_write_line`.
**The Problem:** The plan does not extend this test (or list `stdlib.rs`), so the freestanding shim's contract coverage is weaker than the other two profiles'.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Add `Profile::Freestanding` to the loop and add `stdlib.rs` to the set | Reuses an existing check; catches a shim/symbol mismatch | None material |

**Recommendation:** Option A — the only viable path; the test already encodes the invariant.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-008: The freestanding host artifact path has no home in `artifacts.rs` 🟡 MINOR

**Dimension:** Dependency / Completeness Gaps
**Location:** `03-02-driver.md:63-73` (`<stem>.host`)
**Codebase Evidence:** `compiler/src/clorraine/artifacts.rs` owns every driver path (`c_path`, `bin_path`, `out_path`, `program_dir`); it is not in AR-22.
**The Problem:** The new `<stem>.host` path is introduced by prose only; the plan should either add a helper to `artifacts.rs` (and list it) or state that `run.rs` joins the name locally.

**Recommendation:** Add a `host_bin_path` helper to `artifacts.rs` and include it in the set — keeps path policy in one module.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-009: The `clc` usage message still names only two profiles 🟡 MINOR

**Dimension:** Consistency
**Location:** `03-02-driver.md:92-96` (CLI text update)
**Codebase Evidence:** `compiler/src/driver.rs:231` prints `usage error: --profile <host|hosted-amiga> is required`.
**The Problem:** The plan updates `clorraine/cli.rs` help and the unknown-profile message but not this `clc` message.

**Recommendation:** Include `freestanding` in the `driver.rs:231` message and list `driver.rs` text in the plan.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-010: The `0` null-spelling rationale is inaccurate, and the freestanding `BPTR` type is unspecified 🟡 MINOR

**Dimension:** Ambiguities
**Location:** `03-01-profile-runtime.md:78-81`, `00-ambiguity-register.md:34` (AR-18)
**Codebase Evidence:** `compiler/src/codegen.rs:304-331` special-cases `HostedAmiga` because its `BPTR` is an integer; `program.rs:84` declares `BPTR` as `void *` for `host`.
**The Problem:** The stated reason ("the freestanding shim does not include `<stddef.h>`") is not the real driver; the real choice is whether freestanding's `BPTR` is a pointer or an integer. If freestanding mirrors host (`void *`), `NULL` would also work and the `0` substitution is cosmetic.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | State that freestanding declares `BPTR` as `void *` (like host) and therefore uses `0`/`NULL` interchangeably; keep `0` for consistency with the hosted-amiga provisional ABI | Accurate; removes a wrong reason | None |
| B | Make freestanding's `BPTR` an integer and rely on `0` | — | Diverges from host without a reason |

**Recommendation:** Option A — correct the rationale; the AR-18 decision itself stands.

**User Decision:** Resolved — user accepted the recommendation

---

### PF-011: The "`-lg` is a self-contained runtime" claim needs its constraint stated 🟡 MINOR

**Dimension:** Feasibility / Implicit Assumptions
**Location:** `00-ambiguity-register.md:26,61-71` (AR-10), `03-01-profile-runtime.md:150`
**Codebase Evidence:** Probe: `libg.a` also contains `malloc`/`exit` objects that reference `SysBase`; linking the probe with `-lg` failed when the m68k branch called `_exit` (undefined `_exit`/`SysBase`). With a m68k no-op halt (no `_exit`), the same probe linked.
**The Problem:** `-lg` is self-contained **only if** the runtime never references libg's libc-ish objects. The plan's m68k design does use no-op stubs, but the constraint is implicit; a future `abort`/`exit` call in the m68k path would reintroduce the failure.

**Recommendation:** State the constraint in 03-01 (the m68k runtime references no `exit`/`malloc`/`abort`), and add an implementation test that the m68k shim contains no `_exit`/`exit(`/`malloc(` reference.

**User Decision:** Resolved — user accepted the recommendation

---

## Hardening Disclosure

- **Hardening:** one independent challenger (general subagent) reviewed the MAJOR/CRITICAL batch instead of a same-context re-read. The challenger **confirmed** PF-001, PF-003, PF-004, PF-005; **refined** PF-002 (the Rust `ASSERTED_CODES` sub-claim is handleable by editing the list, but `spec/check.sh` still fails without a replacement golden); and **downgraded** the `nm` finding to MINOR (a prefixed `m68k-amigaos-nm` reads the hunk, and link exit 0 already proves self-containment).
- **Confidence:** High — every finding is grounded in a specific file and a repeated probe or grep.
- **No finding re-litigates an Ambiguity Register decision;** PF-001 and PF-002 arise from information the register did not have (E11213 was assumed free; the one-golden fact was not checked).

## Verdict

**✅ PASSED — all 11 findings resolved.** Every finding was accepted at the recommended option and applied to the plan set. Iteration 2 verified each fix against the edited documents and re-checked the changed references:

| Finding | Fix applied | Verified in |
|---------|-------------|-------------|
| PF-001 | `E11213` → `E11214` everywhere; `spec/check.sh` exclusion and Ch 14 registration named | AR-28/note, 00-index, 01 R9/R14/AC-5, 02, 03-01, 03-03, 07 ST-18, 99 |
| PF-002 | Obsolete golden replaced by `E11200_unavailable_module.clo`; `E11200` kept in the expectation set | AR-30/note, 03-03, 07 ST-22, 99 task 3.1.4 |
| PF-003 | `Makefile` `PROFILE_ARG` change specified; false "no change" claim removed | 03-02, 99 task 3.1.5 |
| PF-004 | `codegen/program.rs` added to AR-22; `02-current-state` row corrected | AR-22, 00-index, 02, 99 |
| PF-005 | `clorraine/mod.rs` added to AR-22 and the run flow; `artifacts.rs` added (PF-008) | AR-22, 00-index, 02, 03-02, 99 |
| PF-006 | m68k assertion changed to "link exits 0" | AR-27, 01 R6/AC-2, 03-01, 07 ST-11/integration, 99 |
| PF-007 | `stdlib.rs` added to AR-22 and its FFI-binding loop extended | AR-22, 02, 03-01, 07, 99 task 4.1.2 |
| PF-009 | `driver.rs` usage text added to the CLI edit | 03-02, 99 task 3.1.1 |
| PF-010 | `0`-spelling rationale corrected; `BPTR = void *` stated | 03-01, AR-18 note |
| PF-011 | `-lg` self-containment constraint and no-libg-symbol test stated | 03-01, AR-10 note, 07 |

No regression was introduced: `E11213` no longer appears as the file-I/O code, no plan text still claims "zero undefined symbols", and the modification sets now name every file the tasks touch. The plan is ready for execution.
