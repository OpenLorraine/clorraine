# Issue drafts for approval (task 10.3.3)

Repository `OpenLorraine/clorraine`. 55 new issues plus 2 comments on existing issues. Creation order after approval: theme parents, then finding issues as sub-issues of their theme (`gh issue create --parent`), then tracking issues. Every label below already exists; none needs creating.

## Comments on existing issues (instead of new issues)

- **#20** — Measured in the independent assessment (baseline `e24a8752`, A500 shipped lens, `vamos` 68000 instruction counts): `-O2 -flto` saves 280–512 B of `.text` and cuts `container` by 7.3%; `-O3` cuts `strings` by 25.1% and `container` by 18.3% for +270–2,216 B (`plans/independent-assessment/findings/WS4-code-quality.md`, ASM-WS4-006; record `findings/records/WS4-flags.log`). The earlier rejection of `-O3` measured the calibration program only.

- **#15** — Data point from the independent assessment (baseline `e24a8752`): a 68020 probe build (`-m68020`, `vamos -C 68020`) of the benchmarks gains −16.3% net instructions on `strings` (hardware divide) and stays within ±5% elsewhere, while `container` regresses by +21.3% (ASM-WS7-003, `plans/independent-assessment/findings/WS7-performance.md`). The report's recommendation is not to add a 68020 profile on these numbers alone; the `strings` gain is available on every CPU through a 16-bit formatting divide.

ASM-WS4-006 (flags) is covered by #20 and gets no new issue; ASM-WS7-003 stays in the report and is noted on #15.

## New issues

