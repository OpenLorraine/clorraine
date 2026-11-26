<!-- title: [performance] Format integers with a 16-bit divide on the 68000 -->
<!-- labels: assessment,kind:finding,severity:opportunity,priority:P2,domain:shims,tier:core,performance,enhancement,amiga-sdk -->
<!-- parent: R13 -->

## Summary
Every template placeholder and `toString` of an integer on the 68000 pays for `__udivsi3` per digit; this is most of ASM-WS4-001's `strings` gap.

| Field | Value |
|---|---|
| Finding | ASM-WS5-022 |
| Severity / Priority / Rank | Opportunity / P2 / 57 of 58 |
| Domain / Lens | shims / A500 |
| Confidence / Verdict | verified / confirmed (Opportunity, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Measurement — `make -C docker exec CMD="python3 plans/independent-assessment/probes/WS4/430-patch-measure.py build/tmp/assessment/baseline/clc build/tmp/assessment/WS5/patch 530"` (A500 -O2, `vamos` 68000 trace): `strings` 1,688,727 → 998,843 instructions (−689,884, −40.9% of net 1,685,808), `.text` 23,920 → 23,988 (+68 B), output identical (`records/WS5-runs.log`). The patch (`probes/WS4/530-format-u32-16bit.diff`) splits the value with one 32-bit division by 10,000 and formats each part with the 68000's 16-bit `divu`.

Probe `plans/independent-assessment/probes/WS4/530-format-u32-16bit.diff` (baseline `e24a8752`):

```diff
--- prog.c
+++ prog.c
@@ -1102,9 +1102,17 @@
     unsigned char digits[10];
     ULONG count = 0;
     /* `count == 0` makes the first pass unconditional, so zero prints "0". */
-    while (value != 0 || count == 0) {
+    while (value > 0xFFFFu) {
         digits[count++] = (unsigned char)('0' + (value % 10u));
         value /= 10u;
+    }
+    {
+        unsigned short small = (unsigned short)value;
+        while (small != 0 || count == 0) {
+            unsigned short q = (unsigned short)(small / 10u);
+            digits[count++] = (unsigned char)('0' + (small - q * 10u));
+            small = q;
+        }
     }
     ULONG index;
     for (index = 0; index < count; index++) {
```

## Root cause
generic 32-bit division on a CPU without a 32/32 divide.

Location: `cl_format_u32` in `compiler/runtime/shim_hosted_amiga.c` (digit loop with `/ 10` and `% 10` on `ULONG`), at e24a8752

## Recommended fix
the 16-bit split (as patched), or a reciprocal-multiply digit loop.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it
- The measured target on the stated lens, with the same lens and flags as the evidence (net 68000 instructions under `vamos` or `.text` bytes; compile time for the compile-time items)

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS4-001 (templates cost 33% more than C on the A500). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
