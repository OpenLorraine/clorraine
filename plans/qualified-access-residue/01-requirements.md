# Requirements

## Scope

The batch closes `DEF-397`, `DEF-398`, `DEF-399`, and `DEF-400`. Each filed
reproduction is an acceptance case. No frozen rule changes.

## Functional requirements

| ID | Requirement |
|----|-------------|
| R-S1 | A generic dot call whose type parameter is fixed by no receiver or argument reports `E10301` once and writes no translation unit. `func fail<T>(x: i32): T`, called as `1.fail()`. |
| R-S2 | A generic dot call whose concrete argument violates a declared callable bound reports `E10303` once, naming the type parameter, exactly as the direct twin does. `apply<T, F: func(T): T>(x: T, f: F)`, called as `1.apply(5)`. |
| R-S3 | A generic dot call that recursively instantiates itself beyond the limit reports `E10305`, and the explicit/inferred route keeps the accepted callable-bound control. |
| R-S4 | A qualified function value resolves to the emitted module symbol. `let g: func(): i32 = q.funcvalue.helper;` builds strictly and calls `helper`. |
| R-S5 | A qualified compiler intrinsic call is accepted: `std.mem.len(a)`, `std.mem.rawSlice<u8>(p, n)`, and `std.mem.rawStr(p, n)` type and emit the same intrinsic as the bare spelling, with the argument judgments of that intrinsic. |
| R-S6 | A cross-module `ffi` call resolves and checks its written count and `cstring` arguments through the target module's signature; the accepted call runs. |
| R-S7 | A module path wins over an imported value head: `spec.cc.qcoll.lib.f()` resolves the module function even when `spec` is an imported value. A lexical local of the same spelling still shadows the path. |

## Non-functional requirements

| ID | Requirement |
|----|-------------|
| R-N1 | Every rejected case reports exactly one occurrence of the expected code per site and writes no translation unit. |
| R-N2 | Every accepted control builds under the fixed strict flags and runs ASan/LSan clean. |
| R-N3 | Accepted controls re-emit byte-identically. |
| R-N4 | `make check` and `make verify` pass at the batch head. |
| R-N5 | No `spec/` oracle, registry entry, or generated C is edited. |

## Acceptance criteria

- `DEF-397`'s `fail`/`apply` shapes report `E10301`/`E10303` once; the accepted
  callable-bound dot call still runs.
- `DEF-398`'s qualified function value builds and runs.
- `DEF-399`'s three qualified intrinsics and the cross-module `ffi` call build
  and run.
- `DEF-400`'s imported-head collision resolves the module function; the lexical
  shadow control still shadows.
- The focused suites and the full workspace suite are green.
