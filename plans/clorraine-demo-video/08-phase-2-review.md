# Phase 2 Review: Clorraine Demo Video Toolkit

> **Document**: 08-phase-2-review.md
> **Parent**: [Index](00-index.md)
> **Dispatch**: phase-reviewer (correctness / maintainability / standards) · scope mode: strict
> **Baseline tree**: 0fbbb6ef913f1c62b0af3fb1194e878c9e6ac355
> **Reviewed revision**: eb6f2e3
> **Verification evidence**: `make demo-check` — `tsc --noEmit` clean, 32/32 tests
> **Date**: 2026-09-24 22:24

## Summary

Independent review of the Phase 2 diff (Clorraine grammar, Shiki viewer, Chrome capture,
ffmpeg encoder, card/code/poster renderers). Two MAJOR and several minor findings; all
fixed except RV-210, which is accepted (the compose normalize step pads every clip).

## Findings

| # | Severity | Finding | Location | Resolution |
|---|----------|---------|----------|------------|
| RV-201 | 🟠 Major | `String.replace` treated `$` in the replacement as special, corrupting pages whose source contains `$&`/`$1` | `src/renderer/code.ts` | Use function replacements |
| RV-202 | 🟠 Major | Char literals were unscoped and ST-3 did not check them | `grammars/`, `test/grammar.spec.test.ts` | Add a `constant.character.clorraine` rule + fixture assertions |
| RV-203 | 🟡 Minor | `module demo.Hello;` name was not scoped | `grammars/` | Add the module-name rule + assertion |
| RV-204 | 🟡 Minor | Interpolation was scoped in every string | `grammars/` | Template `$"` begin; plain strings stay literal |
| RV-205 | 🟡 Minor | `as unknown as` double cast | `src/util/shiki.ts` | Use the `ThemedToken.explanation` type directly |
| RV-206 | 🟡 Minor | No spawn `error` handler; Chrome stderr discarded | `src/util/chrome.ts` | Race the spawn error; append the stderr tail |
| RV-207 | 🟡 Minor | Scroll math ignored viewport padding | `assets/code.html` | Subtract padding |
| RV-208 | 🟡 Minor | Imports at file end | `test/scenes.spec.test.ts` | Moved to the top |
| RV-209 | 🟡 Minor | Ready-timeout error omitted the page URL | `src/util/chrome.ts` | Include the URL |
| RV-210 | 🔵 Observation | Frame-dimension assertion not implemented | `src/util/chrome.ts` | Accepted: the Phase 3 compose normalize step pads every clip to the canvas |

## Fix verification

After the fixes: `make demo-check` — `tsc --noEmit` clean, 32/32 tests. The 🔴/🟠 fixes
(RV-201, RV-202) were re-reviewed on the fix diff; see the re-review note below.
