# Current State: C Name Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

Measured on `main` at `7f63bb3c` (2026-11-28); line numbers are from that commit.

## Existing Implementation

### What Exists

- **Local and parameter names.** `sanitize_identifier` (`compiler/src/codegen.rs:1220-1234`)
  replaces invalid bytes and prefixes `cl_` when `is_c_keyword` (`:1237-1246`) matches: the C89
  keywords plus `inline` and `bool`. `unique_local` (`:848-858`) adds `_<n>` until the name is
  unused in the function; `seed_generated_names` (`:866-872`) reserves `__env`, `__cl_ret`, and
  `__cl_tmp<N>`; `unique_param_names` (`:941-970`) runs the same pass for parameter lists, so the
  signature and body agree. About 60 generated names (`__cl_*`) also go through `unique_local`.
  The per-function locals table (`function.rs:197-203`, lambdas `:296-304`) also holds the
  generated owners `__cl_owned<N>`: `declare_generated_local` marks the binding generated
  (`own.rs:1689-1701`), but `ir::Local` carries only `name` and `ty` (`ir.rs:616-623`), so the
  flag is dropped where the list is built (`own.rs:1250-1259`, `own/lower.rs:1812-1815`,
  `ir.rs:776-780`). The `path()` fallback `sanitize_identifier(first)` (`expr.rs:627-631`) also
  spells `ffi` function values and generated paths such as `cl_hooktramp_<id>`
  (`own/call.rs:628`).
- **Fields and captures.** Struct members are emitted with their source names: the definition at
  `codegen/program.rs:886-889` from the names built at `:954`. They are read and written through
  raw text at many sites, among them `ValueKind::Member` (`expr.rs:66-70`, `.` and `->`),
  `Projection::Field` (`expr.rs:205-210`), the dotted paths (`expr.rs:583-636,711-735`), drops
  (`drop.rs:95,236`), guards (`guards.rs:1564,1639`), `offsetof` (`calls.rs:494,582`), and
  `program.rs:1468,1551,2781`, `function.rs:3148`; the list is not exhaustive. Closure
  environments take each capture's source name (`program.rs:746-753`, reads at
  `expr.rs:265-267,1380-1388,1508-1514`, `function.rs:1626-1633`). Initializers are positional,
  so they carry no member names.
- **Enum constants.** `#define {Enum}_{member} {value}` (`program.rs:999-1001`), used raw at
  `expr.rs:584-585` (the only use site); emitted after the prefix shim and, on hosted-amiga,
  before the suffix. The enum name is not module-qualified, so `enum A_B { C }` and
  `enum A { X, B_C }` both define `A_B_C`; and the macros rewrite any later token of that
  spelling, including struct members and locals inside the suffix (`io_Command`, `font_size`).
  spec/17 §7 says C macros are not used; §3 asks only for "an emitted constant". The standard
  library declares no enums.
- **Fixed prelude helpers.** The emitter writes `cl_str`, `cl_string_view`, and `cl_zero_ClStr`
  itself (`program.rs:206-231`); they are not in the shim text the reserved scan reads
  (`codegen.rs:576-597`), so `module string; func view` collides.
- **Symbols.** `user_symbol` (`codegen.rs:835-845`) spells `cl_<module>_<name>` and adds
  `__user` (growing `_`) when `is_reserved_symbol` (`:605-609`) matches: the shim's `cl_*` scan
  (`types.rs:22-90`), `cl_init_<module>`, embedded `ffi` names, and the prefixes
  `cl_zero_ClSlice_` and `cl_lambda_`. `is_taken_symbol` (`:616-623`) checks the exact sets.
  Instance symbols `cl_<f>__<key>` are spelled at four sites (`program.rs:1019`, `:4896`,
  `expr.rs:1279`, `function.rs:213`) from `ownership.monomorphization.instances()`; Box helpers
  `cl_box_alloc__<key>`/`cl_box_drop__<key>` (`own/lower.rs:79`, `program.rs:802-813`); zero
  helpers `cl_zero_Cl<Type>`, one per emitted struct unit whether moved or not
  (`program.rs:883-905`, `cleanup.rs:946`); hook support `cl_hook_*` (`program.rs:4610-4800`);
  lambda specializations `cl_lambda_<site>__<key>` (`function.rs:314`). Instance symbols can also
  meet each other: generic struct keys contain `__` (`types/keys.rs:98-105`), so `get<T>` at
  `Holder<i32>` and `get__Holder<T>` at `i32` both spell `cl_get__Holder__i32`.
- **Emission order** (`codegen.rs:658-680`): header, prefix shim, typedefs, enum macros, struct
  and slice typedefs, closure declarations, prototypes, Box helpers, globals, initializers,
  bodies, lifted lambdas, hook support, entry stub, then the hosted-amiga suffix.
