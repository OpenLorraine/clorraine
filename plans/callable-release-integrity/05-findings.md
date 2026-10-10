# Findings: Callable Release Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Protocol**: per-phase reviews follow the `AGENTS.md` dispatch matrix; each finding gets a
> ruling before its phase closes. Provider note: the dedicated `correctness-reviewer` agent was
> not dispatchable in the executing session; its brief ran through the generic fallback packet
> (reported per the quality profile). The memory-safety and emission-integrity auditors
> dispatched natively.

## Phase 1: Generated identities (DEF-138, DEF-140, DEF-165)

**Reviewers**: correctness reviewer (fallback packet), memory-safety auditor, emission-integrity
auditor, over the diff `c5d03e5..26b6bd5`.

### Full review verdicts

- **Correctness: one CRITICAL, two MINOR.** RV-001 (critical): the (site, instance) dedupe keeps
  the first record, and for a loop lambda the first record can be the speculative pass's — a
  lowering with owned-temporary materialization suppressed — so a capturing lambda in a loop whose
  body passes an owned temporary to a `borrow` parameter leaked one allocation per call (20 B in
  2). The base could not reach the shape (it failed the strict build with the redefinitions), so
  the phase introduced the leak. RV-002 (minor, test-integrity): the `idspell` impl case's
  assertions also hold at base — it did not pin the fix. RV-003 (minor, coverage bookkeeping): the
  byte-identity element claimed by task 1.3.1 was absent.
- **Memory-safety: the same critical independently (MS-001)**, plus two pre-existing classes (P-1:
  owned temporary arguments to a field call leak — the filed DEF-155, this batch's Phase 2;
  P-2: owned temporaries inside a `defer` block leak — the filed DEF-158, this batch's Phase 3).
  No double-free, use-after-free, or invalid free in any probed head shape; the DEF-138 collision
  family (mixed orders, scoped owners, loops, defer/break/continue/return edges) releases each
  generated owner exactly once; base still reproduces the original double-free.
- **Emission-integrity: no findings.** 15 differing corpus units, every hunk classified to the
  three intended causes (func-key renames, the removed spurious owner copy, the collapsed
  duplicate lifted blocks); base had 7 strict-build failures, head 1 (the pre-existing PRE-1);
  41 variant probes (owner collisions, key grammars, lifted dedupe) all as expected; the func key
  has one owner (`types/keys.rs`); three changed units re-emit byte-identically.

### Fix round (commit follows this ledger)

| Finding | Fix |
| ------- | --- |
| RV-001 / MS-001 (critical) | Only the reporting pass's lifted record is pushed (`own/lower.rs`): a speculative loop pass's record omits the owners and their releases and must never be emitted. Regression spec case `should_release_a_temporary_inside_a_loop_lambda` (ASan-clean per call). |
| RV-002 (minor) | The `idspell` impl pin asserts the base-observable difference: no spurious copy owner may be emitted (the pre-fix head bound a third owner that double-freed). |
| RV-003 (minor) | Byte-identity pin added: `should_re_emit_the_identity_programs_byte_identically` emits two programs twice via `emit_twice` and compares. |

### Filed (pre-existing classes; new roadmap rows)

| # | Sev | Finding | Filed as |
| - | --- | ------- | -------- |
| EIA PRE-1 | 🟠 | A call-shaped comparison expression (`a < b > (c)`) emits `a(c)`; the strict build fails on an accepted program. | DEF-167 |
| EIA PRE-2 | 🟠 | An array-literal argument to a field call emits `(d.call)({ 1, 2 })`; the strict build fails. | DEF-168 |
| EIA PRE-3 | 🟡 | The func-key separator grammar can collide with crafted user identifiers (fail-closed); recorded in the `types/keys.rs` documentation. | DEF-169 |
| MS P-1 | 🟠 | Owned temporary arguments to a field call leak (`own/call.rs` yields no modes for receivers). | DEF-155 (in-batch, Phase 2) |
| MS P-2 | 🟠 | Owned temporaries inside a `defer` block leak. | DEF-158 (in-batch, Phase 3) |

### Verification evidence (fix round)

`RDC_ASAN=1` spec suite 5/5 and impl suite 5/5 green; `make check` green (workspace, container);
the red evidence for the new regression case is the base tree's strict-build failure plus the
reviewer's head probe (`LeakSanitizer: 20 byte(s) leaked in 2 allocation(s)` before the fix).

### Scoped re-review (`26b6bd5..fix`)

_Pending — the once-only scoped re-review of the fix diff is recorded here when it completes._
