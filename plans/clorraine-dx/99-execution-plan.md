# Execution Plan: Clorraine DX Tooling

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-10 19:09
> **Progress**: 0/28 tasks (0%)
> **CodeOps Artifact Schema**: 1

## Overview

Four phases deliver the DX slice ([AR-1](00-ambiguity-register.md)): explain mode plus the twelve
registry help texts, machine-readable diagnostics, the canonical TextMate grammar, and the batch
close-out. Each phase follows the specification-first ordering and cites its owning
[03-XX](00-index.md) document and the ST-cases in [07-testing-strategy.md](07-testing-strategy.md).

Execution commit mode: **auto-commit + auto-design** (user instruction) — commit and push each
verified task via the git-commit skill in push mode; eligible technical mechanism decisions are
delegated under the shared auto-design policy, rulings stay reserved.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Explain mode and the registry help texts | 9 |
| 2 | Machine-readable diagnostics | 8 |
| 3 | Canonical TextMate grammar | 8 |
| 4 | Batch close-out | 3 |

**Total: 28 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Explain mode and the registry help texts

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (the registry help wording describes
> frozen rules; AR-10)
> **Reasoning**: medium — CLI plumbing plus a normative-data parse; the spec edit is bounded

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-explain-mode.md) §Render shape · [AR-2, AR-3, AR-4, AR-10, AR-13,
AR-14](00-ambiguity-register.md)
**Objective**: Pin the seven explain cases — both spellings, both binaries, the exact render, and
the two failure shapes — before any implementation.

- [ ] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-7 — `compiler/tests/dx_tooling_spec_test.rs`
- [ ] 1.1.2 Run the spec suite and record the red phase (expected: every case fails — the flag is unknown)

### Step 1.2: Implementation

**Reference**: [03-01](03-01-explain-mode.md) §Implementation Details
**Objective**: Land the registry parse/render module, wire the flag into both CLIs, and author
the twelve help texts.

- [ ] 1.2.1 Add the twelve approved help cells — `spec/14-diagnostics.md`
- [ ] 1.2.2 Implement the registry parse, lookup, and render — `compiler/src/diag/registry.rs`, `compiler/src/diag.rs`
- [ ] 1.2.3 Implement `clc --explain` (both spellings, early exit) — `compiler/src/driver.rs`
- [ ] 1.2.4 Implement `clorraine --explain` and the help line — `compiler/src/clorraine/cli.rs`, `compiler/src/clorraine/mod.rs`
- [ ] 1.2.5 Run the spec suite to green; on any failure STOP and fix the implementation, never the test

**Verify**: `make -C docker exec CMD="cargo test --test dx_tooling_spec_test"`; `./spec/check.sh`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-explain-mode.md) §Testing Requirements
**Objective**: Pin the parse internals, the 112-code coverage, and the render shape; record the
phase review evidence.

- [ ] 1.3.1 Write the Phase 1 implementation tests (synthetic-table parse, 112-code coverage with help on every row, exact render, trailing newline) — `compiler/tests/dx_tooling_impl_test.rs`, `compiler/src/diag/registry.rs`
- [ ] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- Explain mode on both CLIs with the shared registry parse
- The twelve registry help texts landed
- Phase 1 implementation tests

**Verify**: `make -C docker exec CMD="cargo test --test dx_tooling_impl_test"`; `make check`

---

## Phase 2: Machine-readable diagnostics

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer (CLI output contract; no ownership/codegen surface)
> **Reasoning**: medium — a public output schema with escaping and error-path routing

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-machine-readable-diagnostics.md) §JSON schema · [AR-5, AR-6, AR-7,
AR-15](00-ambiguity-register.md)
**Objective**: Pin the eight JSON cases — error, warning, driver errors, escaping, deny-warnings,
the unchanged human render, the unknown value, and the driver forwarding.

- [ ] 2.1.1 [spec-author] Write specification tests from ST-8 – ST-15 — `compiler/tests/dx_tooling_spec_test.rs`
- [ ] 2.1.2 Run the spec suite and record the red phase (expected: every JSON case fails with `unknown option`; ST-12 passes as the deliberate control)

### Step 2.2: Implementation

**Reference**: [03-02](03-02-machine-readable-diagnostics.md) §Implementation Details
**Objective**: Land the format enum, the JSON renderer with escaping, the pre-scan, and the
error-path routing in both binaries.

- [ ] 2.2.1 Implement the JSON renderer, escaping, driver-error line, and format pre-scan — `compiler/src/diag.rs`
- [ ] 2.2.2 Implement `clc --diagnostics=json` and route its error prints — `compiler/src/driver.rs`
- [ ] 2.2.3 Implement `clorraine --diagnostics=json`: flag, help line, error routing, and compiler forwarding — `compiler/src/clorraine/cli.rs`, `compiler/src/clorraine/mod.rs`, `compiler/src/clorraine/build.rs`
- [ ] 2.2.4 Run the spec suite to green; on any failure STOP and fix the implementation, never the test

