# Phase 1 Review: Clorraine Demo Video Toolkit

> **Document**: 08-phase-1-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: phase-reviewer (correctness / maintainability / standards) · scope mode: strict
> **Baseline tree**: 10048f620a01949c2afc4565ef6b73db9ca1bb12
> **Reviewed revision**: ba55991
> **Verification evidence**: `make demo-check` — `tsc --noEmit` clean, 14/14 tests
> **Date**: 2026-09-24 22:03

## Summary

Independent review of the Phase 1 diff (tool foundation). Two MAJOR findings and five
minor findings, all fixed; three observations tidied. One observation (RV-109, optional
scalar validation) is accepted without change.

## Findings

| # | Severity | Finding | Location | Resolution |
|---|----------|---------|----------|------------|
| RV-101 | 🟠 Major | The root override accepted only `--root`, not the documented `CLORRAINE_DEMO_ROOT` | `src/config.ts` | Added the env fallback |
| RV-102 | 🟠 Major | The CLI entry guard compared paths without canonicalizing, so a symlinked invocation (e.g. `npx`) silently no-opped | `src/cli.ts` | Compare against `realpathSync` |
| RV-103 | 🟡 Minor | `poster` was not type-checked and a numeric value threw a raw `TypeError` | `src/scenes.ts` | Validate `poster` as a non-empty string |
| RV-104 | 🟡 Minor | `stdout`/`stderr` echo was gated inconsistently | `src/util/proc.ts` | Gate both on the `stdio` option |
| RV-105 | 🟡 Minor | `commandExists` interpolated into `sh -c` | `src/util/proc.ts` | Use `which` with an argument array |
| RV-106 | 🟡 Minor | Config/path errors exited 1 instead of 2 | `src/cli.ts` | Map `ConfigError`/`PathError` to exit 2 |
| RV-107 | 🟡 Minor | The validation test did not assert the scene id | `test/scenes.spec.test.ts` | Assert the id; add gif/poster cases |
| RV-108 | 🟡 Minor | `proc.ts`/`cli.ts` had no tests; loader error paths uncovered | `test/` | Add `proc.impl.test.ts` and loader cases |
| RV-110 | 🔵 Observation | Redundant cast; `kind` compared via `as` | `src/scenes.ts` | `Set` lookup + type guard |
| RV-111 | 🔵 Observation | Target `es2023` drifted from the spec's `es2022` | `tsconfig.json` | Set `es2022` |
| RV-112 | 🔵 Observation | `--only` usage wording vs comma-separated spec | `src/cli.ts` | Usage lists comma-separated |
| RV-109 | 🔵 Observation | Optional scalars (`caption`, `subtitle`, …) are not fully validated | `src/scenes.ts` | Accepted: the type guard is documented as structural; full optional validation is deferred until a renderer needs it |

## Fix verification

After the fixes: `make demo-check` — `tsc --noEmit` clean, 21/21 tests. The 🔴/🟠 fixes
(RV-101, RV-102) were re-reviewed on the fix diff; see the re-review note below.

## Re-review (fix diff, commit 9b0d1b8)

The scoped re-review confirmed RV-101–RV-108 and found one regression the RV-102 fix
introduced:

| # | Severity | Finding | Resolution |
|---|----------|---------|------------|
| RV-113 | 🟠 Major | The RV-102 `try/catch` wrapped `await main(...)`, so a thrown usage error was swallowed and the CLI exited 0 | Narrowed the try to the `realpathSync`/URL comparison; `parseArgs` errors now print usage and exit 2 |
| RV-114 | 🟡 Minor | The env-root fallback was untested | Added a `CLORRAINE_DEMO_ROOT` test |
| RV-115 | 🟡 Minor | The CLI entry guard was untested | Added `test/cli.impl.test.ts` (help, unknown option, missing config) |

After the re-review fixes: `make demo-check` — `tsc --noEmit` clean, 25/25 tests. Per the
quality profile, no third review pass was run.

