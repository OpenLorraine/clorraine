# Execution Plan: Generic Substitution Guards

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-26 22:40
> **Progress**: 10/13 tasks (77%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix DEF-12 with one focused audit over the monomorphization set the ownership
stage already collects: a substituted view field reports `E10205`, and a
substituted non-duplicable fill reports `E10215`, while concrete illegal cases
keep their single declaration-site diagnostic (AR-7). The audit lives in
`compiler/src/own/instance_check.rs` and reuses existing helpers; no other stage
changes (AR-10). Specification tests are written red before implementation, and
the legal instantiations are pinned by build-and-run controls (AR-15). The audit
reads only the recorded instance set; unrecorded discovery gaps stay out of
scope (AR-19).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1     | Instantiation-time audit | 10 |
| 2     | Full acceptance and closeout | 3 |

**Total: 13 tasks across 2 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of
> truth** for progress. Every task line appears exactly once in this document.
> The executing agent MUST:
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
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Instantiation-time audit

> **Phase baseline tree**: `3442b3af1a470919cfdfbc3b187f5db3e419ab94`
> **Expected modification set**: `compiler/src/own/instance_check.rs` (new),
> `compiler/src/own/generics.rs`, `compiler/src/own.rs`,
> `compiler/tests/ownership_drop_spec_test.rs`,
> `compiler/tests/ownership_drop_impl_test.rs` (strict scope)
> **Lenses**: correctness; domain lens: compiler and language (PF-007)

### Step 1.1: Specification tests (red)

**Reference**: [03-01](03-01-instantiation-checks.md) · [07](07-testing-strategy.md) ST-1..ST-14 · AR-3, AR-4, AR-7, AR-9, AR-17
**Objective**: Pin the instantiation diagnostics and the accepted controls before the audit exists.

- [x] 1.1.1 [spec-author] Write the view-field cases ST-1..ST-8 — `compiler/tests/ownership_drop_spec_test.rs` ✅ (completed: 2026-09-26 22:35)
- [x] 1.1.2 [spec-author] Write the fill cases ST-9..ST-14 — `compiler/tests/ownership_drop_spec_test.rs` ✅ (completed: 2026-09-26 22:35)
- [x] 1.1.3 Confirm the rejection cases FAIL (red) while the acceptance controls pass; record the run — `compiler/tests/ownership_drop_spec_test.rs` ✅ (completed: 2026-09-26 22:35)

**Deliverables**:
- ST-1..ST-5, ST-7, ST-9..ST-11, and ST-13 fail red; the ST-6, ST-8, ST-12, and ST-14 controls pass unchanged (ST-8/ST-14 are declaration-site controls, PF-004)

**Verify**: `make -C docker exec CMD="cargo test -p clc --test ownership_drop_spec_test"` shows the new rejection cases failing

### Step 1.2: Implementation (green)

**Reference**: [03-01](03-01-instantiation-checks.md) §Implementation Details · AR-2, AR-6, AR-18
**Objective**: Report `E10205` and `E10215` for the parameterized instantiations only.

- [x] 1.2.1 Add the shared `struct_declarations` helper, reuse it in `collect_struct_instances`, and raise `type_param_names` visibility — `compiler/src/own/generics.rs` ✅ (completed: 2026-09-26 22:37)
- [x] 1.2.2 Add the module with `check` and `check_view_fields`, and wire it into the stage — `compiler/src/own/instance_check.rs`, `compiler/src/own.rs` ✅ (completed: 2026-09-26 22:37)
- [x] 1.2.3 Add `check_fill_instances` and its statement/terminator value walk — `compiler/src/own/instance_check.rs` ✅ (completed: 2026-09-26 22:37)
- [x] 1.2.4 Confirm the Step 1.1 cases PASS (green); run `make check` ✅ (completed: 2026-09-26 22:37)

**Deliverables**:
- ST-1..ST-14 pass; the concrete cases are not double-reported; `make check` green

**Verify**: `make -C docker exec CMD="cargo test -p clc --test ownership_drop_spec_test"` and `make check` (exit 0)

### Step 1.3: Implementation tests and phase verification

**Reference**: [03-01](03-01-instantiation-checks.md) §Testing Requirements · AR-15
**Objective**: Cover the internal branches and prove the legal programs build and run.

- [x] 1.3.1 Add unit tests for the parameter-mention gate, a declaration-named nested instance, a multi-extent fill, a duplicable fill, and a value reached through a terminator — `compiler/src/own/instance_check.rs` ✅ (completed: 2026-09-26 22:39)
- [x] 1.3.2 Add the ST-15 and ST-16 build-and-run controls (plain build, no sanitizer) — `compiler/tests/ownership_drop_impl_test.rs` ✅ (completed: 2026-09-26 22:39)
- [x] 1.3.3 Full phase verification — `make check` ✅ (completed: 2026-09-26 22:40)

**Deliverables**:
- Unit tests and controls pass; no double-free abort in the control programs; `make check` exit 0

**Verify**: `make check` (exit 0)

---

## Phase 2: Full acceptance and closeout

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Expected modification set**: `plans/00-roadmap.md`,
> `plans/generic-substitution-guards/99-execution-plan.md`, `AGENTS.md` only if a
> factual note changed (strict scope)
> **Lenses**: regression, documentation

### Step 2.1: Acceptance and documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria · AR-12, AR-16
**Objective**: Prove the whole feature and record completion.

- [ ] 2.1.1 Full acceptance run — `make verify`
- [ ] 2.1.2 Trace R1–R8 to the passing ST cases and record the close-out evidence — `plans/generic-substitution-guards/99-execution-plan.md`
- [ ] 2.1.3 Mark DEF-12 Fixed on `plans/00-roadmap.md`, set this plan's row to Done and link it; update `AGENTS.md` only if a factual note changed

**Deliverables**:
- `make verify` green; every requirement traced to a passing case; roadmap and plan status updated

**Verify**: `make verify` (exit 0)

---

## Dependencies

```
Phase 1 (instantiation-time audit)
    ↓
Phase 2 (full acceptance)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per task; `make verify` final)
3. ✅ No warnings/errors; no dead code — one audit module, no unused helper
4. ✅ Security hardened — the fix rejects a double-free program and a view-field escape before codegen (AR-14)
5. ✅ No `spec/`, `stdlib/`, or corpus source changes (AR-12)
6. ✅ Documentation updated — roadmap DEF-12 fixed; plan done
7. ✅ Code reviewed (strict profile — independent review of the phase diff)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
