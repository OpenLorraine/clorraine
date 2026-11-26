<!-- title: [bug] Follow the AmigaOS protocols in the remaining shim wrappers -->
<!-- labels: assessment,kind:theme,severity:major,priority:P1,domain:shims,amiga-sdk -->
<!-- parent: - -->

## Goal
Follow the AmigaOS protocols the remaining shim wrappers skip

| Field | Value |
|---|---|
| Roadmap entry | R9 `amiga-shim-protocol-fixes` (report §8) |
| Order | 9 of 14 |
| Depends on | R5 |
| Effort | L |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS5-007 (Major, DEF-599) — `cardPresent()` and `cardStatus()` remove `card.resource` from the system — finding issue (sub-issue)
- ASM-WS5-008 (Major, DEF-597) — The screen capture misreads the 2.0+ colour table, and the A1200 Workbench golden pins the wrong colours — finding issue (sub-issue)
- ASM-WS5-009 (Major, DEF-600) — The interrupt servers return with the user handler's condition codes and can end the VERTB chain — finding issue (sub-issue)
- ASM-WS5-010 (Major, DEF-607) — The busy-pointer image has no sprite header or terminator, so sprite DMA reads past the 64-byte allocation — finding issue (sub-issue)
- ASM-WS5-012 (Major, DEF-598) — GadTools: the last gadget is never attached, rendered, or freed — finding issue (sub-issue)
- ASM-WS5-017 (Major, DEF-605) — Layer drawing returns without `WaitBlit`, so CPU access to the same bitmap races the blitter — finding issue (sub-issue)
- ASM-WS5-018 (Major, DEF-606) — Classic menus are never laid out: every menu header and item sits at the same position — finding issue (sub-issue)
- ASM-WS5-023 (Major, DEF-595) — `clipboardWrite` reports success for raw data that the 3.1 clipboard device discards, so `clipboardRead` never sees it — finding issue (sub-issue)
- ASM-WS5-003 (Minor, no DEF) — `setCurrentDir` unlocks the launcher's current-directory lock and never restores it — listed in a tracking issue
- ASM-WS5-011 (Minor, no DEF) — `FileInfoBlock`s on the stack are not guaranteed longword-aligned — listed in a tracking issue
- ASM-WS5-014 (Minor, no DEF) — Device buffers are not in chip memory on the 1.3 profile, which pre-V36 trackdisk requires — listed in a tracking issue
- ASM-WS5-015 (Minor, no DEF) — Commodity brokers are never activated — listed in a tracking issue
- ASM-WS5-019 (Minor, no DEF) — Lower-impact shim defects (grouped) — listed in a tracking issue
- ASM-WS5-021 (Minor, no DEF) — Host and freestanding shim edges: NULL `memcpy` with zero length, negative lengths, no `memmove` — listed in a tracking issue

## How to start
Create the plan with `make-plan amiga-shim-protocol-fixes`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
