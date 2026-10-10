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
