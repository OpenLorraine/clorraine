# Generated symbols: C Name Integrity

> **Document**: 03-02-generated-symbols.md
> **Parent**: [Index](00-index.md)

## Overview

Generated instance and helper symbols never share a C name with a user symbol or with each
other. Closes ASM-WS3-002 (#64). Decisions AR-8, AR-20.

## Architecture

### Current Architecture
02 §What Exists: `user_symbol`'s `__user` rename over the shim set, `cl_init_*`, embedded `ffi`
names, and two prefix families; instance symbols spelled raw at four sites.

### Proposed Changes

1. **Instance helper.** One function spells an instance symbol `cl_<f>__<key>` from the function
   name and its type arguments; the four sites call it. It returns the spec/17 §7 spelling unless
   that spelling meets a helper family or the shim set, in which case it adds `__user` and grows
   `_` until the candidate is in no exact set (step 3).
2. **Trigger families for user symbols.** `is_reserved_symbol` adds `cl_<g>__` for every declared
   generic function `g` (from the AST), `cl_box_alloc__`, `cl_box_drop__`, `cl_zero_Cl`, and
   `cl_hook_`, beside the existing `cl_zero_ClSlice_` and `cl_lambda_`.
3. **Exact sets for the loops.** Before emission, collect every emitted instance symbol, every Box
   helper (per key), zero helper (per moved struct type), lambda specialization, and hook-support
   name. `is_taken_symbol` and the instance helper's loop check these exact sets together with the
   shim set and the user symbols, so a family prefix never loops and a candidate equal to a real
   helper moves on.
4. **Order.** A plain user function keeps losing to an instance of the same spelling (it is the one
   renamed), and an instance loses to a helper; helpers keep their spelling.
5. **`ffi` names.** The wider reserved set also feeds `check_ffi_collisions`
   (`codegen.rs:526-569`): an `ffi` literal equal to a generated symbol becomes E11101, which is
   the existing rule for runtime names.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A symbol emitted outside the exact sets | impl test enumerates every generated family against the sets | AR-20 |
| `__user` candidate equals a real helper | exact-set loop continues | AR-20 |

## Testing Requirements
- ST-12..ST-17 (07); impl tests in 07.
- `docs/known-limitations.md` §7 table gains the instance, Box, zero, and hook rows (03-04).
