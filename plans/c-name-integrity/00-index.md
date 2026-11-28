# C Name Integrity Implementation Plan

> **Feature**: Emit valid C whatever the source names are: escape source names that C keywords, reserved identifiers, or header and shim names would break; keep generated instance and helper symbols apart from user symbols; give every emitted region a true `#line`; keep absolute paths out of driver-built C
> **Status**: Plan Preflighted
> **Created**: 2026-11-28
> **Implements**: none (standalone plan; no RD) — closes DEF-589, DEF-592 and theme issue #42 (findings #61, #64; ASM-WS3-005 and ASM-WS3-006 in #91)
> **CodeOps Artifact Schema**: 1

## Overview

An accepted Clorraine program can still emit C that no compiler on the matrix accepts. A local,
parameter, field, capture, or enum constant spelled like `restrict`, `_Bool`, `long`, `NULL`,
`EOF`, `errno`, `MEMF_CHIP`, `textPen`, or `memcpy` either is a C keyword, is rewritten by a
header macro, or shadows a name the generated code calls. A module function can also spell the
same C symbol as a generic instance (`module get; func _Point` and `get<Point>` both become
`cl_get__Point`) or as a generated per-type helper (`cl_box_alloc__i32`, `cl_zero_ClRec`).

Two tooling defects sit beside them. The emitted C only writes `#line` before Clorraine
statements, so the generated entry stub, each function's header, and about 5,000 appended runtime
lines on hosted-amiga are attributed to an unrelated `.clo` line. And the `#line` path is the path
as given, so an editor build, which passes absolute paths, writes the user's home directory into
the C.

The plan escapes source names through one injective suffix rule backed by a committed list of
every name the supported toolchains' headers and the shim define (AR-5, AR-18, AR-19), emits enum
constants as their values so no enum macro reaches C (AR-16 as amended by preflight), reserves
generated symbols so the existing `__user` rename applies (AR-20), maps every emitted region to a
real source line or runtime file (AR-21), and makes the `clorraine` driver pass sources relative to
its working folder (AR-10, AR-24). No spec chapter or spec test changes.

## Minimum-Sufficient Baseline

**Original goal:** Issue #42: "Escape every C99 keyword and header/NDK macro name and separate
generic-instance symbols from module functions", closing ASM-WS3-001, -002, -005, -006.
**Smallest viable design:** one escape predicate and one name-only suffix function used by every
source-name site, and enum constants emitted as values (03-01); the existing `__user` rename with
a wider reserved set and one precomputed instance map (03-02); a `#line` before each top-level
item and each appended runtime file (03-03); a path rewrite in the driver (03-04). It reuses the existing uniqueness counter, reserved-symbol scan, `__user`
loop, skip-without-toolchain test pattern, and `platform.rs` path helper.
**Excluded machinery:** committed generator tooling for the name list (a one-off recorded command
instead, AR-7); `-fdebug-prefix-map` (a spec/17 §13 change, AR-22); a new instance prefix (AR-8).
**Approved complexity:** None.

## Planning Coverage

> **Coverage**: 4/4 clusters · 12/12 categories · 22 candidates · 9 residual limits · Effort: high/xhigh→unavailable · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-28 01:55; artifacts the register (AR-1..AR-14 at dispatch), issue #42, the WS3 findings and probes, the emitter, profile, shims, driver

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | name positions, `#line` regions, user text | every `Item` kind, emission order, `#line` writers, driver ICE text, docs and site; probes | vbcc predefines; a running hook program | 3 (→ AR-15/16, AR-21; one optional omitted) | general agent with the preflight-auditor brief | high |
| C2 Behavior and edges | field, capture, instance, helper, fallback sites | 10 probes; pins in `compiler/tests` | vbcc/freestanding extraction; stdlib pin blast radius | 7 (→ AR-15..AR-21) | general agent with the preflight-auditor brief | high |
| C3 Technical and data | prelude identity, name counts, vbcc, suffix layout | measured preludes and counts per leg | Windows headers; exact typedef counts | 6 (→ AR-15/16, AR-18, AR-19, AR-21) | general agent with the preflight-auditor brief | high |
| C4 Risk and stakeholders | determinism, size, debug info, rc24, licensing, AR-10 | SDK vs rc24 headers; DWARF probe; bench guards; VS Code tasks | legal merit; gdb stepping; lookup cost | 4 (→ AR-19, AR-22, AR-23, AR-10/24) | general agent with the preflight-auditor brief | high |

