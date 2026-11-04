# Testing Strategy: Clorraine DX Tooling

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| CLI behavior (flags, output, exit codes) | Every ST case below, plus edge cases in the implementation suite |
| Registry parse and render | Unit tests over the embedded registry and synthetic tables |
| JSON renderer | Unit tests for schema, escaping, and positions; ST cases end to end |
| Grammar completeness | Spec test over the frozen EBNF; demo integration through Shiki |

All cases follow the project's spec-first ordering: specification tests are written first and must
fail before implementation; expectations come from the registry, the EBNF, and the AR decisions.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from `01-requirements.md`, the three `03-XX` specs, and the Ambiguity Register.
> The immutable-oracle rule applies: a failing case means the implementation is wrong, never the
> test.

### Explain mode (`dx_tooling_spec_test.rs`)

Fixtures spawn the built binaries exactly like `clorraine_driver_spec_test.rs`
(`env!("CARGO_BIN_EXE_clc")`, `env!("CARGO_BIN_EXE_clorraine")`, private scratch directories).

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-1 | `clc --explain E10034` | stdout exactly `error E10034: cannot release or store borrowed parameter '{name}'` + `  trigger: Borrowed parameter released or stored by the callee` + `  section: Ownership, moves, initialization, nullability, borrows (E100xx)` + `  help: declare the parameter \`take\` or by value; a \`borrow T?\` slot may hold its own incoming value, a static view, or a newly owned value`, exit 0, empty stderr | R1, AR-3, AR-4 |
| ST-2 | `clorraine --explain W10000` | stdout exactly `warning W10000: unused local '{name}'` + `  trigger: Local declared and never used` + `  section: Warnings (W100xx)` + `  help: remove the binding or use it`, exit 0 | R1, R7, AR-2, AR-10 |
| ST-3 | `clc --explain=E10034` | identical stdout to ST-1, exit 0 (joined spelling) | R1, AR-3 |
| ST-4 | `clc --explain NOPE` | stderr exactly `usage error: unknown diagnostic code 'NOPE'`, empty stdout, exit 2 | R2, AR-3 |
| ST-5 | `clc --explain` (no value) | stderr exactly `usage error: --explain needs a value`, exit 2 | R2, AR-3 |
| ST-6 | `clorraine --explain E10034` (no command) and `clorraine run --explain E10034` | both print the ST-1 entry, exit 0 (honoured wherever it appears) | R1, AR-3 |
| ST-7 | `clc --explain E10034 main.clo --profile host` | prints the ST-1 entry, exit 0 (valid extra arguments are ignored) | R1, AR-3, AR-13 |

### Machine-readable diagnostics (`dx_tooling_spec_test.rs`)

Fixtures: `bad.clo` has a module line and a `demo` that uses the null-initialized local, so
`E10012` reports on the `let`; `unused.clo` has a module line and one unused local in `main`.
Both need the module declaration the current parser requires.

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-8 | `clc --diagnostics=json --profile host -o out.c bad.clo` | stderr is exactly one line `{"code":"E10012","severity":"error","message":"'null' is not allowed here","help":"declare the type \`T?\` or avoid the empty state","file":"bad.clo","line":4,"column":5,"endLine":4,"endColumn":22}`, stdout empty, exit 1 | R3, R4, AR-5, AR-6 |
| ST-9 | `clc --diagnostics=json --profile host -o out.c unused.clo` | stderr is exactly one line with `"code":"W10000"`, `"severity":"warning"`, `"message":"unused local 'unused'"`, `"help":null`, `"line":4,"column":9,"endLine":4,"endColumn":15`, exit 0 | R3, R4, AR-6 |
| ST-10 | `clc --diagnostics=json --profile host` (no files) and `clorraine run --diagnostics=json` (no files) | one object line per run with `"code":null`, `"severity":"error"`, and message `usage error: at least one input file is required`, exit 2 | R5, AR-7 |
| ST-11 | `clc --diagnostics=json --profile host --max-errors "3\"z"` and an unknown option argument containing a newline | each stderr is exactly one JSON line whose message carries the argument verbatim with `\"` / `\n` escapes; exit 2 | R4, AR-15 |
| ST-12 | `clc --profile host -o out.c bad.clo` (no flag) | stderr byte-identical to today: `error E10012: 'null' is not allowed here` + `  --> bad.clo:4:5` + `  help: …`; exit 1 | R6, AR-5 |
| ST-13 | `clc --diagnostics=xml --profile host bad.clo` | stderr exactly `usage error: unknown diagnostics format 'xml' (expected json)`, exit 2 (human — no format was selected) | R3, AR-5 |
| ST-14 | `clc --diagnostics=json --deny-warnings --profile host -o out.c unused.clo` | one line with `"severity":"warning"`, exit 1 | R4, R6, AR-6 |
| ST-15 | `clorraine build --diagnostics=json bad.clo` | NDJSON on stderr from the in-process compiler (one `E10012` object), exit 1 | R3, AR-5 |
| ST-20 | `clorraine build --diagnostics=json good.clo` with an empty `PATH` | exactly one object line with `"code":null`, `"severity":"error"`, and a message naming the missing `'cc'` tool; exit 1 (environment error) | R5, AR-7 |
| ST-21 | `clorraine build --diagnostics=json good.clo` with a failing stand-in `cc` on `PATH` | exactly one object line with `"code":null`, `"severity":"error"`, and a message containing `internal compiler error` and `first error:`; the multi-line ICE text is escaped onto that one line; exit 1 | R5, AR-7, AR-15 |