| # | Kind | Title | Labels | Parent |
|---|------|-------|--------|--------|
| 1 | theme | [bug] Release OS resources on every Amiga program exit | assessment, kind:theme, severity:critical, priority:P0, domain:shims, amiga-sdk | — |
| 2 | theme | [correctness] Sequence effects in source order and give temporaries statement scope | assessment, kind:theme, severity:critical, priority:P0, domain:codegen, domain:ownership, domain:semantics | — |
| 3 | theme | [bug] Generate collision-free C names | assessment, kind:theme, severity:critical, priority:P0, domain:codegen | — |
| 4 | theme | [correctness] Bound the front end and enforce its checks on every route | assessment, kind:theme, severity:critical, priority:P0, domain:ownership, domain:semantics | — |
| 5 | theme | [bug] Fix the Amiga shim paths that crash or hang a real machine | assessment, kind:theme, severity:critical, priority:P0, domain:shims, amiga-sdk | — |
| 6 | theme | [bug] Emit C through a typed representation | assessment, kind:theme, severity:major, priority:P1, domain:codegen, domain:tests | — |
| 7 | theme | [correctness] Make the ownership pass the single drop authority | assessment, kind:theme, severity:minor, priority:P3, domain:ownership | — |
| 8 | theme | [test] Make the fuzzer and the gates see the recurring defect classes | assessment, kind:theme, severity:minor, priority:P3, domain:tests | — |
| 9 | theme | [bug] Follow the AmigaOS protocols in the remaining shim wrappers | assessment, kind:theme, severity:major, priority:P1, domain:shims, amiga-sdk | — |
| 10 | theme | [correctness] Bring the standard library in line with its specification | assessment, kind:theme, severity:major, priority:P1, domain:codegen, domain:stdlib | — |
| 11 | theme | [bug] Keep large dispatch code within vbcc's limits | assessment, kind:theme, severity:major, priority:P1, domain:codegen | — |
| 12 | theme | [performance] Lower module initializers and the fault writer compactly | assessment, kind:theme, severity:major, priority:P1, domain:optimizer, domain:perf | — |
| 13 | theme | [performance] Close the template and Array gaps to C | assessment, kind:theme, severity:major, priority:P1, domain:optimizer, domain:perf, domain:shims, amiga-sdk | — |
| 14 | theme | [performance] Keep compile time near-linear | assessment, kind:theme, severity:major, priority:P1, domain:perf | — |
| 15 | finding | [bug] Release interrupt servers and other OS registrations when a program faults | assessment, kind:finding, severity:critical, priority:P0, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R1 |
| 16 | finding | [correctness] Evaluate call arguments left to right | assessment, kind:finding, severity:critical, priority:P0, domain:semantics, tier:core, tier:advanced, bug | R2 |
| 17 | finding | [correctness] Release owned temporaries at the end of their statement | assessment, kind:finding, severity:critical, priority:P0, domain:ownership, tier:core, tier:advanced, bug | R2 |
| 18 | finding | [bug] Clip drawText on screens that have no layer | assessment, kind:finding, severity:critical, priority:P0, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R5 |
| 19 | finding | [bug] Handle runtime faults inside worker tasks without DOS calls or exit() | assessment, kind:finding, severity:critical, priority:P0, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R1 |
| 20 | finding | [bug] Open intuition.library before savePublicScreen and showAlert use it | assessment, kind:finding, severity:critical, priority:P0, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R5 |
| 21 | finding | [bug] Finish the clipboard read protocol so a later write does not hang | assessment, kind:finding, severity:critical, priority:P0, domain:shims, tier:advanced, bug, amiga-sdk | R5 |
| 22 | finding | [bug] Escape C99 keywords and header macro names used as identifiers | assessment, kind:finding, severity:critical, priority:P0, domain:codegen, tier:core, tier:advanced, bug | R3 |
| 23 | finding | [bug] Stop clc overflowing its stack on deeply nested code | assessment, kind:finding, severity:critical, priority:P0, domain:semantics, tier:core, bug | R4 |
| 24 | finding | [bug] Report E10305 for polymorphic recursion with explicit type arguments | assessment, kind:finding, severity:critical, priority:P0, domain:semantics, tier:core, bug | R4 |
| 25 | finding | [bug] Keep generic instance symbols apart from module function symbols | assessment, kind:finding, severity:critical, priority:P0, domain:codegen, tier:core, tier:advanced, bug | R3 |
| 26 | finding | [bug] Prevent use after free of installed clip regions and AmigaGuide messages | assessment, kind:finding, severity:critical, priority:P0, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R5 |
| 27 | finding | [bug] Report read errors from readFile distinctly from end of file | assessment, kind:finding, severity:major, priority:P1, domain:stdlib, tier:core, tier:advanced, bug | R10 |
| 28 | finding | [bug] Make clipboardWrite store data the clipboard device keeps | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R9 |
| 29 | finding | [performance] Lower module-level array fills without one store per element | assessment, kind:finding, severity:major, priority:P1, domain:perf, tier:core, tier:advanced, performance | R12 |
| 30 | finding | [bug] Read 2.0+ screen colours correctly in the native capture | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R9 |
| 31 | finding | [bug] Attach, render, and free every GadTools gadget | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:advanced, bug, amiga-sdk | R9 |
| 32 | finding | [bug] Stop cardPresent and cardStatus removing card.resource | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:advanced, bug, amiga-sdk | R9 |
| 33 | finding | [bug] Return with Z set from the VBlank and PORTS interrupt servers | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R9 |
| 34 | finding | [correctness] Range-check decimal literals in inferred-type bindings | assessment, kind:finding, severity:major, priority:P1, domain:semantics, tier:core, tier:advanced, bug | R4 |
| 35 | finding | [correctness] Emit value-position increments and decrements without signed overflow | assessment, kind:finding, severity:major, priority:P1, domain:codegen, tier:core, bug | R2 |
| 36 | finding | [correctness] Accept slice of a str as the standard library specifies | assessment, kind:finding, severity:major, priority:P1, domain:stdlib, tier:core, tier:advanced, bug | R10 |
| 37 | finding | [bug] Close the library bases the runtime opens | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R1 |
| 38 | finding | [bug] Wait for the blitter before CPU access after layer drawing | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R9 |
| 39 | finding | [bug] Lay out classic menu headers and items | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R9 |
| 40 | finding | [bug] Give the busy pointer a complete sprite definition | assessment, kind:finding, severity:major, priority:P1, domain:shims, tier:core, tier:advanced, bug, amiga-sdk | R9 |
| 41 | finding | [bug] Emit else-if chains without one nested C block per arm | assessment, kind:finding, severity:major, priority:P1, domain:codegen, tier:core, bug | R11 |
| 42 | finding | [performance] Keep large switch statements within vbcc's compile-time budget | assessment, kind:finding, severity:major, priority:P1, domain:codegen, tier:core, performance | R11 |
| 43 | finding | [performance] Make compile time near-linear for call-heavy and generic code | assessment, kind:finding, severity:major, priority:P1, domain:perf, tier:core, performance | R14 |
| 44 | finding | [performance] Lower Array push and get without runtime calls and one-element views | assessment, kind:finding, severity:major, priority:P1, domain:optimizer, tier:core, tier:advanced, performance | R13 |
| 45 | finding | [performance] Lower fixed-shape templates to an inline string builder | assessment, kind:finding, severity:major, priority:P1, domain:optimizer, tier:core, tier:advanced, performance | R13 |
| 46 | finding | [performance] Write runtime faults without linking printf | assessment, kind:finding, severity:opportunity, priority:P2, domain:optimizer, tier:core, performance, enhancement | R12 |
| 47 | finding | [performance] Stop linking unused libamiga stubs and CreatePort | assessment, kind:finding, severity:opportunity, priority:P2, domain:optimizer, tier:core, performance, enhancement | R12 |
| 48 | finding | [performance] Format integers with a 16-bit divide on the 68000 | assessment, kind:finding, severity:opportunity, priority:P2, domain:shims, tier:core, performance, enhancement, amiga-sdk | R13 |
| 49 | tracking | [test] Minor findings from the independent assessment: baseline and harnesses | assessment, kind:tracking, severity:minor, priority:P3, domain:tests, tier:core | — |
| 50 | tracking | [correctness] Minor findings from the independent assessment: semantic core | assessment, kind:tracking, severity:minor, priority:P3, domain:semantics, tier:advanced, tier:core | — |
| 51 | tracking | [correctness] Minor findings from the independent assessment: ownership and drops | assessment, kind:tracking, severity:minor, priority:P3, domain:ownership, tier:advanced, tier:core | — |
| 52 | tracking | [bug] Minor findings from the independent assessment: C code generation | assessment, kind:tracking, severity:minor, priority:P3, domain:codegen, tier:advanced, tier:core | — |
| 53 | tracking | [bug] Minor findings from the independent assessment: runtime shims | assessment, kind:tracking, severity:minor, priority:P3, domain:shims, tier:advanced, tier:core, amiga-sdk | — |
| 54 | tracking | [correctness] Minor findings from the independent assessment: standard library | assessment, kind:tracking, severity:minor, priority:P3, domain:stdlib, tier:advanced, tier:core | — |
| 55 | tracking | [test] Minor findings from the independent assessment: test integrity | assessment, kind:tracking, severity:minor, priority:P3, domain:tests, tier:advanced, tier:core | — |
