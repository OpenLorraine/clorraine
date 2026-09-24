# Phase 5 Review: Clorraine Demo Video Toolkit

> **Document**: 08-phase-5-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: correctness-reviewer + security-auditor · scope mode: strict
> **Baseline tree**: 83162083a60a7a87a6cac3b60995258067b0af04
> **Reviewed revision**: d9eda6d
> **Verification evidence**: `make demo-check` (67 tests) + `DEMO_E2E=1` render smoke
> **Date**: 2026-09-25 01:30

## Summary

Independent correctness and security review of the Phase 5 diff (the `new` scaffold,
the `README`/`AGENTS.md` updates, and the tests). No critical or major findings. One
minor bug (CRLF title) and one low hardening item (path-free write errors) were fixed;
two informational notes were accepted. The security audit confirmed the input validation
that AC-8 asks for.

## Findings

| # | Severity | Finding | Location | Resolution |
|---|----------|---------|----------|------------|
| RV-501 | 🟡 Minor | A CRLF first line lost the title (the trailing `\r` defeated the regex), silently falling back to the stem | `scaffold.ts` `deriveTitle` | Strip a trailing `\r`; locked by a CRLF impl test |
| RV-502 | 🔵 Info | Only the final period is stripped, so `// Foo...` yields `Foo..` | `scaffold.ts` `deriveTitle` | Accepted: matches the singular "a trailing period" wording |
| RV-503 | 🔵 Info | Two sources with the same stem share `demo/<stem>.demo.ts`; the overwrite message hides the collision | `scaffold.ts` | Accepted (path is mandated by 03-04); limitation documented in `tools/demo/README.md` |
| RV-504 | 🟡 Minor | Non-`EEXIST` `mkdir`/`writeFile` errors were rethrown raw, so the CLI printed a Node message with the absolute host path | `scaffold.ts` | Wrap into `ConfigError` with only the relative path and the errno code |
| RV-505 | 🔵 Info | TOCTOU between `resolveRepoPath` and a forced write if a repo directory is swapped for a symlink | `scaffold.ts`, `paths.ts` | Accepted: the default write uses `wx` (`O_EXCL`, no symlink follow) and the model is a local trusted operator |

## Security audit (AC-8)

**No critical or major findings.** Path containment holds: absolute paths are rejected,
`../` and symlink escapes are caught before any read or write, and the output path is
derived with `basename` so the stem cannot traverse. The `wx` overwrite guard refuses an
existing target and symlinks; `deriveTitle`'s regex is linear (no ReDoS); README/AGENTS
introduce no secret or host path. RV-504 was the only fix; RV-505 needs no action.

## Fix verification

- `make demo-check` — tsc clean, 67 tests (1 skipped).
- `DEMO_E2E=1 node --test test/render.smoke.test.ts` — 1 test passing.
