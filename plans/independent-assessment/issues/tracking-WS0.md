<!-- title: [test] Minor findings from the independent assessment: baseline and harnesses -->
<!-- labels: assessment,kind:tracking,severity:minor,priority:P3,domain:tests,tier:core -->
<!-- parent: - -->

## Summary
The independent assessment of baseline `e24a8752` left 1 surviving Minor finding(s) in the baseline and harnesses workstream. Each line names the remediation theme that closes it; details are in `plans/independent-assessment/findings/WS0-baseline.md`.

- [ ] **ASM-WS0-001** — The flag audit's host binary floor fails on `lookup` at the baseline (effort S, verified, verdict confirmed (Minor); theme {THEME:R8})
  - `make bench-flags` exits non-zero on a clean tree, so the size/runtime floor gate is red for every run and cannot signal a real regression on the other rows; no shipped program is affected.
  - Location: `tools/perf/flag_audit.py:444-446` at e24a8752
  - Fix: give data-dominated benchmarks a floor on `.text` instead of the file size, or exclude the table bytes from the host-binary floor; alternatively record `lookup` as exempt from the host floor with its reason.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
