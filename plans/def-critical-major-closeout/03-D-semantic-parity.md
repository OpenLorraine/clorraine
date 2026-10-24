# Semantic Parity: Critical/Major DEF Closeout

> **Document**: 03-D-semantic-parity.md
> **Parent**: [Index](00-index.md)

## Overview

Batch D closes nine generic/null/conditional rows where the declaration-time
judgment or the instance audit diverges from the concrete twin. The default is
to match the concrete twin and the frozen spec: accept the safe spelling, reject
the unsafe one once, and emit valid C for an accepted program.

## Architecture

### Current Architecture

- `compiler/src/own/instance_check.rs` substitutes recorded instances into the
  move, slot, projection, and element duties.
- `compiler/src/own/call_check.rs` already scans a callee body for the boolean
  whole-out-slot store; a viewed-position or return-source scan is a new,
  bounded helper, not the existing scan.
- `compiler/src/own/expr.rs`/`place.rs` own return elision, view provenance, and
  alias roots; `own/statements.rs` owns the declaration-time E10035/E10036
  sites.
- `compiler/src/sema/ops.rs` owns the conditional null rule; `types/mod.rs`
  holds the predicates; `own/loops.rs` owns the loop-head null report.
- `compiler/src/types/table.rs` keys types by simple name; `DEF-416` is closed
  by the Batch E collision report (AR-20), not a type-identity change.

### Proposed Changes

| DEF | Change | Existing helper |
|---|---|---|
| 411 | Split the contexts: bool `if` → E10012; i32 `if` → E10218; i32 cast/binary/annotated let → E10012; unannotated let → a positive nullable control; a nullable conditional loop head → E10012 (the existing null rule, not an optional case) | `sema/ops.rs` null rule, `own/loops.rs` loop head |
| 412 | Resolve the substituted member for a view-array return; remove the false `E10036`; keep the local-backed/duplicate-candidate safety pins | `own/place.rs`/`own/statements.rs` return duty |
| 413 | Resolve a nested element through the instance instead of emitting `#element of T` | iterable/element substitution |
| 415 | Report one E10010 **per nullable arm** for a same-arm nullable conditional at a non-nullable `str` destination | conditional value check |
| 505 | Split: duplicable nested projection accepted (no E10200, no duty); owning nested projection rejected once with the concrete twin's E10026/E10034; add the `h.inner.name` field form | projected-place duty |
| 506 | Revalidate at HEAD; if the exact filed shape still rejects, accept only with a conservative no-escape proof; otherwise close stale | new bounded viewed-position scan |
| 507 | Revalidate at HEAD; the documented safe shapes are already accepted, so close stale unless the exact filed shape still rejects | return-source scan or `body_return_flow_names` |
| 513 | Resolve an alias return over an inline iterable/temporary and report the escape | `own/expr.rs` + `own/loops.rs` alias-root/return walk |
| 522 | Accept the safe move **and read/return** view-member projection; pin the arm backing; reject a local-backed escape | substitution + provenance + view-member emission |

### Precision Without a Summary Subsystem

`DEF-506` and `DEF-507` are the only rows that could tempt a general summary.
The plan explicitly rejects that. `call_check.rs` has only the boolean
whole-out-slot scan; the new work is a **bounded** viewed-position scan (506)
and a return-source check (507), not a cross-function summary store:

- 506 must prove that every direct store/return of the slot is provably static
  and that no indirect call, lambda, or return can pass the view; otherwise the
  conservative rejection stays and the row is not closed.
- 507 is revalidated first; the documented `always(xs)` safe shapes are already
  accepted at HEAD, so the row is closed stale under AR-16 unless the exact
  filed shape still rejects.

If the direct scan cannot prove the safe case without a new cross-function
summary store, the executor stops under AR-12 and the Complexity Escalation
Gate rather than inventing one; the row stays open until the user rules.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Null arm at a non-nullable context | The context's own rule: E10012 for a bool/annotated let/cast/binary; E10218 for an i32 `if`; E10012 for a nullable conditional loop head; no C | AR-5 |
| Same-arm nullable conditional at `str` | One E10010 per nullable arm (two for ST-26) | AR-5 |
| Safe substituted view-array return | Accept; no `E10036`; keep the local-backed/duplicate-candidate pins | AR-5 |
| Unknown nested element | Resolve through the instance; no placeholder | AR-5 |
| Duplicable nested projection | Accepted; no E10200 and no duty | AR-5 |
| Owning nested projection | One concrete-twin rejection (E10026/E10034); no E10200 | AR-5 |
| Safe static-only viewed position | Accept only with the conservative no-escape proof; otherwise keep the rejection | AR-12 |
| Static-return helper | Revalidate; close stale if the documented safe shape is already accepted | AR-12, AR-16 |
| Inline iterable alias return | Report the concrete escape once | AR-5 |
| Safe view-member move/read | Accept and emit valid C; a local-backed escape still rejects | AR-5 |

## Testing Requirements

- One specification case per DEF with the concrete twin as the oracle.
- `DEF-411` is split per context; the nullable conditional loop head is a
  mandatory E10012 case (it is in the filed row); `DEF-415` reports one E10010
  per arm.
- `DEF-505` has an accepted duplicable case and a rejected owning case; the
  `h.inner.name` field form is pinned.
- Accepted controls run exit 0 and are ASAN/LSan clean.
- Rejected cases produce the concrete twin's diagnostic count (one per site or
  per arm) and no translation unit.
- `DEF-506`/`507` include the unsafe twin and the revalidation result; a stale
  row gets a green HEAD pin and a `05-findings` proof.
- `DEF-522` pins both the move/store and read/return forms, the arm backing, and
  the accepted-vs-concrete emitted C.
- `DEF-412` cites the existing E10035/E10036 suite as its safety pin or adds the
  local-backed/duplicate-candidate twins.
