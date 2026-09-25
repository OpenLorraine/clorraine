# Requirements: Clorraine v1 language completion

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The compiler MVP implements the corpus subset of the frozen specification and
reports a staged-feature code outside it. This plan implements the remaining
**v1** surfaces: generics (G1-G8), lambdas (L1-L5, stages 1-4), and volatile
access (T7) with the per-chip MMIO wrappers. It is the owning requirements
document; every requirement traces to a ledger/spec artifact or to a decision in
the [Ambiguity Register](00-ambiguity-register.md).

Two of the gaps are silent wrong-code holes, not just missing features: a
generic struct that stores `T` emits invalid C with exit 0, and the six volatile
intrinsics lower to the literal `0`. Both are fixed here (AR-4).

## Functional Requirements

### Must Have

| # | Requirement | Source |
|---|---|---|
| R1 | A generic struct that stores a type parameter monomorphizes: one concrete C struct per distinct instantiation, deduped by a canonical type key, with substituted field types and the symbol `Cl<Name>__<typekey>` | G1, G3, P11, Ch 07 §9; AR-8, AR-17, AR-24, AR-29, AR-30 |
| R2 | A type argument that does not satisfy a callable bound `F: func(...): R` is `E10303` | G6, G-1; AR-15 |
| R3 | A value of a callable-bound type parameter is callable-only: storing or returning it is `E10304` | G7, G-4; AR-15 |
| R4 | A generic aggregate instantiated with an ephemeral (closure) type argument is `E10306` | G7, G-8, L3; AR-16 |
| R5 | Instantiation recursion beyond the cap is `E10305`, and the cap covers generic-struct instantiation | G5, G-7; AR-18 |
| R6 | `export` on a generic declaration is `E10302` (verification of existing behavior) | G4, G-5; AR-15 |
| R7 | A capture-less lambda lowers to a lifted `static` function whose value is a plain function pointer matching `func(...)` | L1, L5, P7; AR-9, AR-12 |
| R8 | Duplicable captures copy; non-duplicable captures borrow read-only; writing a capture without `[borrow x]` is `E10402`; moving a capture is `E10401`; a `take` capture is `E10403` | L2, L-3..L-5; AR-13, AR-22 |
| R9 | A capturing closure is accepted where a generic callable-bound parameter is the target and is monomorphized into the instance; it is `E10400` where a concrete `func(...)` value type is required | L4, L5; AR-9, AR-10 |
| R10 | A capturing closure is ephemeral: binding it to a name (local, field, global, or array), returning it, or capturing it is `E10404` | L3, L-6; AR-11, AR-14 |
| R11 | The six volatile intrinsics type-check and lower to a C `volatile`-qualified access with no folding, caching, or reordering; access outside `unsafe` is `E10604` | T7, Ch 04 §11; AR-19 |
| R12 | A module-level `volatile let` emits a C `volatile` static object | T7, IH-3; AR-20 |
| R13 | A `std.amiga.custom` module wraps the volatile intrinsics with a safe API over the minimal normative register set; application code never writes a chip address or a volatile intrinsic | appendix-stdlib §6, appendix-amiga §4; AR-7, AR-28 |
| R14 | The previously excluded or skipped compile-fail goldens (`E10303`, `E10304`, `E10306`, `E10400`-`E10404`) are un-skipped and asserted; `E90002` is retired and its golden removed; the registry and `spec/check.sh` stay consistent | AR-21, AR-25, AR-32 |
| R15 | `AGENTS.md`, the affected spec chapters, and `plans/00-roadmap.md` reflect the implemented surface | AR-32 |

### Should Have

| # | Requirement | Source |
|---|---|---|
| R16 | Generic structs compose across modules and with nested generic arguments (for example `Pair<Pair<i32>>`), deduping nested keys | R1; AR-17 |
| R17 | A host runnable case exercises the volatile intrinsics over a real local address, and a hosted-amiga case builds and runs the `std.amiga.custom` wrapper | R11, R13; AR-27 |

### Won't Have (Out of Scope)

- Data inclusion (`embed`) — AR-35.
- `comptime` semantics — AR-36.
- Owning-element drop glue for `Array<move-only>`; the honest `E90000` staged
  error stays — AR-34.
- C-hook trampolines for capturing closures (stage 5, v1.1) — AR-37.
- The G5 opt-in instantiation size/count report — AR-33.
- ARM64 and the 64-bit `usize`/`isize` width — AR-38.
- Broader custom-chip coverage, `std.amiga.exec` message/signal APIs, and the
  library-build entry (MOD-29) — AR-39.

## Technical Requirements

### Performance

The compiler is a development tool, not a runtime. Monomorphization stays
whole-program and deduped, so generic-heavy programs emit one copy per distinct
instantiation.

### Compatibility

- Emitted C remains the strict common subset of Ch 17 §1 accepted by
  `m68k-amigaos-gcc` and `vbcc`: a closure is a plain C struct plus a function
  pointer; a volatile access is a C `volatile` dereference.
- The host profile keeps portable C99 plus libc; the MMIO module is
  hosted-amiga only (`Profile::supports_module`, `compiler/src/profile.rs:73`).
- Output stays deterministic: same source and flags produce byte-identical C
  (P10, P13).

### Security

- The compiler treats every input file as untrusted data; the lexer/parser never
  evaluates source content, and the volatile path emits a fixed access form.
- The MMIO wrappers confine `unsafe` and hardcoded chip addresses to the
  standard library; application code reaches them through safe functions
  (X6, T6).
- No new crates, no build scripts, no shell invocation, and no network access
  (the workspace stays std-only).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Cluster set | three v1 clusters / plus embed+comptime / one cluster | generics + lambdas + volatile/MMIO | Exactly the v1 surfaces the MVP staged out; keeps the plan bounded | AR-2 |
| Capturing-closure passing | callable generic bound / defining scope only / unbounded param | callable generic bound | Makes stage 4 coherent and gives captures a use without a heap environment | AR-9, AR-10 |
| Capturing-closure naming | inline-only / local binding allowed | inline-only | Matches the frozen `E10404` golden and Ch 17 §8's rationale | AR-11 |
| Generic-struct lowering | per-instantiation struct / byte blob | per-instantiation struct | The specification requires a concrete struct per storable instantiation | AR-17 |
| `Array<move-only>` | include drop glue / defer | defer | No corpus needs it; the honest staged error stays | AR-34 |
| Staged-code reconciliation | retire E90002 / re-scope | retire E90002, keep E90000 | N2 keeps removed codes reserved; E90000 still has the `Array` trigger | AR-21 |
| Volatile test legs | host + hosted-amiga / hosted only | both | Proves the access semantics on the fast profile and the wrapper on the target | AR-27 |

> **Traceability:** every scope decision cites the register entry that resolved
> it; see `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] Every Must Have requirement has at least one specification test case in
   `07-testing-strategy.md` and passes.
2. [ ] The previously excluded/skipped goldens (`E10303`, `E10304`, `E10306`,
   `E10400`-`E10404`) are asserted, `E90002` has no trigger, and
   `./spec/check.sh` is green.
3. [ ] `make check` and `make verify` are green end to end in the SDK container.
4. [ ] Emitted C for the new surfaces compiles under the hosted-amiga GCC and
   vbcc legs, and the MMIO wrapper runs under `vamos`/FS-UAE.
5. [ ] Same source and flags still produce byte-identical C (determinism suite).
6. [ ] `AGENTS.md`, the authorized spec chapters, and `plans/00-roadmap.md` are
   current.