- **Suffix.** `HOSTED_AMIGA_SUFFIX` (`profile.rs:18-28`) is one `concat!` of six shim files,
  a separator comment, `picasso96.h`, and a newline; only `hosted-amiga` has one (`:144-149`).
  `shim_suffix() -> &'static str` is read by a spec test (`amiga_optional_spec_test.rs:302-316`),
  `stdlib.rs:1496`, `codegen.rs:581`, and three impl suites.
- **`#line`.** `emit_line_directive` (`codegen.rs:812-816`) and `cleanup.rs:323-330` are the only
  writers, both before Clorraine statements. The path is `SourceMap::path` (`span.rs:108`), the
  spelling given on the command line; stdlib files are `stdlib/<module>.clo`. Module
  declarations keep their span (`ast/mod.rs:35-40`); lambda sites (`ClosureSite`,
  `types/mod.rs:182-190`) and hook call sites (`sema/mod.rs:187-209`) carry none, and
  `ir::Function` has no span field (`ir.rs:643-655`).
- **Driver.** `clorraine` reads the files as given (`driver.rs:197-203` through
  `clorraine/build.rs:65`) and runs the C compiler in its own working folder. The VS Code task
  passes absolute `realPath` names with the workspace root as working folder
  (`tools/vscode/src/actions.ts:63-80,227-295`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/codegen.rs` | names, reservation, `#line` | escape classes, source-name entry point, families, exact loops |
| `compiler/src/{ir,own}.rs`, `compiler/src/own/lower.rs` | `ir::Local` | carry the generated flag |
| `compiler/src/sema/mod.rs`, `compiler/src/types/mod.rs` | lambda and hook sites | carry the source span |
| `compiler/src/clorraine/platform.rs` | path rules | pure relative-path function |
| `compiler/src/codegen/c_names.txt` (new) | class-3 data | the AR-19 list |
| `compiler/src/codegen/{expr,drop,guards,program,function,calls}.rs` | member, capture, enum, instance sites | one spelling per position |
| `compiler/src/codegen/{program,cleanup}.rs` | definitions | per-definition `#line` |
| `compiler/src/profile.rs` | shim suffix | named pieces |
| `compiler/src/clorraine/` | driver | path rewrite |
| `docs/known-limitations.md`, `site/guide/known-limitations.md`, `THIRD_PARTY_NOTICES.md` | docs | §7, §11 rows, residuals, provenance |

## Gaps Identified

### Gap 1: Keywords, reserved identifiers, and prelude names reach C
**Current Behavior:** probes 340 and 341 fail on every leg; a field `long`, a capture `int`, a
parameter `EOF`, a local `memcpy`, `Close`, `__LINE__`, or `textPen`, and an enum `INT { MAX }`
each fail at least one leg (register AR-2, AR-5, AR-15).
**Required Behavior:** R1, R2.
**Fix Required:** 03-01.

### Gap 2: Generated symbols meet user symbols
**Current Behavior:** probe 342, `module box; func alloc__i32`, `module zero; func ClRec`, and
generics named `box_alloc` or `lambda_0` emit duplicate C symbols (AR-3, AR-20).
**Required Behavior:** R5, R6.
**Fix Required:** 03-02.

### Gap 3: Unmapped regions inherit the last `#line`
**Current Behavior:** in hello on hosted-amiga, `#line 73 "stdlib/std.io.clo"` covers the entry
stub and the whole suffix; each function header and its hoisted declarations inherit the previous
function's line (AR-9).
**Required Behavior:** R7.
**Fix Required:** 03-03.

### Gap 4: Driver builds embed absolute paths
**Current Behavior:** an editor build writes the absolute workspace path into every `#line`
(AR-10).
**Required Behavior:** R8.
**Fix Required:** 03-04.

## Dependencies

### Internal Dependencies
- The ownership pass's monomorphization list and the emitter's type table, both complete before
  emission (AR-20).

### External Dependencies
- The SDK image `clorraine-sdk:windows-cli` (host GCC, MinGW-w64, m68k GCC, vbcc) and the rc24
  toolchain root for the extraction and the completeness test (AR-19).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A field site misses the spelling and disagrees with the definition | Med | High (invalid C) | one member-spelling helper every site calls (AR-18); keyword-named fields through every access form built strictly (2.3.1); emission-integrity auditor hunt; ST-7, ST-8 |
| Class 3 renames stdlib names pinned by impl tests | Med | Low | AR-25 intended pin updates, listed per phase |
| A spec test pins a spelling that changes | Low | High | stop for a ruling (AR-25) |
| The list goes stale on a shim or toolchain change | Med | Med | completeness test in `make check` (AR-19) |
| vbcc predefines a name the list misses | Low | Low | residual limit (AR-19) |
| A driver test pins an absolute path | Low | Med | AR-24 reopen trigger |
