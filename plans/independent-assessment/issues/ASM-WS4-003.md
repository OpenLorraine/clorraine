<!-- title: [performance] Write runtime faults without linking printf -->
<!-- labels: assessment,kind:finding,severity:opportunity,priority:P2,domain:optimizer,tier:core,performance,enhancement -->
<!-- parent: R12 -->

## Summary
On a 512 KB A500 every program that indexes, divides, or converts a float (almost all) carries about 16 KB of formatted-output and long-double code it never uses outside a fault message; the same applies to the equivalent C written with `printf`.

| Field | Value |
|---|---|
| Finding | ASM-WS4-003 |
| Severity / Priority / Rank | Opportunity / P2 / 53 of 58 |
| Domain / Lens | optimizer / A500 shipped -O2 (every Amiga lens links the same code) |
| Confidence / Verdict | verified / confirmed (Opportunity, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — `425-size.sh` (`records/WS4-size.log`, A500 -O2 without `-s`): the `sieve` binary retains `___vfprintf_total_size` 10,582 B, `___addxf3` 2,452 B, `___mulxf3` 1,124 B, `___cmpxf2` 776 B, `___xf_pack` 388 B, `___extenddfxf2` 288 B; `calibration` (no runtime check) retains none of them. Patch 417 (`probes/WS4/417-trap-no-printf.diff`) writes the same line with `fputs`/`fputc`: `.text` 23,772 → 7,120 B (−16,652 B, −70%), output identical, instructions +12.

Probe `plans/independent-assessment/probes/WS4/417-trap-no-printf.diff` (baseline `e24a8752`):

```diff
--- prog.c
+++ prog.c
@@ -545,7 +545,11 @@
  * leg reads it from the program's standard output. */
 static inline void cl_trap(const char *code, const char *message)
 {
-    printf("error %s: %s\n", code, message);
+    fputs("error ", stdout);
+    fputs(code, stdout);
+    fputs(": ", stdout);
+    fputs(message, stdout);
+    fputc('\n', stdout);
     fflush(stdout);
     exit(20);
 }
```

## Root cause
the fault path formats with `printf`, which links libnix's full `vfprintf` including floating-point conversion.

Location: `compiler/runtime/shim_hosted_amiga.c:544-549` (`cl_trap` uses `printf("error %s: %s\n", …)`), at e24a8752

## Recommended fix
write the fault line with `fputs`/`fputc` (or `Write` to `Output()`), exactly as patch 417 does; check the other shim `printf`/`snprintf` users the same way.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it
- The measured target on the stated lens, with the same lens and flags as the evidence (net 68000 instructions under `vamos` or `.text` bytes; compile time for the compile-time items)

## Verification
- AGENTS.md gates: `make check`; `make bench` and `make bench-compare` (and `make bench-compile` for compile time)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, memory-safety auditor, emission-integrity auditor

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
WS4-07; WS7-02 (A500 footprint). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS4-code-quality.md`.
