# Generated symbols: C Name Integrity

> **Document**: 03-02-generated-symbols.md
> **Parent**: [Index](00-index.md)

## Overview

Generated instance and helper symbols never share a C name with a user symbol or with each
other. Closes ASM-WS3-002 (#64). Decisions AR-8, AR-20.

## Architecture

### Current Architecture
02 §What Exists: `user_symbol`'s `__user` rename over the shim set, `cl_init_*`, embedded `ffi`
names, and two prefix families; instance symbols spelled raw at four sites; the emitter's fixed
prelude helpers outside the reserved set.

### Proposed Changes

1. **Instance symbol map.** Before emission, one pass walks `ownership.monomorphization.instances()`
   in its order (a discovery-order `Vec`, `ir/mono.rs:699`, covered by the determinism suite) and
   assigns each instance its C symbol: the spec/17 §7 spelling `cl_<f>__<key>`, unless that
   spelling meets a helper family, the shim set, the fixed prelude helpers, or a symbol already
   assigned to an earlier instance; then it adds `__user` and grows `_` until the candidate is in
   no exact set (step 3). The four sites (`program.rs:1019`, `:4896`, `expr.rs:1279`,
   `function.rs:213`) read the map, so a prototype and its definition always agree.
2. **Trigger families for user symbols.** `is_reserved_symbol` adds `cl_<g>__` for every declared
   generic function `g` (from the AST), `cl_box_alloc__`, `cl_box_drop__`, `cl_zero_Cl`, and
   `cl_hook_`, beside the existing `cl_zero_ClSlice_` and `cl_lambda_`, and the exact fixed
   prelude names `cl_str`, `cl_string_view`, and `cl_zero_ClStr` (`program.rs:206-231`).
3. **Exact sets for the loops.** Before emission, collect every emitted instance symbol (from the
   map), every Box helper (per key), zero helper (one per emitted struct unit, each struct and
   each generic struct instance, moved or not, plus `cl_zero_ClStr`), lambda specialization,
   hook-support name, and the fixed prelude helpers. `is_taken_symbol` and the instance loop
   check these exact sets together with the shim set and the user symbols, so a family prefix
   never loops and a candidate equal to a real helper moves on.
4. **Order.** A plain user function keeps losing to an instance of the same spelling (it is the one
   renamed); an instance loses to a helper and to an earlier instance; helpers keep their
   spelling.
5. **`ffi` names.** The wider reserved set also feeds `check_ffi_collisions`
   (`codegen.rs:526-569`): an `ffi` literal equal to a generated symbol that the program emits
   becomes E11101, which is the existing rule for runtime names. A generated name the program
   does not emit stays legal.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A symbol emitted outside the exact sets | impl test enumerates every generated family against the sets | AR-20 |
| `__user` candidate equals a real helper | exact-set loop continues | AR-20 |
| Two instances spell one symbol | the later instance in map order takes `__user` | AR-20 |

## Testing Requirements
- ST-12..ST-17, ST-26, ST-27 (07); impl tests in 07.
- `docs/known-limitations.md` §7 gains the instance, Box, zero, and hook rows, and the §11
  "Hook symbol names" and "Box helper names" rows narrow to names the program does not emit
  (Phase 3 docs task).
