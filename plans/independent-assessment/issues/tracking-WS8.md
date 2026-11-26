<!-- title: [test] Minor findings from the independent assessment: test integrity -->
<!-- labels: assessment,kind:tracking,severity:minor,priority:P3,domain:tests,tier:advanced,tier:core -->
<!-- parent: - -->

## Summary
The independent assessment of baseline `e24a8752` left 4 surviving Minor finding(s) in the test integrity workstream. Each line names the remediation theme that closes it; details are in `plans/independent-assessment/findings/WS8-test-integrity.md`.

- [ ] **ASM-WS8-001** — The fuzzer cannot generate the constructs where the defects are (effort M, verified, verdict confirmed (Minor, verified); theme {THEME:R8})
  - The differential fuzzer guards arithmetic and layout, where few defects were found; the classes that recur (memory safety, invalid C, miscompile) are outside its reach.
  - Location: `tools/fuzz/fuzz.py:155-283` (`GENERATORS = [_narrow, _fill, _enum, _float, _shift, _struct_loop]`), at e24a8752
  - Fix: add typed generators for calls with effectful arguments (print order), owned values with instrumented deleters (drop order), module-level data, identifiers from a C keyword/macro list, generics and closures; run them under ASan/UBSan as the seeds already are.
- [ ] **ASM-WS8-002** — Tests and gates assert weaker properties than the behavior they cover (effort M, verified, verdict confirmed (Minor, verified) with a census correction; theme {THEME:R8})
  - Several tests and gates assert a weaker property than the behavior they cover: drop-timing tests check leak-freedom, the fault gate checks the message but not the system state, the A1200 Workbench golden was captured by the defective capture code, and the A600 gate counts colours instead of gadgets. (About 32 spec tests check only a clean sanitizer run; most "sanitizer-only" tests also assert a computed exit code.)
  - Location: the suites and gates named in the gap table (B and C rows): `docker/scripts/run-fault-amiga.sh`, `docs/amiga-sdk/a1200-workbench.png`, `docker/scripts/a600-addons-smoke.sh:235-236`, the conditional-projection "statement end" tests, at e24a8752
  - Fix: assert the observable behavior (deleter print order, the walker's system state after a fault, colours computed from the known palette, the gadget count from the window's list); regenerate goldens only from an independent oracle.
- [ ] **ASM-WS8-003** — Memory-safety and invalid-C defects recur because drop and emission decisions are made per expression shape, in two stages (effort XL, verified, verdict confirmed (Minor; counts verified, structure probable), narrowed to causes (2)–(4); theme {THEME:R6})
  - Invalid C, route-specific false accepts, and unsequenced-effect miscompiles recur because emission is composed from strings per shape, semantic checks are attached to particular syntactic routes, and effects are left inside C expressions whose order C leaves unspecified (118, 100, and 51 DEF rows). The fourth cause, the second drop decision in codegen, is ASM-WS2-003.
  - Location: `compiler/src/own/` (137 `duplicable`/`maybe_owns` decisions) and `compiler/src/codegen/` (87 `needs_drop`/`duplicable` decisions re-derived, 709 `format!`-built C fragments), at e24a8752
  - Fix: one structural change addresses all four: lower to an A-normal-form IR (every effectful subexpression bound to a temporary in source order) before ownership checking; let the ownership pass record every move and drop in the IR and have codegen emit only recorded drops; emit through a small typed C syntax tree and printer that escapes names and parenthesizes by construction; run the semantic checks on the normalized form so every spelling reaches the same check. Then extend the fuzzer (ASM-WS8-001) to the shapes the IR must normalize.
- [ ] **ASM-WS8-004** — The Amiga shims never run under a sanitizer, and UBSan covers a third of the ASan suites (effort M, verified, verdict confirmed (Minor, verified); theme {THEME:R8})
  - The Amiga shims (10,404 lines) never run under AddressSanitizer or UBSan, and UBSan covers 12 suites against ASan's 38. This gap explains why the 23 WS5 findings were invisible to the existing checks; many of them are OS-protocol errors that no sanitizer would catch, so the remedy is behavioral gates as much as sanitizers.
  - Location: `compiler/runtime/shim_hosted_amiga*.c` (10,404 lines); the `RDC_ASAN` (38 suites) and `RDC_UBSAN` (12 suites) markers in `compiler/tests/`, at e24a8752
  - Fix: keep the `vamos --vamos-leaks` detector and the FS-UAE system-state walker as standing gates; extract the OS-independent shim parts (buffers, strings, formatting, IFF, colour decoding) into a host-testable file under ASan/UBSan; run every ASan suite under UBSan as well.

## Constraints
- Do not change `spec/` without a ruling; keep plan identifiers out of source and doc comments (AGENTS.md).
