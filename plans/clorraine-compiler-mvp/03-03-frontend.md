# Front End: Clorraine Compiler MVP

> **Document**: 03-03-frontend.md
> **Parent**: [Index](00-index.md)

## Overview

The front end turns source files into a typed, checked program: workspace and
crate layout, the diagnostic pipeline, the lexer, the parser, and the CLI
skeleton. It implements the frozen surface grammar (`grammar.ebnf.md`
sections 2-10, disambiguation section 11) and the lexical chapter (Ch 01).
Behavior is owned by the specification; this document owns only the Rust
architecture and interfaces (AR-20, AR-24).

## Workspace and Crate Layout

```
Cargo.toml                     # [workspace] members = ["compiler"]; no dependencies
compiler/
├── Cargo.toml                 # package clc; [[bin]] clc; lib clc
├── src/
│   ├── main.rs                # thin CLI entry (calls driver::run)
│   ├── lib.rs                 # module declarations; compile() for unit tests
│   ├── span.rs                # SourceMap, FileId, Span
│   ├── diag.rs                # Diagnostic, Severity, Registry, renderer
│   ├── lex.rs                 # Token, TokenKind, Lexer
│   ├── ast.rs                 # AST node types
│   ├── parse.rs               # recursive descent + Pratt parser
│   ├── load.rs                # file loading, module table, stdlib embedding
│   ├── resolve.rs             # scopes, imports, name resolution
│   ├── types.rs               # Ty, type table, unification, conversions
│   ├── consts.rs              # constant evaluation
│   ├── sema.rs                # statement/expression/struct/enum checking
│   ├── ir.rs                  # typed control-flow IR (Ch 17 §2, Ch 16 §6)
│   ├── own.rs                 # definite assignment, move state, drop planning
│   ├── borrow.rs              # loan collection and conflict checking (v1 lexical)
│   ├── abi.rs                 # FFI ABI mapping and emitted type names
│   ├── codegen.rs             # C emitter
│   ├── profile.rs             # compiled-in profile tables (Ch 15)
│   ├── stdlib.rs              # embedded stdlib module table (include_str!)
│   └── driver.rs              # pipeline orchestration and CLI behavior
├── runtime/                   # C shims embedded by codegen (AR-19)
│   ├── shim_host.c
│   └── shim_hosted_amiga.c
└── tests/                     # CLI-level golden drivers (AR-24, AR-26)
    ├── compile_fail_spec_test.rs
    ├── run_fault_spec_test.rs
    ├── corpus_spec_test.rs
    ├── determinism_spec_test.rs
    └── cli_spec_test.rs
stdlib/                        # Clorraine stdlib sources (03-07)
```

Std-only: no external crates, no build scripts (AR-20). The workspace root
`target/` is already gitignored.

## Pipeline

```
load (CLI files + imports + embedded stdlib)
  → lex → parse (AST)
  → resolve (symbols, modules, imports)
  → check (types, consts, structs, enums, generics)
  → own (IR/CFG, definite assignment, moves, loans, FFI contracts)
  → codegen (single C99 TU: shim + program)
  → render diagnostics / write -o output
```

Each phase consumes the previous phase's output; a phase that reports an error
poisons the affected declaration and suppresses dependent diagnostics (Ch 14 §1)
but lets independent errors be reported (N5). No phase may re-derive an
ownership decision later (Ch 17 §2); the IR is the single carrier.

## Diagnostics

`diag.rs` owns the diagnostic value and rendering (AR-22):

```rust
pub enum Severity { Error, Warning }
pub struct Diagnostic {
    pub code: &'static str,      // "E10010", "W10004", ...
    pub severity: Severity,
    pub message: String,         // rendered template, e.g. "value 'f' may be null here"
    pub span: Span,
    pub help: Option<String>,
}
```

- Rendering: first line exactly `error CODE: message` / `warning CODE: message`
  (N4); then `  --> file:line:col`; then an optional `  help: ...` line.
- All diagnostics go to stderr; the compiler reports every recoverable error
  (N5); `--deny-warnings` promotes warnings to exit-code failure (but the first
  line stays `warning CODE: message`, AR-33/AR-34); `--max-errors=N` caps output.
- The registry of codes and message templates is Ch 14; the compiler holds one
  `fn message(...)` per diagnostic that renders the template. The harness
  matches the registered template with `{...}` wildcards (AR-23); the plan
  does not duplicate the registry.
- Span rendering uses the logical CLI path (AR-35), not absolute paths.

## Lexer