**Verify**: `make -C docker exec CMD="cargo test --test dx_tooling_spec_test"`; `make check`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-machine-readable-diagnostics.md) §Testing Requirements
**Objective**: Pin the exact line shapes, key order, escaping, and span-less diagnostics; record
the phase review evidence.

- [ ] 2.3.1 Write the Phase 2 implementation tests (key order, multi-byte span end, escapes, empty set, pre-scan) — `compiler/tests/dx_tooling_impl_test.rs`, `compiler/src/diag.rs`
- [ ] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- `--diagnostics=json` on both CLIs with the frozen schema
- Driver errors machine-readable with `code: null`
- Phase 2 implementation tests

**Verify**: `make -C docker exec CMD="cargo test --test dx_tooling_impl_test"`; `make check`

---

## Phase 3: Canonical TextMate grammar

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer (data move plus a completeness contract; no codegen surface)
> **Reasoning**: low — a file move, a data completion driven by the frozen EBNF, and one toolkit
> path

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-textmate-grammar.md) §Completeness contract · [AR-8, AR-9](00-ambiguity-register.md)
**Objective**: Pin the EBNF completeness scan, the path move, the grammar metadata, and the v1
surface tokens.

- [ ] 3.1.1 [spec-author] Write specification tests from ST-16 – ST-19 — `compiler/tests/dx_tooling_spec_test.rs`
- [ ] 3.1.2 Run the spec suite and record the red phase (expected: every case fails against the demo-owned grammar)

### Step 3.2: Implementation

**Reference**: [03-03](03-03-textmate-grammar.md) §Scope map
**Objective**: Move the grammar, complete it against the EBNF, and update the toolkit.

- [ ] 3.2.1 Move the grammar to the canonical path and point the demo toolkit at it — `editors/clorraine.tmLanguage.json`, `tools/demo/src/util/shiki.ts`
- [ ] 3.2.2 Complete the grammar (missing keywords, `as`, the reserved built-ins, `=>`, `freedBy` replacing `deleter`) — `editors/clorraine.tmLanguage.json`
- [ ] 3.2.3 Extend the demo fixture with the newly covered tokens and their scope assertions — `tools/demo/test/grammar.spec.test.ts`
- [ ] 3.2.4 Run the spec suite to green; on any failure STOP and fix the implementation, never the test

**Verify**: `make -C docker exec CMD="cargo test --test dx_tooling_spec_test"`; `make demo-check`

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-textmate-grammar.md) §Testing Requirements
**Objective**: Pin the pattern-extraction scanner's edge cases and the scope spot-checks; record
the phase review evidence.

- [ ] 3.3.1 Write the Phase 3 implementation tests (escaped backslashes in patterns, alternation boundaries, scope spot-checks) — `compiler/tests/dx_tooling_impl_test.rs`
- [ ] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- Canonical grammar under `editors/` with EBNF completeness
- Demo toolkit consuming the canonical file
- Phase 3 implementation tests

**Verify**: `make demo-check`; `make check`

---

## Phase 4: Batch close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: none new — the close-out runs the full verification over already-reviewed phases
> **Reasoning**: low — verification and documentation sync only

### Step 4.1: Final Verification & Documentation

**Reference**: [AR-11](00-ambiguity-register.md) · [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Run the full acceptance, sync the docs and the roadmap, and close the plan.

- [ ] 4.1.1 Run the full verification: `make check`, `make verify`, and `make demo-check`; record the evidence
- [ ] 4.1.2 Update `AGENTS.md` (the commands table with `--explain` and `--diagnostics=json`; the layout table with `editors/`)
- [ ] 4.1.3 Sync the roadmap (`PLAN-05` → Plan Created, linked, with the installer/LSP note) and close the plan

**Deliverables**:
- Full verification green
- AGENTS.md and roadmap updated
- Plan closed with its review evidence

**Verify**: `make check`; `make verify`; `make demo-check`

---

## Dependencies

```
Phase 1 (explain + registry help)
    ↓
Phase 2 (machine-readable diagnostics)
    ↓
Phase 3 (TextMate grammar)
    ↓
Phase 4 (close-out)
```

The three feature phases are independent in code but share `dx_tooling_spec_test.rs` and build on
the same CLI option-parsing paths, so they run in order.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed (28/28 tasks)
2. ✅ All verification passing (`make check`, `make verify`, `make demo-check`)
3. ✅ No warnings/errors in the workspace build
4. ✅ No dead code — no unused flags, helpers, or grammar patterns
5. ✅ Security addressed — the JSON escaper prevents stream corruption; no new input surface
6. ✅ Documentation updated (AGENTS.md commands and layout)
7. ✅ Code reviewed (the per-phase reviews recorded in the findings ledger)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
