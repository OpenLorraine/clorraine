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
