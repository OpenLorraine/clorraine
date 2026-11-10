# Requirements Preflight Report: Build-Flag and Binary-Size Closeout (PLAN-26)

> **Document**: 00-preflight-report-requirements.md
> **Parent**: [Index](00-index.md)
> **Audit target**: [`01-requirements.md`](01-requirements.md)
> **Context documents**: [`00-ambiguity-register.md`](00-ambiguity-register.md),
> [`00-index.md`](00-index.md), `plans/00-roadmap.md`
> **Scope mode**: strict
> **Audit target hashes**:
> - scan 1 draft (before the re-scan fixes): `d9fd2147eb994b4c8a22623c292c8f5b61b9c350c3c1628b5766ad26794d2676`
> - final: `4c7e317617b5c79162c0d13e151c8621af9675e5d044d7d75a1456a1dc9eb111`
> **Verdict**: ✅ **PASSED** — every critical/major finding is resolved; every
> minor finding was fixed in the document rather than accepted.

> ⚠️ **Same-session review.** The requirements were authored in this session.
> The first scan and the re-scan were dispatched to independent read-only
> auditors with the full codebase packet. The user made the one reserved
> decision (the frozen-spec update); the remaining decisions are delegated
> technical records under auto-design policy version 1.

## Codebase Context Summary

| Fact | Evidence |
|---|---|
| The driver owns the exact compile-matrix commands | `compiler/src/clorraine/toolchain.rs:1-10,36-54,144-157` |
| The spec pins the two GCC release rows | `spec/17-codegen.md:384-396` |
| The harness hard-codes its own release flags | `tools/perf/bench.py:36-41`; flags recorded at `:493-498`; compare at `:625-630` |
| The shape gate uses warning-bearing GCC lines | `docker/scripts/bench-shape.sh:27-30` |
| The base worktree builds the base `clc` | `docker/scripts/bench-base.sh:1-60` |
| `v1.1.1` predates the constant-array feature | tag `2d1eb30`; `cef0b91` is not an ancestor |
| `f9d5be8` is the current post-PLAN-25 head | `git log v1.1.1..HEAD`; `build/perf/head.json` records `clc 1.1.1` |
| The freestanding host-run tool is separate from the m68k tool | `toolchain.rs:122-136`; `compiler/src/clorraine/mod.rs:338` |
| The removed constructors are `__amigapath.o`/`environ.o` | `-Wl,--print-gc-sections`; `nm`; probe traces |
| The 1.3 portable corpus set is `file-copier`, `string-parser`, `buffer-container`, `volatile-mmio` | `docker/scripts/corpus-amiga.sh:36`; `interrupt-counter` is FS-UAE-only |
| The probe measurements reproduce | `build/perf/probe/*`; `m68k-amigaos-size`; `vamos --instr-trace`; two identical links/traces |

## Scan 1 — Soundness, Grounding, Delivery, and Risk

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-001 | 🔴 CRITICAL | The frozen spec §13 pins the old release lines; R2.2/N6 contradicted it. | User ruling 2026-11-10: update the two GCC release rows. Added AR-19, R2.9, the scope entry, and the N6 exception. |
| PF-002 | 🟠 MAJOR | AR-08 still said `v1.1.1`; `v1.1.1` predates PLAN-25 and would double-count it. | AR-08 now uses `f9d5be8`; R4.5 asserts the full hash and `dirty == false`. |
| PF-003 | 🟠 MAJOR | `make verify` did not exercise release+GC for Workbench or the 1.3 corpus. | Added AR-20 and R4.6/R4.7: keep the warning-bearing debug Workbench build, add a release build/leg, and run the four 1.3 programs under `vamos`. |
| PF-004 | 🟠 MAJOR | The constructor evidence said "empty walk"; the list is not empty and the remaining entries still run. | Rewrote the overview, AR-09, and AR-13: GC removes the `__amigapath.o`/`environ.o` objects and their `___initamigapath`/`___fillenviron` entries; the 444-instruction delta is those bodies. |
| PF-005 | 🟠 MAJOR | The non-Linux fallback test could be dead `cfg` code on the Linux gate. | AR-06/R2.5/R5.3 require a pure platform/linker predicate with both outcomes exercised on Linux. |
| PF-006 | 🟠 MAJOR | The freestanding **host-run** release command was not pinned; a global flag change would alter it. | R2.3/R2.6/R5.1 pin both `native_syscalls` branches at `-O2` with no GC/strip. |
| PF-007 | 🟠 MAJOR | Review dispatch was contradictory (performance auditor vs design challenge). | AR-16, the index, R7.2, and the acceptance summary now consistently use correctness + emission integrity + design challenge. |
| PF-008 | 🟡 MINOR | Matrix base numbers were wrong (77,604/100,196 vs 76,676/100,276). | Corrected the table and AR-04; delta is −69.2%/−74.6%. |
| PF-009 | 🟡 MINOR | Freestanding strip-only was stated as 268; it is 244 (GC+strip is 268). | Corrected in the requirements and AR-07. |
| PF-010 | 🟡 MINOR | Strip ranges were wrong (10–11%/18–22%). | Corrected to 7–27% m68k and 6–12% host after GC, with the per-program values. |
| PF-011 | 🟡 MINOR | Calibration was treated as scored and the ranges included it. | AR-10 separates the five scored programs from calibration; the index states the same. |
| PF-012 | 🟡 MINOR | "1.3 corpus programs" was undefined. | Named the portable set in R1.2/R3.4/AR-04/AR-20. |
| PF-013 | 🟡 MINOR | `m68k_release_bytes` was required for calibration, whose record is smaller. | R4.2 scopes it to scored benchmarks. |
| PF-014 | 🟡 MINOR | The stripped-release limitation was unscoped and missed the production-readiness register. | R6.2 scopes it to Linux/Amiga release builds and names `plans/v1-production-readiness/00-ambiguity-register.md`. |
| PF-015 | 🟡 MINOR | LTO/LTO+GC issue count conflicted with the "exactly four" requirement. | One LTO issue; R6.3, AR-14, AR-18, and the out-of-scope table agree. |
| PF-016 | 🟡 MINOR | PLAN-26 had no roadmap row. | Added at `RD Drafted`; progress is 569/572, 1 open, 2 parked. |
| PF-017 | 🟡 MINOR | The shape gate could lose its `-Wall -Wextra -Werror` line. | R4.6 adds the shipped flags to the warning-bearing line; nix13 is compile-only. |
| PF-018 | 🟡 MINOR | The floors were not automated. | R1.6/R5.6 require a committed script/make target that exits nonzero on a missed floor. |
| PF-019 | 🔵 OBS | The probe trail was incomplete and the AR-03/AR-09 wording overstated. | AR-03 says "no compiler source change"; AR-09 names the constructor bodies; the plan re-runs the audit. |
| PF-020 | 🔵 OBS | Three flag-list copies had no cross-check. | AR-22/R2.6/R5.4 add a driver/harness cross-check; the audit uses the same constants. |

