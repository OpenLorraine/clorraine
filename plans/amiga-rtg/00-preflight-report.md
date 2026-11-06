# Preflight Report: Amiga RTG Execution Plan

> **Audit target**: `plans/amiga-rtg/99-execution-plan.md` with the requirement set in the same folder
> **Artifact hash**: `5307c32369a7a564bd39766d13b765b27328f1ccbfd56b01cfd072f1b428a5f4` (plan + affected requirement docs; second pass)
> **Date**: 2026-11-06
> **Reviewer**: independent plan preflight (subagent) plus lead reconciliation
> **Verdict**: ✅ PASSED — all findings resolved

⚠️ **Same-session review:** the plan was authored in this session and reviewed
independently. Findings were checked against the actual compiler, tests,
Makefile, registry, gate scripts, and P96 headers before resolution.

## Codebase Context

| Area | Evidence |
|---|---|
| Spec-test harness | `compiler/tests/amiga_optional_spec_test.rs:73-110` (strict cross-build), `:192-200` (appendix section) |
| Impl pin harness | `compiler/tests/amiga_optional_impl_test.rs:38-100` |
| Stdlib registration | `compiler/src/stdlib.rs:35` (`SOURCES`), `:239` (`MODULES`), `:1446-1468` (FFI coverage test) |
| Profile gate | `compiler/src/profile.rs:91-123`; `compiler/src/driver.rs:258-273` (`E11200`) |
| Suffix placement | `compiler/src/profile.rs:18-24`, `:140-144`; `compiler/src/codegen.rs:644` |
| Generated C output | `compiler/src/clorraine/artifacts.rs`; no include path from the repo runtime directory |
| Verify list | `Makefile:117-137` |
| Registry schema | `tools/amiga-sdk/registry.sh:17-105`; 14 columns, `10-rtg`, `SDK-RTG-001` valid |
| Gate helpers | `docker/scripts/workbench-media.sh`; `docker/scripts/a1200-addons-smoke.sh:100-220` |
| Corpus | `docker/scripts/corpus-amiga.sh:30-112` |
| P96 API | `build/rtg/p96dev/.../inline/Picasso96.h`, `pragmas/Picasso96_pragmas.h`, `fd/Picasso96API_lib.fd` |
| P96 evidence | `build/rtg/loop-evidence.txt`, `build/rtg/rect-fill-evidence.txt`, `build/rtg/probe/rtgprobe.c` |
| Media | `/home/gevik/amiga/picasso96/Picasso96.lha` SHA-256 `0146d4a2…`; WB3.1 ADF; A1200 ROM SHA-1 `e2154572…` |

## Findings and Resolutions

