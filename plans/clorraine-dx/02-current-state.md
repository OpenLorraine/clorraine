# Current State: Clorraine DX Tooling

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

Both CLIs already render diagnostics as the frozen human format (`error CODE: message`, an
indented `--> path:line:col`, and an optional `help:` line) from the same `Diagnostic` values.
The normative registry — 112 code rows with a trigger, a message template, and (for all but
twelve) a help text — lives in `spec/14-diagnostics.md`. The demo toolkit owns a 134-line TextMate
grammar that covers a subset of the language; the frozen EBNF declares exactly 50 keywords, one
contextual keyword, and eleven reserved built-in names.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/diag.rs` | Diagnostic values, collection, human rendering (`Diagnostic` at `:23`, `Diagnostics` at `:80`, `render` at `:188`) | Add the JSON renderer, the format enum, escaping, and the null-code driver-error line |
| `compiler/src/driver.rs` | `clc` options (`Options::parse` at `:40`, `run` at `:236`) and every error print (`:240`, `:250`, `:254`, `:263`, `:283`, `:307`, `:321`, `:327`) | Add `--explain` (early exit) and `--diagnostics=json`; route error prints through the format |
| `compiler/src/clorraine/cli.rs` | `clorraine` help text (`HELP` at `:65`), parse (`:100`), option parse (`:140`) | Add `--explain` (anywhere, like `-h`), `--diagnostics=json`, and the help lines |
| `compiler/src/clorraine/mod.rs` | Driver flow: `run_with_marker` (`:42`), `invoke` (`:67`), environment gate (`:85`), `emit` (`:123`), error prints at `:45`, `:69`, `:127`, `:141` | Add the Explain action; route error prints through the format; forward the flag |
| `compiler/src/clorraine/build.rs` | In-process compiler call (`crate::driver::run(args)` at `:72`); ICE display (`:28`) | Append `--diagnostics=json` to the compiler args; route the ICE text |
| `spec/14-diagnostics.md` | Normative registry: `\| Code \| Trigger \| First line \| Help \|` tables | Add the 12 approved help cells (AR-10) |
| `spec/grammar.ebnf.md` | Normative grammar: `keyword` (`:551`), `contextual_keyword` (`:567`), `reserved_builtin` (`:576`) | No change — it is the completeness oracle |
| `tools/demo/grammars/clorraine.tmLanguage.json` | Demo-owned grammar | Remove (moved to `editors/`) |
| `editors/clorraine.tmLanguage.json` | New canonical grammar | Create from the moved file, completed |
| `tools/demo/src/util/shiki.ts` | Grammar path at `:9` (`join(toolDir, "grammars", ...)`) | Point at the canonical file |
| `tools/demo/test/grammar.spec.test.ts` | Category-scope assertions over a fixture | Extend the fixture with the newly covered tokens |
| `compiler/tests/clorraine_driver_spec_test.rs` | Driver spec conventions: scratch dirs, `env!("CARGO_BIN_EXE_clc")`, `env!("CARGO_BIN_EXE_clorraine")` | No change — the new suites follow the same pattern |

### Code Analysis

- `Diagnostic.code` is `&'static str` and `span` is `Option<Span>`; `Span` carries `start`/`end`
  byte offsets (`span.rs:14-20`), and `SourceMap::location(id, offset)` (`span.rs:121`) yields
  1-based line/column — so the JSON end position is `location(file, span.end)`.
- `Diagnostics::render` (`diag.rs:188`) returns a `String` that the drivers `eprint!`; the JSON
  renderer mirrors that shape, so no stream handling changes.
- `clc` renders diagnostics in four places (`driver.rs:254`, `:283`, `:307`, `:321`); the other
  prints are plain `usage error:` / `clc: cannot ...` lines.
- `clorraine` calls the compiler in process with a constructed argument vector
  (`build.rs:72`), so JSON mode is a matter of appending `--diagnostics=json` and routing the
  driver's own prints.
- The demo grammar's keyword pattern
  (`"\\b(module|import|export|func|let|take|borrow|handle|deleter|...)\\b"`) misses real
  keywords — `as`, `do`, `in`, `from`, `freedBy`, `void`, `f32`, `f64`, `i16`, `u16`, `usize`,
  `isize`, `cstring` — and every reserved built-in (`sizeof`, `offsetof`, `len`, `embed`,
  `panic`, `assert`, the six `*Volatile*` names).

## Gaps Identified

### Gap 1: No explain mode

**Current Behavior:** an unknown flag exits 2; the registry is only readable as markdown.
**Required Behavior:** `--explain <CODE>` on both CLIs renders the registry entry (R1, R2).
**Fix Required:** a registry parse module plus flag plumbing (03-01).

### Gap 2: Human-only diagnostics

**Current Behavior:** `Diagnostics::render` emits the human text only.
**Required Behavior:** `--diagnostics=json` emits NDJSON on stderr (R3, R4).
**Fix Required:** a JSON renderer with escaping and a format enum (03-02).

### Gap 3: Driver errors have no machine-readable path

**Current Behavior:** `usage error:` / `clc:` / `clorraine:` / ICE messages are free text.
**Required Behavior:** one JSON object with `code: null` per driver error in JSON mode (R5).
**Fix Required:** a shared driver-error line plus routing at every print site (03-02).

### Gap 4: Grammar is incomplete and demo-owned

**Current Behavior:** the grammar misses EBNF terms and lives under `tools/demo/`.
**Required Behavior:** the canonical file under `editors/` covers every EBNF keyword and
reserved built-in; the demo consumes it (R8, R9).
**Fix Required:** move, complete, and test (03-03).

### Gap 5: Twelve registry rows carry no help text

**Current Behavior:** `E90000`, `E90001`, `E90003`–`E90005`, `W10000`–`W10006` lack a Help cell.
**Required Behavior:** every row has the approved help text (R7).
**Fix Required:** the authorized spec edit (AR-10).

## Dependencies

### Internal Dependencies

- `Diagnostics` and `SourceMap` (both already shared by the two binaries).
- The registry table shape in `spec/14-diagnostics.md` (the parse oracle).
- The EBNF productions `keyword` / `reserved_builtin` (the grammar oracle).
- The existing test conventions in `compiler/tests/clorraine_driver_spec_test.rs`.

### External Dependencies

- None for the Rust workspace (std-only). `make demo-check` runs the npm toolkit that consumes
  the moved grammar.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Registry table shape drifts and the parse breaks silently | Low | Med | Parse-guard tests pin the code count and representative rows; a malformed edit fails `make check` (AR-14) |
| A diagnostic message breaks the NDJSON stream | Low | High | Escape `"`, `\`, and control characters; tests cover a quote and a newline (AR-15) |
| The moved grammar breaks the demo toolkit | Med | Low | Update `shiki.ts` in the same task; `make demo-check` verifies (AR-8, AR-11) |
| `clorraine` fails to forward the format to the in-process compiler | Low | Med | Spec test runs `clorraine build` with a broken fixture under the flag |
| TextMate regexes misfire around generics `<` and captures `[` | Med | Low | Completeness is test-enforced per keyword; rendering fidelity is spot-checked through the demo fixture, not over-promised |
