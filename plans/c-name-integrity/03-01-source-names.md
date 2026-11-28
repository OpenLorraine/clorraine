# Source names: C Name Integrity

> **Document**: 03-01-source-names.md
> **Parent**: [Index](00-index.md)

## Overview

One rule decides the C spelling of every source name that reaches C, in all four name positions
(locals, parameters, struct fields, closure-capture fields; AR-2, AR-15). Enum constants stop
reaching C as names (AR-16 as amended). Closes ASM-WS3-001 (#61).

## Architecture

### Current Architecture
02 §What Exists: keyword-only `cl_` prefix for locals and parameters; raw fields, captures, and
enum macros.

### Proposed Changes

#### Escape classes (AR-5)
1. C99 keywords (`auto break case char const continue default do double else enum extern float
   for goto if inline int long register restrict return short signed sizeof static struct switch
   typedef union unsigned void volatile while _Bool _Complex _Imaginary`) and `bool`.
2. Reserved identifiers: a name beginning with `__`, or with `_` and an upper-case letter.
3. Every name in `c_names.txt` (AR-19). The program's enum constants do not join this class: they
   no longer reach C as names.

#### Spelling (AR-18)
`marked(n)`: `n`, or `n` with one or more trailing `_<digits>` groups removed, is in a class.
`escape(n) = n + "_0"` if `marked(n)`, else `n`. A local or parameter then goes through the existing
uniqueness counter (`_1`, `_2`, …) for shadowing and duplicates; for a source name the counter
also skips any candidate that is itself in class 1–3, so a counter output never lands on a header
name. An empty name (unreachable from source, spec/01 LEX-5) is spelled `_0`.

#### Source-name entry point (AR-17)
One function (name chosen by the executor) returns `escape(sanitize(n))`, where `sanitize` keeps
today's byte replacement without the `cl_` prefix. It is applied where a source name is *given*
its C spelling, and every later use reads that spelling back:
- locals: the `local_names` table (`function.rs:197-203`, lambda bodies `:296-304`), for every
  `ir::Local` that is not generated. `ir::Local` gains a `generated: bool` copied from the
  binding (`own.rs:1250-1259`, `own/lower.rs:1812-1815`; `ir::Builder::add_local`,
  `ir.rs:776-780`, defaults to `false`); generated locals (`__cl_owned<N>`) keep plain
  `unique_local`. A spelling pattern is never used to tell them apart: a source local may be
  named `__cl_owned0`.
- parameters: `unique_param_names` for function, lambda, prototype, and function-type parameters
  (masked spellings such as `give#0` included).
- the fallbacks that run when a head is not in `local_names` (`expr.rs:241,247,627-631,1370-1375`,
  `function.rs:608,663`) are not source-name sites: they spell `ffi` function values and
  generated paths (`cl_hooktramp_<id>`) whose C spelling is fixed elsewhere. They keep today's
  `sanitize` without escaping. An impl test asserts every source local and parameter of a
  sample function is in `local_names`, so the fallback is never reached for a source name.
- fields: one member-spelling helper, `escape(name)`, used by the struct definition
  (`program.rs:886-889` from the names at `:954`) and by **every** site that writes a member
  name after `.` or `->` or inside `offsetof`. The rule, not a site list, is the contract; the
  known sites include `expr.rs:66-70,205-210,583-636,711-735`, `drop.rs:95,236`,
  `guards.rs:1564,1639`, `calls.rs:494,582`, `program.rs:1468,1551,2781`, `function.rs:3148`.
  The executor records the grep of `\.\{|->\{|offsetof` over `compiler/src/codegen/` in the
  Phase 2 notes as the site audit.
- captures: the environment field (`program.rs:746-753`) and its reads (`expr.rs:265-267,
  1380-1388,1508-1514`, `function.rs:1626-1633`), through the same member helper.
- enum constants: the one use site (`expr.rs:584-585`) returns the member's folded value as
  `(<value>)`, the same token text the macro expands to today, parenthesized so a negative value
  stays one operand; the `#define` loop (`program.rs:999-1001`) is removed. The literal keeps the
  macro's C type and its integer-constant-expression status (case labels, array extents, static
  initializers).
