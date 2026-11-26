<!-- title: [bug] Return with Z set from the VBlank and PORTS interrupt servers -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
While a Clorraine VBlank handler is installed, servers below priority 0 on the VERTB chain (and on PORTS, CIA-driven and expansion-card servers) are skipped on most frames. What runs below priority 0 depends on the system (third-party drivers, commodities); the A1200 runs in 541c did not show a visible failure.

| Field | Value |
|---|---|
| Finding | ASM-WS5-009 · DEF-600 |
| Severity / Priority / Rank | Major / P1 / 19 of 58 |
| Domain / Lens | shims / A500, A1200 shipped |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -S` of probe 541's emission (`build/tmp/assessment/WS5/fsuae/541.s`): `_cl_vblank_code` is `move.l _cl_vblank_handler,a0; cmp.w #0,a0; jeq L7; jmp (a0)` — a tail call — and the handler `onTick` ends `move.l d0,_cl_Probe_Main_ticks; rts`, which leaves Z clear for a non-zero count. Exec `AddIntServer` autodoc: "Each link in the chain will be called … until … one of the servers returns with the 68000's Z condition code clear … VERTB servers should always return Z set. (Take care with High Level Language servers…)".

## Root cause
a `void` C function cannot set the condition codes the exec protocol needs, and the tail call hands the decision to user code.

Location: `compiler/runtime/shim_hosted_amiga.c:600-612` (`static void cl_vblank_code(void)`, `cl_ports_code`), `:643` (priority 0), at e24a8752

## Recommended fix
make the servers return `LONG` 0 after the call (GCC then emits `moveq #0,d0; rts`, which sets Z) and stop the tail call; for PORTS, return 0 (not "handled") unless the source is known.

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
ASM-WS5-001 (the same servers after a fault). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