`lex.rs` implements Ch 01 and `grammar.ebnf.md` §10:

- 50 keywords, one contextual keyword (`as`, only inside imports), reserved
  built-ins as identifier-shaped tokens (LEX-8/LEX-9, grammar §10.2-10.4).
- Literals keep their spelling and a parsed value: decimal/hex/binary with `_`
  separators (never leading/trailing), floats requiring digits on both sides of
  `.` (LEX-17), char literals as `u8` (LEX-20/LEX-21), string literals as
  `static str`. Context typing and fit are semantic (E10202); malformed text is
  lexical (E20004).
- Templates: the lexer scans `$"..."` tracking nested braces and strings
  (grammar §7) and yields a `Template` token carrying literal parts and
  placeholder source spans; unbalanced braces are E10503 and a `:` directly
  inside a placeholder is E10501 (parser-only detection per PF-031). Nested
  templates are E90005.
- Comments: `//`, `/* */` non-nesting, `///` as an ignored doc comment
  (LEX-27); unterminated block comment/string/char/template are E20001/E20002.
- Foreign syntax (`->`, `::`, `#`, `@`, `?.`, `??`) is E20008 before parser
  recovery (grammar §10, Ch 01).
- The lexer never fails hard: it emits a diagnostic token and continues, so
  the parser can report independent syntax errors.

## Parser

`parse.rs` is recursive descent for declarations/statements and Pratt for
expressions (`grammar.ebnf.md` §6.2 precedence chain, EX-1); it implements
every production in the index (grammar §12) and the disambiguation notes
(grammar §11):

| Case | Rule |
|---|---|
| `{` block vs struct literal | statement position = block; after a type reference in expression position = struct literal |
| Generic `<` vs comparison | bounded speculative parse: `<` starts type arguments only when a type list closes with `>` immediately followed by `(` or `{` |
| Lambda vs grouping / capture list | `(` + `identifier :` typed params starts a lambda; `[` closes with `]` followed by `(` starts captures |
| `?` | type position = nullability; expression position = conditional |
| Element loop vs C-style `for` | `borrow` or `identifier in` after `(` selects the element form |
| `raw` binding | binds to the immediately following type atom |
| `unsafe {` vs `unsafe <expr>` | block form vs prefix operator |
| Diagnostic-only starts | `union` → E10204, misplaced `volatile` → E10208, `:` in a placeholder → E10501 |

AST (`ast.rs`) mirrors the grammar: `Program { module, imports, items }`;
items `Func, Struct, Enum, Handle, FfiBlock, Intrinsic, GlobalLet, GlobalConst,
ComptimeFunc`; statements `Let, Const, Expr, If, While, DoWhile, ForC, ForIn,
Return, Break, Continue, Switch, Defer, UnsafeBlock, Block`; expressions
`Literal, Name, QualifiedName, Call, CallMarker, Index, Member, Unary, Binary,
Assign, Conditional, Cast, RawCast, StructLiteral, ArrayLiteral, Lambda,
UnsafeExpr, IntrinsicCall, Template`. Every node carries a `Span`.

Lambda bodies and `comptime` bodies parse fully; their semantic stage reports
E90000/E90002/E90001 (AR-10, AR-14; Ch 06 §6.1/§7).

## CLI Skeleton

`driver.rs` implements the AR-21 contract from the first phase:

```text
clc --profile <host|hosted-amiga> [--deny-warnings] [--max-errors=N]
      [-o OUT.c] FILE...
```

- `--profile` is required and validated (`E11201` unknown; `freestanding` is
  E11200 via the profile table, AR-10).
- At least one input file is required; files are read as UTF-8 (LEX-1). All
  user files are loaded up front (AR-25).
- Exit code 1 when any diagnostic had `Error` severity, or when
  `--deny-warnings` is set and a warning was reported; otherwise 0.
- Until Phase 6, a successful run writes an empty translation unit; no test
  asserts output content before the codegen phase (the harness only uses error
  cases in Phases 2-5).

## Testing Requirements

- Spec tests: the CLI-level golden driver for E20xxx syntax cases and the
  parser-only diagnostics (E10204, E10208, E10501, E90005), plus the ST cases
  in [07-testing-strategy.md](07-testing-strategy.md).
- Impl tests: lexer tokenization and parser AST unit tests in-module
  (`#[cfg(test)]`), covering the disambiguation table above.
- The red phase must show the golden driver failing before the lexer/parser
  exist; the green phase shows the in-scope E20xxx files passing with the
  Ch 14 first-line shape.
