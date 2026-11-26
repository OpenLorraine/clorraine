<!-- title: [bug] Attach, render, and free every GadTools gadget -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
Every GadTools window shows one gadget fewer than the program created, and leaks it. The A600 gate counts capture colours only, so it does not catch this.

| Field | Value |
|---|---|
| Finding | ASM-WS5-012 · DEF-598 |
| Severity / Priority / Rank | Major / P1 / 17 of 58 |
| Domain / Lens | shims / A1200 shipped |
| Confidence / Verdict | probable / confirmed (Major; evidence now verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading. `CreateContext` puts a context gadget at the head of `glist`, so the chain has `count + 1` gadgets; `AddGList`, `RefreshGList`, and `RemoveGList` are all passed `count`, which covers the context gadget and the first `count − 1` user gadgets. `AddGList` relinks the last added gadget's `NextGadget`, cutting the last user gadget out of the chain that `FreeGadgets` later walks. Intuition `AddGList` "Returns the position of where the first gadget in the list was actually added", so `== 0` is a valid position, not a failure; if it fires, the attached list is freed.

## Root cause
off-by-one on the context gadget and a wrong return-value test.

Location: `compiler/runtime/shim_hosted_amiga_optional_ui.c:236-252` (`cl_gt_build`), `:267` (`cl_gt_free`), `:562` (same `AddGList == 0` test), at e24a8752

## Recommended fix
pass `-1` (all) or `count + 1`; drop the `== 0` failure test; guard against rebuilding after a partial failure.

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
