# Findings Ledger: Closure Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

Independent per-phase review record. One compact row per ruling batch; the full evidence lives in
the reviewers' reports. Append rows as each phase's review closes — never batch across phases.

Review dispatch (AR-15): P1 — correctness reviewer + semantics reviewer; P2 — correctness reviewer
+ memory-safety auditor; P3 — correctness reviewer + semantics reviewer. Packets include the
blast-radius and test-integrity steps.

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MAJOR | `sema/mod.rs` `name_is_module_binding` | fixed — the E10406 predicate covered only `let`/`const`/imports, so module function and type names in a capture list were still accepted and emitted the same environment-less call; broadened to every module-level name (the user-approved trigger text already reads "a module-level binding"; AR-8 amended with the reopen evidence); function, type, and import spellings pinned in both suites; re-reviewed on the fix diff — accepted |
| RV-002 | 1 | MINOR | impl relay span pin | fixed — the split source left the entry and the body read on one line, so the line-only fragment did not discriminate; the pin now names the entry's column (`main.clo:7:11`), per the re-review's exact instruction |
| RV-003 | 1 | MINOR | entry-loop comment | fixed — the comment now states that a rejected entry stays in the raw list so the body read keeps one report |
| SR-001 | 1 | MINOR | import-binding branch untested | fixed — the import spelling is pinned in both suites |
| SR-002 | 1 | MINOR | AR-9 span wording versus the code | fixed — AR-9 and 03-01 now say the report points at the entry inside the enclosing lambda (the text whose fix is to extend the enclosing list) |
| SR-003 | 1 | MINOR | cascading relay reports (one per broken entry) | dismissed — the per-entry rule reports each unreachable entry once; the component spec pins the behavior and no accept/reject outcome changes |
| — (clean) | 1 | — | the relay rule, the dotted root, the E10406 precedence, the accepted controls | no critical findings — the semantics review confirmed conformance to the frozen listing and write rules and the register decisions; the relay rule cannot over- or under-report across the probed depths, module spellings, and lexical shadows; the dotted root reaches only the two capture rules; the spec delta is exactly the authorized row, golden, and driver registration |
| RV-004 | 2 | MINOR | duty recording in block-bodied lambdas | fixed — a block body consuming its by-value capture reported `E10034` plus the new `E10401` (base: `E10034` only); the duty now records only inside expression-bodied lambdas (a block body's move is already rejected at the declaration), and a regression pin asserts one report per body form |
| RV-005 | 2 | MINOR | `lowered_lend_source` guard scope | fixed — the temp-owner collection now skips receiver calls (`receiver: None`), mirroring the may-lend gate's exclusion; the residual module-qualified-name ambiguity is pre-existing and was not reproduced |
| RV-006 | 2 | MINOR | block-bodied generic capture return reports `E10034`, not `E10401` (pre-existing) | accepted divergence — the shape is safely rejected and base-identical; the proper fix (suppress the not-lend duty for lambda captures, record the duty at the block return) is filed as DEF-126 |
| MS-001 | 2 | MAJOR (pre-existing) | conditional capture return accepted and double-frees | filed as DEF-127 (outside this batch's scope; base-identical) |
| MS-002 | 2 | MAJOR (pre-existing) | owned temporary argument to a func-value call leaks | filed as DEF-125 during the phase |
| MS-003 | 2 | — | block-bodied lender-capture variant rejected with `E10041` (base-identical) | accepted — base and head agree and the shape is rejected safely; no action |
| — (clean) | 2 | — | DEF-104/DEF-109 conformance, the gate guard, the corrected tests | correctness review: no critical or major findings — the duty matches the concrete rule across instance orders, duplicable, nullable, and view instances, take forms, and mere reads; the gate guard is release-clean for lender, local, and value shapes; the two corrected tests are legitimate and still discriminate |
| — (clean) | 2 | — | new memory defects | memory audit: none introduced — 45 host ASan probes, every head/base difference a fix or a no-op (the double-free and leak repros fixed; duplicable instances clean) |
| RV-301 | 3 | MAJOR | dot-call path misses the param-typed field call | fixed — `h.call(5)` (field type `T`, no bound) was accepted and emitted unqualified `call(h, 5)`; the dot-call fallthrough now reports `E10307` on the same rule (the ruled trigger text covers the shape), with a spec case and an impl pin added; the scoped re-review accepted the remediation with no findings |
| RV-302 | 3 | MINOR | the callable-bound control used a function pointer, while the testing strategy freezes the shape as a matching lambda | fixed — the matching-lambda control was added (exit 5) beside the function-pointer control (exit 6) |
| RV-303 | 3 | MINOR | the ST-14 doc comment said the report is "at the declaration" | fixed — "where the call is written", matching the callee span |
| — (clean) | 3 | — | G-1/G-6 conformance, the bounded path, diagnostic quality, the spec boundary | semantics review: the rejection follows the ruled rule; the bounded path and every accepted call form stay accepted; the registry wording, help, and span match the approved text; the spec delta is exactly the authorized row, golden, and driver entry |
| OBS-1 | 3 | — | the field-case span is the receiver expression (consistent with every other dot-call diagnostic) | accepted — consistent behavior; the tests pin the code and message, not the line |
| OBS-2 | 3 | — | a member call on a bare-parameter receiver is accepted and emits `foo(x, 5)` (base == head) | filed as DEF-129 (a member-lookup gap, not the callability trigger) |
