<!-- title: [bug] Lay out classic menu headers and items -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
A program with two or more menus, or two or more items in a menu, shows overlapping entries; only the first is reachable. The GUI gate drives menus with Amiga-key shortcuts, which do not depend on layout.

| Field | Value |
|---|---|
| Finding | ASM-WS5-018 · DEF-606 |
| Severity / Priority / Rank | Major / P1 / 25 of 58 |
| Domain / Lens | shims / A500, A1200 shipped |
| Confidence / Verdict | probable / confirmed (Major, probable) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading. `LeftEdge` is assigned from `index` before the loop that counts the existing menus, so every header gets x = 0; items are allocated `MEMF_CLEAR` and never given a `TopEdge`, so every item overlaps item 0. Classic `SetMenuStrip` does not lay out menus; the application must (RKM Intuition, Menus).

## Root cause
statement order and a missing item layout.

Location: `compiler/runtime/shim_hosted_amiga.c:2372`, `:2397` (`menu->LeftEdge = (WORD)(index * CL_MENU_WIDTH)` while `index` is still 0), `:2403-2413` (index computed afterwards), `:2474-2486` (items never set `TopEdge`), at e24a8752

## Recommended fix
compute the index first; set `TopEdge = index * CL_MENU_HEIGHT` per item.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
—. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