Generated names keep the plain `unique_local` and are never escaped.

#### The name list (AR-19, AR-26)
- File: `compiler/src/codegen/c_names.txt`, one name per line, sorted, ASCII, no comments; built
  in with `include_str!` and probed by binary search over a lazily parsed `&'static [&'static str]`.
- Content: the union over these legs of every non-reserved ordinary identifier the unit defines
  or declares at file scope (macros of both kinds, typedefs, functions, objects, and enumerators),
  minus the sample program's own symbols (`cl_<module>_*`, `cl_init_*`, user typedefs):
  | Leg | Compiler and flags (as `toolchain.rs`) |
  |---|---|
  | host | SDK `cc -std=c99` |
  | host Windows | SDK `x86_64-w64-mingw32-gcc -std=c99` |
  | hosted-amiga | SDK `m68k-amigaos-gcc` with the profile flags, whole unit including the suffix |
  | hosted-amiga-13 | SDK `m68k-amigaos-gcc -mcrt=nix13` |
  | freestanding | SDK `m68k-amigaos-gcc -ffreestanding` and host `cc` |
  | vbcc | GCC's preprocessor `-dM -nostdinc -undef -D__VBCC__ -D__M68K__ -I<vbcc include> -I<ndk-include>`, plus every `#define` name under the vbcc include directory (`/opt/amiga/m68k-amigaos/vbcc/include` in the image) |
  | rc24 | the rc24 root's `m68k-amigaos-gcc` for hosted-amiga and hosted-amiga-13 |
- Sample program: one fixed program (a new fixture beside the tests) whose emission contains the
  whole prelude, typedef block, and suffix for each profile.
- Extraction method, the same in the one-off command and in the test:
  - macros: the names from `-E -dM`;
  - functions: the names from `-aux-info`;
  - typedefs, objects, and enumerators: a std-only token scan of the `-E` output. It tracks brace
    and paren depth, skips `__attribute__((…))`, `__asm__(…)`/`__asm(…)`, and `__extension__`,
    and collects, at brace depth 0, the declarator identifier before `;`, `,`, `=`, `[`, or `:`
    (outside parentheses, except the `(*name)` form), and every enumerator inside a file-scope
    `enum { … }` body. Prototype parameter names (paren depth > 0) are not collected. Struct,
    union, and enum tags are a separate C namespace and are not collected. The scan may
    over-collect; an extra name only adds one `_0`.
  - recall check: on the hosted-amiga leg, every function `-aux-info` reports is also found by the
    scan; the test asserts it.
- The executor records the exact one-off command in the Phase 1 notes of 99 and runs it inside
  `make -C docker exec` and against the rc24 root. The scanner lives in the impl test; a test
  that re-derives the list is a test, not committed generator tooling (AR-7).
- Completeness test (impl): re-runs the extraction for every leg present (skips a missing one with
  the suites' `skip:` message; rc24 only with `CLORRAINE_RC24_ROOT`, and when that variable is
  set but the root is unusable the test fails rather than skips), asserts every extracted name is
  in the list or reserved-shaped, asserts the list is sorted and unique, and asserts the AR-18
  invariant. Failure output lists the missing names.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A shim or header change adds a name | completeness test fails with the names to add | AR-19 |
| A list entry breaks the `_0` invariant | completeness test fails | AR-18 |
| A member site skips the helper | keyword-named fields through every access form fail the strict C build (2.3.1) | AR-18 |
| An escaped spelling is pinned by a spec test | stop for a ruling | AR-25 |

## Testing Requirements
- ST-1..ST-11 and ST-25 (07); impl tests in 07 §Implementation Tests.
