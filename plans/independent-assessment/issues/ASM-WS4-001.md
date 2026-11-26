<!-- title: [performance] Lower fixed-shape templates to an inline string builder -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:optimizer,tier:core,tier:advanced,performance -->
<!-- parent: R13 -->

## Summary
Every program that formats text in a loop (status lines, logs, UI labels) pays about a third more CPU on the 68000 than hand-written C for the string building.

| Field | Value |
|---|---|
| Finding | ASM-WS4-001 · DEF-612 |
| Severity / Priority / Rank | Major / P1 / 31 of 58 |
| Domain / Lens | optimizer / A500 shipped -O2 (also A500 -Os +19.4%, A1200 shipped +51.3%, 68020 probe +71.5%) |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — Phase 0 matrix (`records/WS0-matrix.json`, A500 shipped -O2 (hosted-amiga-13, `-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s`, `vamos` 68000), pinned `clc` sha256 `0c2fd66e…`, baseline e24a8752): `strings` Clorraine 1,685,808 vs C reference 1,271,509 net instructions (+32.6%; raw 1,688,727 / 1,274,427; calibrations 2,919 / 2,918). Attribution (`400-attribute.py`, `records/WS4-attribution.log`): `cl_string_concat` 326,086, `cl_string_new` 32,032, and 36,045 extra `memcpy` instructions; formatting is identical on both sides. Patch 413 (`probes/WS4/413-strings-template-inline.diff`, `430-patch-measure.py`, `records/WS4-patches.log`) builds the same string inline: 1,688,727 → 1,286,737 (−401,990, −23.8% of net), output identical.

Probe `plans/independent-assessment/probes/WS4/413-strings-template-inline.diff` (baseline `e24a8752`):

```diff
--- prog.c
+++ prog.c
@@ -6146,7 +6146,19 @@
 #line 13 "/work/benchmarks/strings/program.clo"
     if (s) { FreeString(s); }
     s = 0;
-    s = cl_string_concat((ClStrPart[]){ cl_str_part_bytes((const u8 *)"token-", 6u), cl_str_part_u32((u32)(i)), cl_str_part_bytes((const u8 *)"-", 1u), cl_str_part_u32((u32)((i * (u32)(3)))) }, 4u);
+    {
+        cl_string_block *tb = (cl_string_block *)cl_string_new(27u);
+        unsigned char *td = cl_string_bytes(tb);
+        ULONG tn = 6;
+        memcpy(td, "token-", 6);
+        tn += cl_format_u32(td + tn, (unsigned int)i);
+        td[tn++] = '-';
+        tn += cl_format_u32(td + tn, (unsigned int)(i * (u32)(3)));
+        tb->len = tn;
+        tb->cap = tn;
+        td[tn] = '\0';
+        s = tb;
+    }
 #line 14 "/work/benchmarks/strings/program.clo"
     v = cl_string_view(s);
 #line 15 "/work/benchmarks/strings/program.clo"
```

## Root cause
each template fills a stack array of tagged `ClStrPart` structs and calls a generic builder that walks the parts twice (sizing, then writing) and copies literal pieces with `memcpy`, instead of emitting the fixed sequence of copies and formats the template's shape already determines.

Location: template lowering (`cl_string_concat((ClStrPart[]){…}, n)` per template) in `compiler/src/codegen/`; `cl_string_concat` in `compiler/runtime/shim_hosted_amiga.c:1046-1094`, at e24a8752

## Recommended fix
specialize templates with a fixed shape at lowering time (reserve the sum of the part maxima, copy literals with known lengths, format scalars in place), keeping `cl_string_concat` for the general case.

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
WS4-09 template specialization; WS6-03 (hidden costs). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS4-code-quality.md`.
