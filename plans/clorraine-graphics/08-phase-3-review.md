# Phase 3 Review: Clorraine Amiga Graphics

> **Document**: 08-phase-3-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: phase-reviewer (correctness / documentation accuracy) · scope mode: strict
> **Baseline tree**: abf26acd21205f14112b6f7e6edc8d27522c2296
> **Reviewed revision**: e7776f6
> **Verification evidence**: `spec/check.sh` green (27 files, 102 compile-fail tests); `make check` 15/15
> **Date**: 2026-09-24 19:17
> **Auditor skip (logged)**: docs-only diff — security and performance auditors skipped

## Summary

Independent review of the Phase 3 documentation diff (4 files: `appendix-amiga.md`,
`appendix-stdlib.md`, `16-borrow-rules.md`, `AGENTS.md`). The documented graphics
surface matches the module sources, the shim, and the export tables (signatures,
`borrow` modes, return values, `depth`, the null-safe drop, and the `title`
lifetime). Section numbering in `appendix-stdlib.md` is sequential (8→13).
AGENTS.md is accurate. One major and two minor findings.

## Findings

| # | Severity | Finding | Location | Evidence | Recommended fix |
|---|----------|---------|----------|----------|-----------------|
| RV3-001 | 🟠 Major | The renumbering update is incomplete: several corpus READMEs still cite `appendix §9`, which now resolves to Drawing, not the String/`str` helper table (moved §9→§11). | `spec/tests/corpus/string-parser/README.md:59`; `spec/tests/corpus/file-copier/README.md:26,27,28,68`; `spec/tests/corpus/graphics/README.md:27` | e.g. file-copier README "`bytes(...)` ... appendix §9"; graphics README "helper table appendix §9". `spec/check.sh` does not link-check corpus READMEs, so it stayed green. | Update the six `§9` references to `§11` |
| RV3-002 | 🟡 Minor | The new §9 table writes the pen range as `0..(1 << depth) - 1`, which reads as exclusive under `..`; for depth 4 that reads 0–14, but valid pens are 0–15. | `spec/appendix-stdlib.md:215` | The module text clarifies "accepts 0-15"; the appendix does not. | Write `0..(1 << depth)` or `0..=(1 << depth) - 1` |
| RV3-003 | 🟡 Minor (pre-existing) | `appendix §7` is cited for `delayTicks`, but neither appendix §7 documents it (amiga §7 = ROM/runner; stdlib §7 = interrupt dispatcher). Not phase-caused. | `spec/tests/corpus/graphics/README.md:34`; `spec/tests/corpus/interrupt-counter/README.md:30` | Sections ≤7 did not move. | Retarget to `appendix-amiga.md` §1 or drop the citation |

## Rulings

The user approved fixing RV3-001, RV3-002, and RV3-003; all were applied and
verified (`spec/check.sh` green; `make check` 15/15).

| # | Ruling | Action |
|---|--------|--------|
| RV3-001 | Fixed | The six `appendix §9` citations in `string-parser`, `file-copier`, and `graphics` corpus READMEs now read `appendix §11`. The `appendix §7` citations that point to the interrupt dispatcher are correct and were left unchanged. |
| RV3-002 | Fixed | The §9 table now writes the pen range as `0..(1 << depth)`. |
| RV3-003 | Fixed | The stale `appendix §7` clause was dropped from the `delayTicks` rows in the graphics and interrupt-counter READMEs; both keep the `X6 (Ch 12 §1)` citation. |
