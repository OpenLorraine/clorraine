<!-- title: [test] Make the fuzzer and the gates see the recurring defect classes -->
<!-- labels: assessment,kind:theme,severity:minor,priority:P3,domain:tests -->
<!-- parent: - -->

## Goal
Extend the fuzzer to calls, owned values, generics, module data, and identifiers; make gates assert behavior (walker system state, independent goldens); keep the `vamos` leak detector and FS-UAE walker as standing gates

| Field | Value |
|---|---|
| Roadmap entry | R8 `test-and-gate-integrity` (report §8) |
| Order | 8 of 14 |
| Depends on | — |
| Effort | M |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |

## Findings this plan closes
- ASM-WS8-001 (Minor, no DEF) — The fuzzer cannot generate the constructs where the defects are — listed in a tracking issue
- ASM-WS8-002 (Minor, no DEF) — Tests and gates assert weaker properties than the behavior they cover — listed in a tracking issue
- ASM-WS8-004 (Minor, no DEF) — The Amiga shims never run under a sanitizer, and UBSan covers a third of the ASan suites — listed in a tracking issue
- ASM-WS0-001 (Minor, no DEF) — The flag audit's host binary floor fails on `lookup` at the baseline — listed in a tracking issue

## How to start
Create the plan with `make-plan test-and-gate-integrity`, starting from this issue and the finding issues below it; execute it with `exec-plan`. The report (`plans/independent-assessment/10-assessment-report.md` §5 and §8) explains the theme.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