## Re-scan — Fix Verification and Regressions

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| RS-001 | 🟠 MAJOR | `interrupt-counter` is FS-UAE-only, not a portable `vamos` 1.3 program. | Replaced with `volatile-mmio` everywhere; measured 53,512 → 5,944 `.text` (−88.9%), output `volatile-mmio: ok`. |
| RS-004 | 🟠 MAJOR | The PLAN-26 roadmap row was missing. | Added at `RD Drafted`; counter updated. |
| RS-002 | 🟡 MINOR | The requirements still said freestanding strip-only = 268. | Corrected to 244 / GC+strip 268. |
| RS-003 | 🟡 MINOR | The scored `.text` maximum was 69.3%, not 69.6%. | Corrected in AR-10 and the index. |
| RS-005 | 🟡 MINOR | A copy-paste block cannot be the gate. | R1.6 requires a committed script/make target; the block is documentation. |
| RS-006 | 🟡 MINOR | R3.5 did not name the comparator. | It now says "the same `clc` under the old flags (the flag audit)". |
| RS-007 | 🟡 MINOR | The `--release` help text was broader than the flags. | R6.1a scopes it to Linux/Amiga; freestanding and non-Linux keep symbols. |
| RS-008 | 🟡 MINOR | The production-readiness register path was unnamed. | R6.2 names `plans/v1-production-readiness/00-ambiguity-register.md`. |
| RS-009 | 🟡 MINOR | Two `-Os` rows read as two issues. | The table and R6.3 now state one deferred issue. |
| RS-010 | 🟡 MINOR | Replacing the debug Workbench build would lose the strict-warning compile. | AR-20/R4.6 keep the debug build and add a release leg. |
| RS-011 | 🟡 MINOR | "Same output directory per variant" was self-contradictory. | R1.3/AR-09 use one fixed path per program, rebuilt in place. |
| RS-012 | 🟡 MINOR | The 1.3 corpus was not in the repeat set. | R1.5 adds base/GC on the four named programs. |
| RS-013 | 🟡 MINOR | The audit's new-flag array was a fourth copy. | R1.6/R5.4/AR-22 source it from the harness constants or a shared manifest. |

## User Ruling

- **Decision**: update the two GCC release rows in `spec/17-codegen.md` §13.
- **Date**: 2026-11-10.
- **Record**: AR-19; R2.9; scope boundary; N6 exception.
- **Why**: the compile matrix is the exact acceptance test; a separate
  shipped-release section would create two definitions that can drift, and
  keeping the old rows would make the spec false.

## Final Verification

- Every 🔴/🟠 finding is resolved in the requirements, register, and index.
- Every 🟡/🔵 finding is fixed, not accepted.
- The reserved spec decision has an explicit user ruling; all other
  decisions are delegated technical records under policy version 1.
- The 13 audit dimensions were re-scanned over the final target; no new
  critical/major issue remains.
- The requirements are internally consistent with the codebase evidence and
  the roadmap row.

**Verdict: ✅ PREFLIGHT PASSED — all 33 findings resolved (1 critical, 8 major, 24 minor/observation), 0 accepted, 0 open.**
