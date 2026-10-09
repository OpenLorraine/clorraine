# Findings Ledger: Borrow-Store Integrity

Independent per-phase review record. One compact row per ruling batch; the full evidence lives in
the reviewers' reports. Append rows as each phase's review closes — never batch across phases.

Review dispatch (AR-13): P1 — correctness reviewer + memory-safety auditor + semantics reviewer;
P2 — correctness reviewer + memory-safety auditor + emission-integrity auditor; P3 — correctness
reviewer + memory-safety auditor + emission-integrity auditor. Packets include the blast-radius
and test-integrity steps.

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| — (clean) | 1 | — | `own/expr.rs` recorder, `own/instance_check.rs` audit, duty plumbing | no findings — correctness, memory-safety, and semantics reviews all clean; report-only observations: no `raw`/`cstring` nullable-substitution pin in the suite (equivalence proven by the semantics review's predicate argument); the bare-name instance filter is shared with the existing audits (pre-existing) |
