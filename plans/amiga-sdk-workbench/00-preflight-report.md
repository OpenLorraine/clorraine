# Preflight Report: Amiga SDK Bundle 8 — Workbench and Services

> **Artifact**: `plans/amiga-sdk-workbench/` (full implementation plan set)
> **Audit target**: the eleven plan documents in that folder
> **Audited**: 2026-11-05 22:05
> **Auditor**: independent read-only general agent (fresh context, three iterations)
> **Verdict**: ✅ PASSED WITH NOTES — 20 findings resolved; 0 open

## Audit scope

The audited set is `00-index.md`, `00-ambiguity-register.md`,
`01-requirements.md`, `02-current-state.md`, `03-01-workbench-appicon.md`,
`03-02-icon-library.md`, `03-03-expansion-card-keymap.md`,
`03-04-demo-and-gates.md`, `05-findings.md`, `07-testing-strategy.md`, and
`99-execution-plan.md`.

Context read, not audited: the existing exec/intuition/dos/text modules, the
hosted-Amiga shim, `compiler/src/{profile,stdlib,codegen}.rs`, the A500 smoke
scripts, the registry, and the frozen appendices.

## Checks run

| Check | Result |
|---|---|
| `cd spec && ./check.sh` | `spec check: OK (27 files, 111 compile-fail tests)` |
| `cargo test --test amiga_exec_spec_test` in the SDK container | 14 passed, 0 failed |
| `bash -n docker/scripts/a500-*.sh` | exit 0 |
| `git diff --check` | clean |
| `tools/amiga-sdk/registry.sh check` | OK (79 capabilities, 10 bundles); the planned 008/010 split yields 10 Bundle 8 rows / 80 capabilities |
| 1.3 vs 3.2 NDK headers | 1.3 lacks `clib/wb_protos.h`, `clib/keymap_protos.h`, `clib/cardres_protos.h`, `resources/card.h`; the 3.2 NDK has them |

## Findings and resolutions

### Major (🟠)

| ID | Finding | Resolution |
|---|---|---|
| PF-001 | `CreatePort` does not copy the port name and `openScreen` does not copy its title; temporary `String`s left dangling names. | The demo declares `portName`/`screenTitle` `String`s before the port/screen; the shim copies AppIcon/AppMenuItem titles into their records. |
| PF-002 | The `clib` stubs reference the NDK base globals, which are zero by default. | The 2.0 shim assigns `WorkbenchBase`, `IconBase`, `ExpansionBase`, `KeymapBase`, and `CardResource`; the impl tests pin them. |
| PF-003 | `SDK-WBK-008` bundled nonvolatile and a library-call-only keymap while the plan marked it done. | Split nonvolatile into deferred `SDK-WBK-010`; 008 is expansion/keymap/card with the keymap limitation recorded; 4 done / 6 deferred. |
| PF-004 | The A1200 gate did not prove the services, and the A600 gate did not prove Workbench, so the 2.0 minimum for 001 was unproven. | Both gates run the Workbench and services sequence; the A1200 adds the capture/crop. |
| PF-005 | The AppWindow was created on a custom screen, and only the AppIcon message type was tested. | Scoped the AppWindow proof to lifecycle; the fixture posts icon/window/menu AppMessages and the demo asserts all three types. |
| PF-006 | The `svc` line did not report the board fields or card status. | `svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 0`. |
| PF-007 | A stale HD file could satisfy a poll. | Both gates clear/rebuild the HD and assert start/ready/report/fixture lines. |
| PF-008 | The plan claimed no codegen change while adding `CL_HOSTED_AMIGA_20`. | Disclosed the define and added the emission-integrity auditor to the review. |

### Minor (🟡)

| ID | Finding | Resolution |
|---|---|---|
| PF-009 | Services printed and wrote the same file. | Services prints only; the gate redirects; the Workbench demo writes only. |
| PF-010 | Start/ready handshakes and the hang bound were not asserted. | Assert `startwb 1` and `ready`; document the 60 s gate timeout. |
| PF-011 | The capture golden was not pinned to a position and had no icon-region check. | Pin `(400,140)` with `WBAPPICONA_PropagatePosition`; add a non-blank check and an AppIcon crop golden. |
| PF-012 | The fixture omitted the AppMessage header fields and link line. | Specify `mn_Length`, `NT_MESSAGE`, the reply port, `am_Version`, three types, and `-lamiga -lm`. |
| PF-013 | Phase 1 added `createPublicPort` without scheduling §13.2, so the exec appendix test would fail. | Add §13.2 and the module doc to task 1.1.2; 6.1.1 includes §13.2/§14. |
| PF-014 | 009 depended on deferred 006; the 15-profiles update was not scheduled. | 009 depends on 001; 6.1.1 includes `spec/15-profiles.md`; 6.1.2 updates the dependency. |
| PF-015 | The A600 AppWindow/AppMenuItem and 1 MB budget were claim-only. | Added the A600 `objects205` probe evidence; A600 is report-only; the performance text states the demo's own allocations. |
| PF-016 | The AppIcon crop golden was compared but never captured. | 5.1.4/5.1.6 capture both goldens; AR-5/00-index/07 list the crop. |
| PF-017 | Stale “one AppMessage” text remained. | All documents say three AppMessages (icon/window/menu). |
| PF-018 | IT-13 had no scheduled task and the ranges were stale. | Ranges are IT-1–IT-13; 5.1.5 includes IT-11/IT-13. |
| PF-019 | The gate steps conflated `clorraine build` with the C fixture link and omitted `-lamiga -lm`. | Split the build steps; ST-3/gates include `-lamiga -lm`. |
| PF-020 | The 02 relevant-files table omitted the crop golden. | Added `docs/amiga-sdk/a1200-workbench-icon.png`. |

## Verdict

**✅ PASSED WITH NOTES** — no 🔴 or 🟠 findings remain. The two final 🟡
documentation leftovers (a stale “one AppMessage” sentence and the missing
crop-golden row in 02) were fixed after iteration 3. The plan is grounded in
the real KS2.05/KS3.1 NDKs and probes, the scope deferrals are evidence-backed,
and the test order is specification-first. Execution may proceed per AR-18.
