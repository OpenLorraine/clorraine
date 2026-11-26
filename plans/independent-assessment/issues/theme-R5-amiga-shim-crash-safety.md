<!-- title: [bug] Fix the Amiga shim paths that crash or hang a real machine -->
<!-- labels: assessment,kind:theme,severity:critical,priority:P0,domain:shims,amiga-sdk -->
<!-- parent: - -->

## Goal
Fix the shim paths that crash or hang a real machine: open Intuition on demand, clip screen text, finish the clipboard read protocol, track borrowed clip regions and guide messages

| Field | Value |
|---|---|
| Roadmap entry | R5 `amiga-shim-crash-safety` (report §8) |
| Order | 5 of 14 |
| Depends on | R1 (shares the registry) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS5-002 (Critical, DEF-587) — `savePublicScreen` and `showAlert` call Intuition through a NULL `IntuitionBase` and crash the machine — finding issue (sub-issue)
- ASM-WS5-004 (Critical, DEF-585) — `drawText` writes unclipped text through a screen `RastPort` that has no layer — finding issue (sub-issue)
- ASM-WS5-016 (Critical, DEF-588) — The clipboard read never finishes the read protocol — finding issue (sub-issue)
- ASM-WS5-006 (Critical, DEF-593) — Safe handles let a borrowed OS object outlive its owner: clip regions and AmigaGuide messages — finding issue (sub-issue)

## How to start
Create the plan with `make-plan amiga-shim-crash-safety`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
