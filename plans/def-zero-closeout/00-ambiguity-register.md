# Ambiguity Register: Zero-Defect Closeout

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Status**: ✅ GATE PASSED — 2026-10-28
> **Scope**: the 114 open DEF rows in `plans/00-roadmap.md` (5 critical, 9 major, 100 minor)

## Gate Contract

Every row is either ✅ Resolved with a recorded decision and owner, or ⏸ Deferred with an explicit user approval. No item is silently deferred. The user confirmed the scope and mode on 2026-10-28; the three spec/registry rulings and the DEF-557 complexity approval were confirmed on 2026-10-28. All other rows are delegated technical resolutions under `--auto-design` and are grounded in the frozen `spec/` or the row's recorded fix direction.

| AR | Kind | Decision | Status | Owner |
|---|---|---|---|---|
| AR-1 | Scope | Close all 114 open DEF rows; no feature work; the `05-findings.md` pre-existing non-DEF list stays out of scope. | ✅ Resolved | User 2026-10-28 |
| AR-2 | Mode | Execute with `--auto-commit --auto-design`; critical/major or spec/design decisions still stop for the user. | ✅ Resolved | User 2026-10-28 |
| AR-3 | Spec | `len(cstring)` is supported and lowers to the runtime C-string length; `len` on `usize`, `u8`, and other scalars reports `E10200`. | ✅ Resolved | User 2026-10-28 |
| AR-4 | Spec | Add `E10309` to `spec/14-diagnostics.md` for a call whose callee is not a function or callable value. | ✅ Resolved | User 2026-10-28 |
| AR-5 | Complexity | Authorize the bounded instance-bound static-return marker needed by `DEF-557`; it is the only new cross-function support surface in this plan. | ✅ Resolved | User 2026-10-28 |
| AR-6 | Semantics | `DEF-190` reports `E10010` naming the call spelling (for example `value 'get()' may be null here`); the existing registry template already carries the name slot. | ✅ Resolved | auto-design, spec TY-6/E10010 |
| AR-7 | Semantics | `DEF-191` drops the `unsafe` exemption at the index and store-index sites to match TY-6, which lists no exemption. | ✅ Resolved | auto-design, spec TY-6 |
| AR-8 | Semantics | `DEF-193` narrows through the right operand of `&&`/`||` and the guarded body once the left comparison succeeded; a loop-head comparison narrows the body. | ✅ Resolved | auto-design, spec TY-6/EX-3 |
| AR-9 | Semantics | `DEF-263` evaluates a folded `len` receiver's effects once, then uses the constant extent. | ✅ Resolved | auto-design, spec EX-3/GF14 |
| AR-10 | Process | Phase 0 may close a row as stale or incidentally fixed only with a HEAD command and the accepted result recorded in `05-findings.md`. | ✅ Resolved | plan policy |
| AR-11 | Order | Risk-first batches: critical/major backlog, null/types, calls/resolution, borrow/loans, emission/ABI, initializers/carriers, then the full close. | ✅ Resolved | auto-design, risk order |
| AR-12 | Verification | `make check` + `make verify`; focused `RDC_ASAN=1` for memory rows and strict `-Werror` C for emission rows; every phase closes green. | ✅ Resolved | AGENTS.md |
| AR-13 | Spec boundary | Only the authorized registry additions (`E10309`, the `E10010` call spelling), the `len` surface notes, and the `DEF-557` marker documentation change `spec/`; no golden changes without a new ruling. | ✅ Resolved | user + auto-design |
| AR-14 | Discovery | A Phase 0 row that needs an unauthorized spec change or a new material surface stops the phase for a user ruling; it is never implemented silently. | ✅ Resolved | gate policy |
| AR-15 | Testing | One ST case per row; new per-batch `*_spec_test.rs` / `*_impl_test.rs` pairs; existing per-file compile/run/ASAN/strict-C helpers; no new shared harness. | ✅ Resolved | AGENTS.md |
| AR-16 | Scope | A confirmed row is fixed in the owning stage; no generalized analysis framework; the DEF-557 marker is the only approved new surface. | ✅ Resolved | auto-design |
| AR-17 | Done | The plan is complete only when `plans/00-roadmap.md` has zero open DEF rows (546/546 Done) and the final reviews are clean or recorded. | ✅ Resolved | user goal |
| AR-18 | Docs | Roadmap/ledger state is closed through the roadmap skill; plan progress stays in `99-execution-plan.md`. | ✅ Resolved | AGENTS.md |

## Deferred Items

None.

## Gate Result

✅ GATE PASSED — all 18 items resolved with a named owner and evidence; no deferred item is implemented by this plan.
