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

### Scoped re-review (`26b6bd5..57d46cc`)

One scoped round, both lenses:

- **Memory-safety: no findings.** All seven MS-001 probes (while / for-in / do-while / nested /
  generic / generic-nested / two-site loops with owned temporaries in the lambda body) run
  ASan-clean on the fix; the base reproduces each leak. The DEF-138 collision family stays clean,
  the controls stay clean, and `RDC_ASAN=1` spec suite stays green.
- **Correctness: RV-004 (🔴 CRITICAL) — the guard dropped the defer registration record.**
  `check_defer` lowers the emitted defer body with `report_diagnostics = false`, and a
  capture-less lambda's site is pass-ordered, so the reporting re-walk allocated a different site
  and the emitted body called an undefined `cl_lambda_N` (strict build failure; not present at
  `26b6bd5`). RV-001 and RV-003 confirmed closed; RV-002 confirmed closed against `c5d03e5` (the
  earlier packet's stated base was corrected).

### Fix round 2 (this commit)

| Finding | Fix |
| ------- | --- |
| RV-004 (critical) | The guard keys on a new `speculative_lowering` flag set only by the discarded loop fixpoint pass (`own.rs::loop_entry_state`), so the defer registration's retained records — the ones the emitted body's call sites reference — are kept. Regression spec case `should_build_a_capture_less_lambda_inside_a_defer` (strict build + run). |

**Trade-off recorded:** the defer registration lowering suppresses owned-temporary materialization
(DEF-158), so a capturing lambda in a `defer` that borrows an owned temporary leaks 10 B —
byte-identical to the base `26b6bd5` measurement and the filed DEF-158 (this batch's Phase 3).
Keeping the registration records makes the emitted IR's site resolvable; DEF-158's fix (enabling
defer materialization) upgrades the same record in Phase 3. The interim reporting-only guard had
masked that leak while breaking the capture-less shape — an accidental state, not a deliverable.

### Verification evidence (fix round 2)

`RDC_ASAN=1` spec suite 6/6 and impl suite 5/5 green; `make check` green (workspace, container);
the RV-004 reproduction builds and runs with the narrowed guard; the loop-lambda probe is
ASan-clean; the defer-lambda probe is ASan-identical to base (DEF-158). Scoped review budget: one
round; the second fix is verified directly (no third review round).

## Phase 2: Callable effect modes (DEF-155, DEF-156, DEF-157, DEF-166)

**Reviewers**: correctness reviewer (fallback packet), semantics reviewer (fallback packet),
memory-safety auditor, emission-integrity auditor, over the diff `c137429..94ba12b`.

### Merge table

| Finding | Severity | Area | Ruling |
| ------- | -------- | ---- | ------ |
| MS-001 (= memory P-1's module twin) | 🔴 critical | A mixed conditional lend argument (`pick ? stringOf("aa") : b`) was adopted by the new argument materialization: the store emptied the place arm, the checker had recorded a lend, and the accepted program segfaulted on all four callee kinds. The pre-existing module spelling crashed identically. | Fixed: a place-rooted arm stays an inline read and each ownerless arm binds through the deferred fill; the intrinsic spelling's fresh-arm leak closed too. |
| RV-001 = SR-001 = memory P-2 | 🟠 major | A call-result callee (`pick()(s)`) had its modes applied by the lowering but checked nowhere; reuse was accepted and read the emptied place, and a borrow conflict was unchecked. | Fixed: the checker resolves the returned signature (same resolution the lowering uses) and runs the marker and argument checks. |
| SR-002 (check half) | 🟠 major | A qualified callable global (`mod.g(s)`) applied the take move without the check. | Fixed: `value_callee_global_type` resolves the value path in the checker and the lowering; the marker check applies. |
| EIA-001 = RV-002 = SR-003 | 🟡 minor | The new global branch skipped the `E10039` call-site marker check. | Fixed. |
| RV-003 | 🟡 minor | Three doc comments still claimed field calls carry no modes. | Fixed. |
| RV-004 | 🟡 minor | The two take move-out pins also matched the function-exit cleanup. | Fixed: the pin bounds the gap after the call to `#line` markers. |
| RV-005 | 🟡 minor | The re-pinned may-lend test's doc described the old double-release failure mode. | Fixed. |
| SR-004 | 🟡 minor | A stale "i32 then String" doc line survived the re-pin. | Fixed. |
| memory coverage note | 🟡 minor | The suites never *ran* an accepted take (rejection and emitted-text pins only). | Fixed: two runtime spec cases (place arm; the fresh arm across five callee kinds). |

### Governance ratification (user, 2026-10-10)

The DEF-166 runtime correction (AR-17) and the two re-pinned `*_spec_test.rs` fixtures were
submitted for the user's ruling, as the project review contract requires for a specification-test
assertion change. **Ratified**: the correction conforms to the frozen chapters (unmarked
parameters are read-only lends; the module twin and the block-bodied lambda already report
`E10034` on the base compiler), and the frozen `spec/` files were never touched.

### Filed (pre-existing classes; new roadmap rows)

| # | Sev | Finding | Filed as |
| - | --- | ------- | -------- |
| EIA PE-2 + PE-3 (= memory P-4) | 🟠 | A call through an imported or qualified module global (a `func` value or a struct value's field) emits the bare written name; the strict build fails. | DEF-170 |
| EIA PE-1 | 🟠 | `spec/tests/compile-fail/E10213_embed_outside_const.clo` is accepted and emits invalid C. | DEF-171 |
| EIA PE-4 | 🟠 | A module `func`-typed global initialized with a lambda is emitted as NULL. | DEF-172 |
| EIA PE-5 | 🟡 | A same-named function in another loaded module supplies modes to a current-module `func`-typed global. | DEF-173 |
| SR/RV P-2 = EIA M04 | 🟡 | A call-site marker on a closure-call argument is accepted. | DEF-174 |

### Fix-round verification

Spec suite 24/24 plain and under `RDC_ASAN=1`; impl suite 11/11; `make check` 58 suites green.
Host probes of every fixed shape (both conditional arms across the callee kinds, call-result
take/borrow, imported and qualified globals, the global marker) run clean under the sanitizer.

### Scoped re-review (fix diff)

_Pending — one scoped round after the fix commit._
