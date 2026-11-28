# Line mapping: C Name Integrity

> **Document**: 03-03-line-mapping.md
> **Parent**: [Index](00-index.md)

## Overview

Every region of emitted C after the first `#line` names a real source or runtime line. Closes
ASM-WS3-005. Decisions AR-9, AR-21.

## Architecture

### Current Architecture
02 §What Exists: `#line` only before Clorraine statements; one `concat!` suffix.

### Proposed Changes

| Region (emission order, `codegen.rs:658-680`) | `#line` before it |
|---|---|
| Prefix shim, typedefs, prototypes, Box helpers, globals (before the first `#line`) | none: the C file's own lines stay right |
| Module initializer `cl_init_<module>` | the module's `module` declaration line, its file |
| User or stdlib function, and each generic instance | the `func` declaration line, its file; covers the header and hoisted declarations |
| Lifted lambda `cl_lambda_*` | the lambda expression's line |
| Hook trampoline and wrapper | the hook lambda's line |
| Hook env-stack helpers (`cl_hook_env_*`) | the first hook lambda's line |
| Entry stub `int main(void)` | the root module's `main` declaration line |
| Each hosted-amiga suffix file | `#line 1 "compiler/runtime/<file>"` |

- `Profile` returns the suffix as an ordered list of `(logical name, text)` pieces: the six shim
  files and `picasso96.h` with names, the two literal separators without one. A piece without a
  name is written before the next named piece's `#line`, so the counted lines stay exact.
- The paths use the existing `c_string_body` escaping.
- Host emission gains no runtime name (host has no suffix), so the `windows_cli_spec_test` path
  rule holds.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A definition emitted without a source span | impl test scans every top-level definition after the first `#line` for a preceding directive | AR-21 |

## Testing Requirements
- ST-18..ST-20 (07).
