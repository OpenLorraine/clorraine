<!-- title: [bug] Close the library bases the runtime opens -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R1 -->

## Summary
Each run leaves an open count on graphics, intuition, layers, and diskfont (and the 2.0+ libraries a program used): disk-loaded `diskfont.library` and the optional libraries can never be expunged, so memory is lost until reboot; for `bsdsocket.library` the per-task base is never closed, so TCP/IP stacks keep the exited task's sockets and signal bindings.

| Field | Value |
|---|---|
| Finding | ASM-WS5-013 · DEF-604 |
| Severity / Priority / Rank | Major / P1 / 23 of 58 |
| Domain / Lens | shims / A500, A1200 shipped |
| Confidence / Verdict | probable / confirmed (Major, probable) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Code reading plus measurement: `grep -n CloseLibrary compiler/runtime` finds only the user wrapper (`shim_hosted_amiga.c:3254`); the link map shows the initialized bases are the program's own symbols (`IntuitionBase` in `.bss`, only `libstubs.a(dos.o)` linked). Exec `OpenLibrary` requires a matching `CloseLibrary`. For the uninitialized (common) bases the picture is mixed: probe 510 uses `UtilityBase` and is leak-free under `vamos`, so libnix manages that one; the others were not probed.

## Root cause
lazy opens with no exit hook.

Location: `compiler/runtime/shim_hosted_amiga.c:1294-1297` (`GfxBase`, `IntuitionBase`, `DiskfontBase`, `LayersBase`, initialized definitions) and their lazy opens (`:1307`, `:1311`, `:1600-1604`, `:3682`, `:5742`, `:5859`); the optional suffixes' bases (`optional.c:848`, `:1062`; `optional_net.c:6`, `:16`; `optional_rtg.c:18`; `optional_services.c:19-22`; `optional_ui.c:16-17`; `_20.c:12`, `:82`, `:311-312`), at e24a8752

## Recommended fix
record each lazy open and close it from the exit path (ASM-WS5-001's registry or a libnix destructor).

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
ASM-WS5-001, ASM-WS5-002. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
