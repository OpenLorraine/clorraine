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
| RV-001, RV-002 | 2 | MINOR | spec-suite doc comment; plan progress header | fixed — comment reworded (the generic read-only lend rejects like the concrete one, not like the accepted nullable out-slot); header corrected to 14/22 |
| — (clean) | 2 | — | `codegen/cleanup.rs` direct-return branch | no findings — memory-safety and emission-integrity clean; pre-existing emission defects EI-P1/EI-P2 are base==head and already filed (DEF-97/DEF-98) |
| RV-001 | 3 | MINOR | spec-suite determinism case | fixed — plan-structure references renamed to behavioral names |
| — (clean) | 3 | — | `Statement::Guards`, the DEF-105 walk, the emitter arm | no findings — correctness (one minor fixed), memory-safety, and emission-integrity clean; pre-existing defects surfaced and filed: DEF-114 (`str` element store), DEF-115 (slice-of-struct typedef order), DEF-116 (module-initializer array with a global slice); DEF-97/DEF-98 re-confirmed base==head |
