# Current State: Zero-Defect Closeout

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Baseline

| Item | Value |
|---|---|
| Roadmap | 432/546 DEF rows Done; 114 open (5 critical + 9 major + 100 minor) |
| HEAD at plan creation | `54197ce` (main, pushed) |
| Toolchain | Docker SDK container; `make check` and `make verify` |
| Frozen spec | `spec/`; the plan's authorized changes are AR-3, AR-4, AR-13 |
| Existing machinery | ownership/place state, per-instance duties, conditional materialization, strict-build suites |

## HEAD Revalidation Probes

The five decision-bearing rows were probed at HEAD `54197ce` with the
`clorraine emit` driver:

| Row | Probe | Observed |
|---|---|---|
| DEF-190 | `func get(): String?`; `let g: str = get();` | Accepted (exit 0); the nullable call result widens with no test |
| DEF-195 | `let u: usize = 3; len(u)`; `len(toCString("ab"))` | Accepted; both emit the constant `0` |
| DEF-214 | `let n: i32 = 1; n();` | Accepted; the emitted C calls a non-function (strict build fails) |
| DEF-483 | `let u: u8 = 7; len(u)` | Accepted; emits `0` |
| DEF-557 | `viewTag<T>(borrow h: T): str { return h.tag; }` over `tag: static str` | Rejected `E10036`; the concrete twin is accepted |

`DEF-544`…`DEF-555` and the remaining minors were last confirmed by the
`def-critical-major-closeout` phase reviews; Phase 0 revalidates every one at
HEAD before any fix.

## Batch Mapping

| Phase | Rows |
|---|---|
| 1 | DEF-544…DEF-557 (14) |
| 2 | DEF-190…DEF-195, 246…249, 383, 414, 417, 420, 467, 483, 542 (16) |
| 3 | DEF-169, 173, 174, 198, 203, 205, 212, 214, 236, 239…241, 244, 254, 349, 382, 387…389, 404, 405, 419 (22) |
| 4 | DEF-250, 252, 311, 312, 354, 370, 422, 434, 444, 463, 474, 484, 490…492, 494, 524, 530 (18) |
| 5 | DEF-194, 199…202, 206, 208, 217, 218, 221…228, 234, 253, 263, 313, 320, 324…328, 330, 334, 352, 355, 358, 392, 393, 438, 482, 500 (37) |
| 6 | DEF-210, 314, 316…319, 321 (7) |

## Patterns To Reuse

- Conditional store/take integrity: `codegen/function.rs` capture and
  self-alias machinery; ASAN pins in `borrow_store_integrity_*` and
  `conditional_projection_residue_*`.
- Per-instance duties: `own/instance_check.rs` and `ir/mono.rs`
  `Requires*` records; the DEF-557 marker extends this pattern.
- Null/narrowing: `own/expr.rs` `check_nonnull_expr`/`check_value_into`, the
  sema arm records, and the `def_semantic_parity_integrity_*` suite.
- Emission/order: `codegen/function.rs` materialization and guards; the
  `call_value_integrity_*`, `store_emission_integrity_*`, and
  `eval_emission_integrity_*` suites.
- Resolution/diagnostics: `sema/call.rs`, `sema/mod.rs`, `types/table.rs`,
  and `checker_conformance_spec_test.rs`.

## Risks

| Risk | Mitigation |
|---|---|
| A row is stale or incidentally fixed | Phase 0 revalidates at HEAD; close only with evidence |
| A minor row hides a spec change | AR-14 stops the phase for a user ruling |
| Test families grow too large | One spec/impl pair per batch; cases stay behavior-named |
| The DEF-557 marker grows into a summary subsystem | Bound it to the instance's static-return proof; no general store |
| Existing pins conflict with a corrected rule | Retarget only with the frozen-spec justification recorded in 05 |

## Verification Commands

`make check` and `make verify` from the repository root. Focused probes use
`make -C docker exec CMD="cargo test --test <suite>"` and `RDC_ASAN=1` for the
memory rows.
