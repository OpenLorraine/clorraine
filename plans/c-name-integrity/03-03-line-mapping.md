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

**Top-level item** (the term R7, ST-18, and the impl test use): a function definition, a
file-scope object definition, or a struct definition that the compiler writes outside any
function body. The rule covers every top-level item after the first `#line` and before the first
`#line 1 "compiler/runtime/…"` directive; each is immediately preceded, ignoring blank lines and
comments, by a `#line`. Items of one group repeat the group's line. The runtime files' own
definitions are covered by their file's single `#line 1`.

| Region (emission order, `codegen.rs:658-680`) | `#line` before each item |
|---|---|
| Prefix shim, typedefs, prototypes, Box helpers, globals (before the first `#line`) | none: the C file's own lines stay right |
| Module initializer `cl_init_<module>` | the module's `module` declaration line (`ModuleDecl.span`), its file |
| User or stdlib function, and each generic instance | the `func` declaration line, its file; covers the header and hoisted declarations |
| Lifted lambda `cl_lambda_*` | the lambda expression's line |
| Hook trampoline and wrapper | the hook lambda's line |
| Hook env-stack support (`struct ClHookEnvNode`, `cl_hook_env_top`, the `cl_hook_env_*` functions) | the first hook lambda's line, repeated before each item |
| Entry stub `int main(void)` | the root module's `main` declaration line |
| Each hosted-amiga suffix file | `#line 1 "compiler/runtime/<file>"` |

- Span sources: a lambda's span is recorded on its `ClosureSite` when sema creates the site
  (`types/mod.rs:182-190`, `sema/mod.rs:511-520`), and a hook lambda's span on its hook call
  site (`sema/mod.rs:187-209`); both are new fields. Functions, instances, and the entry stub use
  their `FuncDecl`; initializers use `ModuleDecl.span` (`ast/mod.rs:35-40`).
- `Profile::shim_suffix() -> &'static str` keeps its signature and bytes (a spec test reads it).
  A new accessor returns the same content as an ordered list of `(logical name, text)` pieces
  built from the same `include_str!` constants: the six shim files and `picasso96.h` with names,
  the two literal separators without one. A unit test asserts that joining the pieces equals
  `shim_suffix()`. Only the emitter's suffix step (`codegen.rs:678`) uses the pieces; a piece
  without a name is written before the next named piece's `#line`, so the counted lines stay
  exact.
- The paths use the existing `c_string_body` escaping.
- Host emission gains no runtime name (host has no suffix), so the `windows_cli_spec_test` path
  rule holds.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A definition emitted without a source span | impl test scans every top-level item in the covered region for a preceding directive | AR-21 |
| Pieces drift from `shim_suffix()` | join-equality unit test | AR-21 |

## Testing Requirements
- ST-18..ST-20 (07).