**Source note:** per-row CV mapping in [the register's coverage map](00-ambiguity-register.md#coverage-map). No Discovery keys.

## Document Index

| #     | Document                                                    | Description                                              |
| ----- | ----------------------------------------------------------- | -------------------------------------------------------- |
| AR    | [Ambiguity Register](00-ambiguity-register.md)              | Zero-Ambiguity Gate decisions (audit trail)              |
| 00    | [Index](00-index.md)                                        | This document                                            |
| 01    | [Requirements](01-requirements.md)                          | Scope and acceptance criteria                            |
| 02    | [Current State](02-current-state.md)                        | Today's naming, reservation, `#line`, and path code      |
| 03-01 | [Source names](03-01-source-names.md)                       | Escape classes, the name list, the spelling, every site  |
| 03-02 | [Generated symbols](03-02-generated-symbols.md)             | Instance and helper reservation, rename loops            |
| 03-03 | [Line mapping](03-03-line-mapping.md)                       | `#line` for every emitted region, suffix pieces          |
| 03-04 | [Driver paths and docs](03-04-driver-paths-and-docs.md)     | Path rewrite, docs, notices, residual limits             |
| 07    | [Testing Strategy](07-testing-strategy.md)                  | ST cases and verification                                |
| 99    | [Execution Plan](99-execution-plan.md)                      | Phases and task checklist                                |
| PF    | [Preflight Report](00-preflight-report.md)                  | Post-creation audit and its resolutions                  |

## Quick Reference

### Usage Examples

```clorraine
struct Gps { lat: i32; long: i32; }          // field `long` → C member long_0
enum INT : i32 { MIN, MAX, }                 // INT.MAX is emitted as (1); no INT_MAX macro
func f(EOF: i32, restrict: i32): i32 {       // parameters EOF_0, restrict_0
    let memcpy: i32 = EOF + restrict;        // local memcpy_0; the array copy still calls memcpy
    return memcpy;
}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Escape classes | C99 keywords, reserved identifiers, names the unit defines or declares (AR-5) |
| Enum constants | emitted as parenthesized values; no `#define` (AR-16, amended by PF-001) |
| Spelling | `n + "_0"` when `n` or its `_<digits>`-stripped root is escaped (AR-18) |
| Name list | one committed union over every leg and toolchain, completeness-tested (AR-19) |
| Generated symbols | wider reservation, existing `__user` rename, one instance map, exact-set loops (AR-20) |
| `#line` | every definition and runtime file mapped; suffix split into named files (AR-21) |
| Paths | the driver passes sources relative to its working folder; `clc` unchanged (AR-10, AR-24) |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| emission-integrity-auditor | reviewer (existing) | every phase: base/head corpus emission diff and strict C build hunt | AGENTS.md dispatch |
| memory-safety-auditor | reviewer (existing) | every phase (emission changes) | AGENTS.md dispatch |
| amiga-platform-reviewer | reviewer (existing) | Phase 1 (m68k/vbcc/rc24 name extraction) and Phase 4 (hosted-amiga suffix pieces) | AGENTS.md dispatch |

**No new specialist.** The capability-gap check found no gap: the codegen and Amiga reviewers
already exist and the name-list mechanics fit a dynamic packet.

_Detection evidence: `.opencode/agents/emission-integrity-auditor.md`, `memory-safety-auditor.md`,
`amiga-platform-reviewer.md` exist; AGENTS.md "Phase-review dispatch" requires them for IR,
codegen, symbol-name, and hosted-amiga shim changes; no repeated finding area outside their
checklists in recent `05-findings.md` ledgers._

## Related Files

- `compiler/src/codegen.rs` — escape classes, source-name entry point, reserved set, rename loops
- `compiler/src/{ir,own}.rs`, `compiler/src/own/lower.rs` — the generated flag on `ir::Local`
- `compiler/src/sema/mod.rs`, `compiler/src/types/mod.rs` — lambda and hook spans
- `compiler/src/codegen/c_names.txt` (new) — the committed name list
- `compiler/src/codegen/{expr,drop,guards,program,function,calls,cleanup}.rs` — field, capture, enum, instance, and `#line` sites
- `compiler/src/profile.rs` — a suffix-pieces accessor beside the unchanged `shim_suffix()`
- `compiler/src/clorraine/{build,run,cli,platform}.rs` — the driver path rewrite
- `compiler/tests/c_name_integrity_{spec,impl}_test.rs` (new)
- `compiler/tests/compiler_robustness_impl_test.rs`, `codegen_ownership_impl_test.rs`, and other impl pins (AR-25)
- `docs/known-limitations.md`, `site/guide/known-limitations.md`, `THIRD_PARTY_NOTICES.md`
