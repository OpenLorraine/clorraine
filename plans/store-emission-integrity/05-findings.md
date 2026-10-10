# Findings Ledger: Store-Emission Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

Independent per-phase review record. One compact row per ruling batch; the full evidence lives in
the reviewers' reports. Append rows as each phase's review closes — never batch across phases.

Review dispatch: P1/P2 — correctness reviewer + memory-safety auditor + semantics reviewer;
P3/P4 — correctness reviewer + memory-safety auditor + emission-integrity auditor. Packets include
the blast-radius and test-integrity steps.

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MINOR | plan progress header | fixed — the counter and stamp were corrected to 6/26 with the phase close (the review read the header before the last promotion landed) |
| RV-002, RV-003, SR-001, SR-002 | 1 | MINOR | pre-existing multi-arm read-only reports; impl-helper empty-emission reads; ASan env flag; standalone-pin discrimination | dismissed — pre-existing behavior or established helper conventions with no phase action; the suite discriminates the change through ST-1/ST-2 (suppression removed) and ST-5/ST-6 (non-target lend arms retained) |
| — (clean) | 1 | — | `own/expr.rs` `lend_moves_judged` filter, the spec suite, the impl pins | no critical/major findings — correctness clean (one report per site, direct branch unchanged, blast radius confined to the single suppression test); memory-safety clean (the amended shapes are rejected before emission after a base-compiler ASan use-after-free repro; every accepted shape builds and runs ASan-clean with leak detection, including nested, both-arms-self, call-arm, loop, and view instances; 23 related suites pass); semantics clean (conforms to the lend-store rule and every pinned spelling; no evaluation change) |
| RV-004 | 2 | CRITICAL | `own/expr.rs` value-position update walk | fixed — an update used as a value (`i32(s[0]++)`) now judges the immutable-view rule through a shared helper; the statement path evaluates its operand directly so the site keeps exactly one report; ST-14b, an impl pin, and positional probes (condition, for-step, two sites) pin the behavior; re-reviewed on the fix diff |
| SR-003 | 2 | MAJOR | `own/place.rs` `projection_ty` field step through `raw` | fixed — the field step now reaches through a raw pointer to its pointee, closing the `raw S` field spelling (`p.tag[0] = 65` reports one `E10042` naming `p.tag`); impl pin added; re-reviewed on the fix diff |
| RV-005 | 2 | MINOR | element loop over `str` | deferred — filed as DEF-118 (pre-existing, separate mechanism; the read-only loop also emits C that does not build) |
| RV-006 | 2 | MINOR | `ASSERTED_CODES` line length | fixed — reflowed to the sibling line width |
| SR-004 | 2 | MINOR | module-level fixed-array literal (memory-audit Finding 2) | deferred — this is the batch's own DEF-116, fixed by Phase 4 |
| SR-005 | 2 | MINOR | `cstring` element writes still accepted | dismissed — explicitly out of scope (AR-15); the mutability-table question stays open for a user decision |
| SR-006 | 2 | MINOR | nested field-path naming (`s.inner.tag[0]` → `'s.tag'`) | dismissed — matches the design's documented binding/field naming rule |
| SR-007 | 2 | MINOR | rejected local writes still lower IR after the report | dismissed — no observable effect (the driver gates output on a zero exit code; codegen never runs) |
| — (clean) | 2 | — | mutability-table conformance, taxonomy ordering, spec boundary | no findings — the check rejects exactly the `str`/`static str` element forms with one report per site and the E10023/E10024 precedence preserved; the spec delta is exactly the authorized row, golden, and driver registration; the audit found no memory-safety defect in the reviewed change (the rejected forms are compile-time rejections; accepted controls build and run ASan-clean) |