### Grammar (`dx_tooling_spec_test.rs`)

| # | Input / Scenario | Expected Output / Behavior | Source |
| - | ---------------- | -------------------------- | ------ |
| ST-16 | Every term of the EBNF `keyword`, `contextual_keyword`, and `reserved_builtin` productions vs `editors/clorraine.tmLanguage.json` | each term appears word-bounded inside an extracted `match`/`begin`/`end` pattern string | R9, AR-9 |
| ST-17 | Repository paths | `editors/clorraine.tmLanguage.json` exists; `tools/demo/grammars/clorraine.tmLanguage.json` does not; `tools/demo/src/util/shiki.ts` references `editors` and `clorraine.tmLanguage.json` | R8, AR-8 |
| ST-18 | Grammar metadata | the file contains `"scopeName": "source.clorraine"` and a `fileTypes` list containing `"clo"` | R9, AR-9 |
| ST-19 | v1 surfaces | `=>` appears in a pattern string; `freedBy` appears; the stale `deleter` appears in no pattern string | R9, AR-8, AR-9 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/dx_tooling_spec_test.rs` | ST-1 – ST-21 | All three components |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/dx_tooling_impl_test.rs` | The registry parse over synthetic tables (missing help, odd cells, numbering), the 114-code coverage and every-row-has-help (R7), the exact render shape and trailing newline, JSON string escaping (`"` `\` newline tab control), key order, span-less diagnostics, the pre-scan helper, the human/JSON error formatter, and the pattern-extraction scanner's escaping edge cases | High |
| `compiler/src/diag.rs` `mod tests` | Unit tests for `DiagnosticFormat`, `format_from_args`, `push_json_string`, and `render_json` shapes | High |
| `compiler/src/diag/registry.rs` `mod tests` | Unit tests for `parse_registry` over synthetic documents | High |
| `tools/demo/test/grammar.spec.test.ts` | The extended fixture asserts the newly covered tokens' scopes through Shiki | Med |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make demo-check` | Demo toolkit + canonical grammar | The toolkit loads the moved grammar and renders the extended fixture |
| `make check` | All suites | The new suites run under the container |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Editor-style JSON consumption | `clorraine build --diagnostics=json` on a broken fixture inside the container (ST-15) | stderr parses as NDJSON; every line is one JSON object; exit code 1 |
| Explain/diagnostic agreement | `clc --explain E10034` (ST-1) vs compiling a program that triggers `E10034` | The explain text's template and help match the diagnostic's `message` and `help:` line |

Both scenarios run through existing suites (ST-1 and ST-15); no new harness is built.

## Test Data

### Fixtures Needed

| Fixture | Used by | Content |
| ------- | ------- | ------- |
| `bad.clo` | ST-8, ST-12, ST-15 | A module line, a `demo` that uses the null-initialized local, and `main` |
| `unused.clo` | ST-9, ST-14 | A module line and a `main` with one unused local |
| `good.clo` | ST-20, ST-21 | A module line and a minimal valid `main` that reaches the C compiler |
| failing `cc` (created by the test) | ST-21 | A script that writes one stderr line and exits 1 |

### Mock Requirements

The cases spawn the real binaries with real fixtures. ST-20 and ST-21 replace the process `PATH`
for the child, so they exercise the environment and ICE paths without relying on a broken SDK.
The SDK toolchain is otherwise present because the suites run inside the pinned container (same
assumption as `clorraine_driver_spec_test.rs`).

## Verification Checklist

- [ ] All 21 specification test cases (ST-1 – ST-21) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase per phase: ST-1 – ST-7
  before Phase 1's code, ST-8 – ST-15 and ST-20 – ST-21 before Phase 2's, ST-16 – ST-19 before
  Phase 3's)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] `make check`, `make verify`, and `make demo-check` pass
- [ ] No regressions in existing tests (the default human output is unchanged, R6)
