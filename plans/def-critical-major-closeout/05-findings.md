# Findings Ledger: Critical/Major DEF Closeout

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| — | — | — | Awaiting execution | — |

## Preflight Rulings

The iteration 1 preflight report (`00-preflight-report.md`) records 28 findings
(6 critical, 16 major, 6 minor) and the delegated technical resolutions. The
user ruled the reserved items on 2026-10-24:

| Ref | Ruling | Date |
|---|---|---|
| R-1 / PF-001 | Add `DEF-458` to Batch B; the active set is 34 (1 critical + 33 major), 35 DEF rows including `DEF-470` | 2026-10-24 |
| R-2 / PF-003 | Add a parse-only/stage-select mode for the `DEF-167` stage test; the full compile reports E10300 | 2026-10-24 |
| R-3 / PF-006 | Close `DEF-416` by the conservative collision report (E11101), not module-qualified identity | 2026-10-24 |
| R-4 / PF-021 | Restore the roadmap row evidence/header; phase closes go through the roadmap skill | 2026-10-24 |
| R-5 / PF-018 | Revalidate `DEF-506`/`507`; use the conservative proof; a summary surface needs a new ruling | 2026-10-24 |
| AR-22 | Apply the delegated fixes; the plan documents are corrected | 2026-10-24 |

## Iteration 2 Preflight Rulings

Preflight iteration 2 (2026-10-24) verified the iteration-1 corrections and
found residual and new defects. The report `00-preflight-report.md` carries the
full evidence. The applied corrections are:

| Ref | Ruling | Date |
|---|---|---|
| PF-002 | Add the constant-context flag; a module `const embed` reports only `E10224` | 2026-10-24 |
| PF-003 | The full compile of the filed local-i32 stage shape reports one `E10300`; keep the parse-only stage mode | 2026-10-24 |
| PF-004 | Put the source set on `PlaceState`; strong-update on assignment, join in `merge`, prune with the place; keep the destination-root, take, and type-parameter rules | 2026-10-24 |
| PF-005 | Test source-vs-destination containment; clear the taken subrange of the save copy; attach the range to the capture | 2026-10-24 |
| PF-006 | Report the `DEF-416` collision from `sema` with a pure `TypeTable` query, not the table constructor | 2026-10-24 |
| PF-008 | The nullable conditional loop head is a mandatory `E10012` case | 2026-10-24 |
| PF-010 | Complete the remaining ST sources, unsafe contexts, exit values, and strict-C assertions | 2026-10-24 |
| PF-011 | Pin the `DEF-522` arm backing and the exact `E10030` local-arm code | 2026-10-24 |
| PF-013 | Define the must-source walk for the cross-frame scan | 2026-10-24 |
| PF-017 | Thread the `DEF-472`/`473` destination through `statements.rs`, `place.rs`, and `mono.rs` | 2026-10-24 |
| PF-019 | Extend the test-impact inventory and add a retarget line to each phase close | 2026-10-24 |
| PF-023 | Require a struct declaration order that differs from the source order in ST-20 | 2026-10-24 |
| PF-029 | Allow the `DEF-506`/`DEF-507` stale green pin in R6 and the acceptance criteria | 2026-10-24 |
| PF-030 | Build the probe binary before the Phase 0 evidence probe; author the source-set pin after the field exists | 2026-10-24 |
| PF-031 | Use the pinned destination index and the run-time source shape in ST-9 | 2026-10-24 |
| PF-032 | Add the four new test pairs to the AGENTS.md test-naming map | 2026-10-24 |
| R-6 / AR-23 | User chose **follow `spec/04` §16**: a module-level `let embed` reports `E10213`; the `spec/13` §1 sentence is recorded as a frozen inconsistency; no chapter change | 2026-10-24 |

## Red-Phase Evidence

Every active row records one line per ST case before its fix:

| ST | DEF | Command | Base commit | Observed failure |
|---|---|---|---|---|
| ST-N | DEF-N | `cargo test --test def_*_integrity_spec_test <name>` | `<sha>` | `<diagnostic, ASAN/LSan, strict-C, or trace>` |

ST-12 and any AR-16 stale close record the HEAD command and the accepted result
instead of a failure.

## Phase 0 — Revalidation and Baseline