| ID | Severity | Finding | Resolution |
|---|---|---|---|
| PF-PLAN-001 | MAJOR | The vendored header had no reachable include path because the shim is concatenated into generated C. | The header is embedded directly before the shim in `HOSTED_AMIGA_SUFFIX` with `include_str!`; the shim has no `#include "picasso96.h"`. `03-02` and task 2.1.3 state this. |
| PF-PLAN-002 | MAJOR | The spec suite was asked to go green before the shim and appendix existed. | Task 1.1.2 now runs ST-1/2/3/7 only; ST-4 runs in 2.1.3; ST-6 runs after 4.1.2. The suite is split, not weakened. |
| PF-PLAN-003 | MAJOR | Fixed-arg `LP*` macros are not vbcc-safe (vbcc rejects `p96CloseScreen`). | The header now branches: GCC uses `LP*` macros; vbcc uses plain prototypes and the SDK `amiga.lib` P96 stubs. Task 3.1.4 compiles the emitted C with both toolchains; the runtime gate is GCC. |
| PF-PLAN-004 | MAJOR | The gate doc blitted at (200,200) while the PPM covered (0,0)-(63,63). | `03-03` now writes at (200,200) and blits at (210,210); the PPM checks relative yellow (10,10), blue (11,11), and blue (20,20). |
| PF-PLAN-005 | MAJOR | Several accessors and the missing-library guard had no behavioral test. | The demo now calls every screen/bitmap/mode accessor, fills the screen away from the capture, and reads a 32x32 bitmap region. The impl suite pins `P96Base == NULL`. |
| PF-PLAN-006 | MINOR | No task flipped the registry row to `done`. | Task 4.1.1 sets `SDK-RTG-001` to `done` after the gate is green. |
| PF-PLAN-007 | MINOR | The stdlib task named a nonexistent `P96` profile flag and omitted `SOURCES`/`MODULES`. | Task 1.1.2 names both tables, states that `hosted-amiga` already accepts every stdlib module, and leaves the suffix to 2.1.3. |
| PF-PLAN-008 | MINOR | Header said 0/24 tasks but the checklist had 19. | Header is 0/19; the final is 19/19. |
| PF-PLAN-009 | MINOR | The bitmap-fill claim lacked a negative control. | `build/rtg/rect-fill-evidence.txt` records `p96RectFill` returning red for a blue fill; `build/rtg/loop-evidence.txt` records the pixel-loop result. IT-8 pins the loop. |
| PF-PLAN-010 | MINOR | The gate's media path did not contain the archive. | The archive is installed at `/home/gevik/amiga/picasso96/Picasso96.lha` with the approved hash; `03-03` also accepts `P96_LHA`. |
| PF-PLAN-011 | MINOR | The requirements preflight hash/verdicts were stale after the first pass. | Requirements preflight was re-run and updated to second-pass hash `3c9547d0…`; the four stale contradictions are recorded as resolved. |

## Requirement Coverage

| Req | Tasks | Acceptance evidence |
|---|---|---|
| R1 | 1.1.1, 1.1.2, 2.1.3 | `E11200` spec; suffix placement; module registration |
| R2 | 2.1.2, 2.1.4 | `rtgBoards` gate + missing-library pin |
| R3 | 2.1.2, 2.1.4, 3.1.1 | Mode query/accessors in demo + impl |
| R4 | 2.1.2, 2.1.4, 3.1.1 | Screen handle/accessors/drop + `rtgScreenMode` |
| R5 | 2.1.2, 2.1.4, 3.1.1 | Bitmap alloc/accessors/free + bytes-per-row |
| R6 | 2.1.2, 2.1.4, 3.1.1 | Screen/bitmap pixel and fill checks; bounds |
| R7 | 2.1.2, 2.1.4, 3.1.1, 3.1.2 | Blit + screen and bitmap RGB readback + PPM |
| R8 | 3.1.1-3.1.3 | `make sdk-a1200-rtg` report and PPM |
| R9 | 4.1.1 | Full registry row/done, coverage, issue #11 |
| R10 | 4.1.2 | Appendix, limitations, AGENTS, notices, roadmap |
| R11 | 3.1.4 | GCC and vbcc RTG build-only corpus leg |
| R12 | 2.1.4, 5.1.3 | Bounds pins + reviews |
| R13 | 1.1.1, 2.1.4, 3.1.2 | Export/pin/PPM checks |

## Pass Tier

**✅ PASSED** — every critical/major finding is resolved. The two remaining
minor findings (attribution and registry transition) are explicitly covered by
R10/R9 and task 4.1.1/4.1.2. Plan execution may proceed.

## Residual Risks

| Risk | Mitigation |
|---|---|
| vbcc pragma/prototype behavior | Task 2.1.2/3.1.4 compiles an emitted RTG unit with `vc`; failure is a plan failure, not a silent skip. |
| P96 monitor icon tooltypes | The monitor is copied as `uaegfx`; the monitor guesses its board type from its name when the icon is absent. The spike proved it. |
| P96 bitmap fill quirk | The proven pixel loop is the only bitmap fill path; the negative control is retained. |
| External media | The archive is outside the repo and checked by SHA-256; the gate exits 2 with instructions when absent. |
