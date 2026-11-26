<!-- title: [correctness] Minor findings from the independent assessment: standard library -->
<!-- labels: assessment,kind:tracking,severity:minor,priority:P3,domain:stdlib,tier:advanced,tier:core -->
<!-- parent: - -->

## Summary
The independent assessment of baseline `e24a8752` left 3 surviving Minor finding(s) in the standard library workstream. Each line names the remediation theme that closes it; details are in `plans/independent-assessment/findings/WS6-stdlib.md`.

- [ ] **ASM-WS6-003** — NaN prints as `-nan` on the host and `NaN` on the Amiga profiles (effort S, verified, verdict confirmed (Minor); theme {THEME:R10})
  - A host test golden for a NaN result does not match the target; a NaN's sign leaks into text on the host only.
  - Location: `cl_text_f64` / `cl_str_part_f64` through each C library's `%g` (`compiler/runtime/shim_host.c:540-548`, `compiler/runtime/shim_hosted_amiga.c:1158-1166`), at e24a8752
  - Fix: format non-finite values in the shim (`nan`, `inf`, `-inf`) before calling `%g`; record the spelling in the spec (SQ-WS6-1).
- [ ] **ASM-WS6-004** — 25 public functions, including `std.io`'s writers, have no specification entry (effort M, verified, verdict confirmed (Minor); theme {THEME:R10})
  - The appendix is the frozen API oracle; functions outside it have no stated ownership, nullability, or fault behavior to conform to (the `readFile` defect, ASM-WS6-001, sits in such a gap), and changes to them are not spec changes.
  - Location: `stdlib/` exports listed below, at e24a8752
  - Fix: add a `std.io` section and the missing rows (a spec change, so it needs a user ruling); or mark the functions internal.
- [ ] **ASM-WS6-005** — The appendix's Kickstart minimums and Box signatures drift from the code (effort S, verified, verdict confirmed (Minor); theme {THEME:R10})
  - On a V36 Kickstart 2.0 (early A3000 ROMs) the five modules return null while the appendix says they work; the Box rows teach a parameter spelling the grammar rejects.
  - Location: `spec/appendix-amiga.md:236-250` ("2.0" for `std.amiga.utility`, `gadtools`, `asl`, `regions`, `commodities`) against their opens at V37 (`shim_hosted_amiga_optional.c`, `_optional_ui.c:38`, `:46`, `_optional_services.c:76`); `spec/appendix-stdlib.md:88-92` (`value: take T`) against `stdlib/std.mem.clo:75-91` (`take value: T`) and grammar `param = [ param_mode ] , identifier , ":" , value_type`, at e24a8752
  - Fix: state "2.04 (V37)" in the table; respell the Box rows (both are spec edits needing a ruling).

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
