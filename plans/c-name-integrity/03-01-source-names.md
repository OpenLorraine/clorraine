# Source names: C Name Integrity

> **Document**: 03-01-source-names.md
> **Parent**: [Index](00-index.md)

## Overview

One rule decides the C spelling of every source name that reaches C, in all six positions of
AR-2 and AR-15. Closes ASM-WS3-001 (#61).

## Architecture

### Current Architecture
02 §What Exists: keyword-only `cl_` prefix for locals and parameters; raw fields, captures, and
enum constants.

### Proposed Changes

#### Escape classes (AR-5)
1. C99 keywords (`auto break case char const continue default do double else enum extern float
   for goto if inline int long register restrict return short signed sizeof static struct switch
   typedef union unsigned void volatile while _Bool _Complex _Imaginary`) and `bool`.
2. Reserved identifiers: a name beginning with `__`, or with `_` and an upper-case letter.
3. Every name in `c_names.txt` (AR-19), plus the program's own final enum-constant spellings
   (AR-16).

#### Spelling (AR-18)
`marked(n)`: `n`, or `n` with one or more trailing `_<digits>` groups removed, is in a class.
`escape(n) = n + "_0"` if `marked(n)`, else `n`. A local or parameter then goes through the existing
uniqueness counter (`_1`, `_2`, …) for shadowing and duplicates.

#### Source-name entry point (AR-17)
One function (name chosen by the executor) returns `escape(sanitize(n))`, where `sanitize` keeps
today's byte replacement without the `cl_` prefix. Every source-name site calls it:
- locals: the `local_names` table; the fallbacks at `expr.rs:241,247,631`, `function.rs:608,663`,
  and the func-typed callee fallback at `expr.rs:1370-1375`;
- parameters: `unique_param_names` for function, lambda, prototype, and function-type parameters
  (masked spellings such as `give#0` included);
- fields: the struct definition (`program.rs:753,954`) and every member spelling site listed in
  AR-18;
- captures: the environment field (`program.rs:746`) and its reads (`expr.rs:265-267,1380-1388,
  1508-1514`, `function.rs:1626-1633`);
- enum constants: one helper for `<Enum>_<Member>` used at `program.rs:1000` and `expr.rs:585`;
  the constant is escaped when `<Enum>_<Member>` is marked against classes 1–3 without the
  program's own constants.
Generated names keep the plain `unique_local` and are never escaped.

#### The name list (AR-19, AR-26)
- File: `compiler/src/codegen/c_names.txt`, one name per line, sorted, ASCII, no comments; built
  in with `include_str!` and probed by binary search over a lazily parsed `&'static [&'static str]`.
- Content: the union over these legs of every non-reserved identifier the unit defines or
  declares at file scope (macros of both kinds, typedefs, functions, objects), minus the sample
  program's own symbols (`cl_<module>_*`, `cl_init_*`, user typedefs):
  | Leg | Compiler and flags (as `toolchain.rs`) |
  |---|---|
  | host | SDK `cc -std=c99` |
  | host Windows | SDK `x86_64-w64-mingw32-gcc -std=c99` |
  | hosted-amiga | SDK `m68k-amigaos-gcc` with the profile flags, whole unit including the suffix |
  | hosted-amiga-13 | SDK `m68k-amigaos-gcc -mcrt=nix13` |
  | freestanding | SDK `m68k-amigaos-gcc -ffreestanding` and host `cc` |
  | vbcc | GCC's preprocessor `-dM -nostdinc -undef -D__VBCC__ -D__M68K__ -I<vbcc include> -I<ndk-include>`, plus every `#define` name under the vbcc include directory |
  | rc24 | the rc24 root's `m68k-amigaos-gcc` for hosted-amiga and hosted-amiga-13 |
- Sample program: one fixed program (a new fixture beside the tests) whose emission contains the
  whole prelude, typedef block, and suffix for each profile.
- Extraction: macros from `-E -dM`; functions from `-aux-info`; typedefs and objects from the
  preprocessed unit's file-scope declarations. The executor records the exact command once in the
  Phase 1 notes of 99 and runs it inside `make -C docker exec` and against the rc24 root; it is not
  committed as a tool (AR-7).
- Completeness test (impl): re-runs the extraction for every leg present (skips a missing one with
  the suites' `skip:` message; rc24 only with `CLORRAINE_RC24_ROOT`), asserts every extracted name is
  in the list or reserved-shaped, asserts the list is sorted and unique, and asserts the AR-18
  invariant. Failure output lists the missing names.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A shim or header change adds a name | completeness test fails with the names to add | AR-19 |
| A list entry breaks the `_0` invariant | completeness test fails | AR-18 |
| An escaped spelling is pinned by a spec test | stop for a ruling | AR-25 |

## Testing Requirements
- ST-1..ST-11 (07); impl tests in 07 §Implementation Tests.