**Baseline (2026-10-25):** `cargo build --workspace` passed; `make check` passed
(spec check plus the full Rust workspace); `make verify` passed every leg after
refreshing the gitignored `docker/.env` XAUTHORITY cookie for the FS-UAE leg
(environment-only change, not a repository artifact). Phase baseline tree
`bdf231350a5e3371a40a524c0f9dc5ddc0d6ee55`.

**Roadmap check:** all 34 active rows link to this plan, keep their
severity/evidence notes, and `DEF-470` is Done. The 34 rows moved from
`Plan Preflighted` to `Executing` at phase start.

**Active count:** 34 active rows (1 critical + 33 major) plus the stale
`DEF-470` close = 35 DEF rows. 35 ST rows and 76 tasks.

### Evidence probes

| DEF | Filed shape | Phase 0 result |
|---|---|---|
| DEF-543 | `func fill(p: str, c: bool, borrow slot: str?) { let s: str = p; if (c) { s = "AA"; } slot = s; }` called same-frame | Accepted; strict C builds — defect reproduces |
| DEF-167 | `let call_shaped = a < b > (c);` with i32 locals | Accepted; emits `call_shaped = a(c);`; strict C fails — defect reproduces |
| DEF-171 | `let data: u8[16] = embed("assets/logo.bin");` in a function body | Accepted; emitted C fails strict C — defect reproduces |
| DEF-470 | `take h: H; let r: String? = c ? h.name : null; let s = h.name;` | One `E10020` — stale, close with the HEAD proof |
| DEF-506 | `struct H { name: String; tag: static str; }`; `viewTag(borrow h: H): str { return h.tag; }` | One `E10036` at the callee return — reproduces; not stale |
| DEF-507 | `always(xs: str[2]): str { return "ZZ"; }` then `return always(xs)` | Accepted; strict C builds — stale, close with the HEAD proof |
| DEF-458 | `v = (g \|\| eat(take v) == 2) ? x : y` with `g = true` | Iteration-1 HEAD probe: accepted; 11-byte LSan leak. The code is unchanged at the same HEAD |

The remaining 28 active rows (`DEF-411`, `412`, `413`, `415`, `416`, `454`,
`455`, `456`, `457`, `460`, `461`, `462`, `464`, `465`, `466`, `471`, `472`,
`473`, `475`, `476`, `477`, `478`, `479`, `480`, `481`, `505`, `513`, `522`)
carry the iteration-1 HEAD evidence and the roadmap's filed shapes. Each batch's
specification task re-confirms its rows red before the fix; a row that no longer
reproduces is closed stale under AR-16 with a HEAD proof.

**Stale-row decisions:** `DEF-470` and `DEF-507` close stale with the proofs
above. `DEF-506` stays active; its safe shape is not accepted today.

### Existing-test inventory

| Family | Expected change | Owning phase |
|---|---|---|
| `stage_spec_test.rs` | `comparison_and_generic_call_forms_parse` uses the new parse-only mode | 5 |
| `compile_fail_spec_test.rs` | E10213 moves from `EXCLUDED_CODES` to `ASSERTED_CODES`; E10214 stays excluded | 5 |
| `checker_conformance_spec_test.rs` (`qinitstructname`) | moves from `emit_pair` success to the `E11101` rejection | 5 |
| `borrow_generic_integrity_spec_test.rs` (`generic-mixed`/`-local`) | report counts and any accepted controls change with DEF-472/473 | 3 |
| `borrow_store_integrity_*` | out-slot controls gain the source-set judgments | 1 |
| `conditional_projection_integrity_*`, `conditional_projection_residue_*` | fresh-root, per-arm, and read/return pins may change | 3/4 |
| `store_emission_integrity_*`, `call_value_integrity_*` | conditional store and guard shapes may change | 2/3 |
| `runtime_value_integrity_*`, `eval_emission_integrity_*` | release and evaluation-order pins may change | 2/3 |
| `module_initializer_integrity_*`, `callable_release_integrity_*` | initializer and callable C pins may change | 3/4 |
| `own`/`sema` unit tests | source-set, conditional, and type-argument internals | 1/3/5 |

Each phase close retargets the tests in this inventory and records every
expected change.

## Phase Records

Execution adds one section per phase with the revalidation results, review
findings, and the final verification evidence. No row is marked Done without a
current-HEAD reproduction or a documented stale proof.
