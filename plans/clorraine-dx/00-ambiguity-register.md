# Ambiguity Register: Clorraine DX Tooling

> **Status**: ✅ GATE PASSED — all 15 items resolved
> **Last Updated**: 2026-10-10 19:09

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which deliverables constitute the DX batch (PLAN-05 has more: SDK installer, LSP, packaging, watch mode) | The three-deliverable DX slice (`--explain`, machine-readable diagnostics, TextMate grammar) / the full PLAN-05 surface | **User:** the three-deliverable DX slice; the installer, LSP, VSCode packaging, and watch mode stay backlog | ✅ Resolved |
| 2 | Integration | Which binary carries `--explain` | both `clc` and `clorraine` / `clorraine` only / `clc` only | **User:** both — `clc` owns the registry, `clorraine` is the developer entry point | ✅ Resolved |
| 3 | Behavioral | `--explain` invocation spelling, precedence, and failure behavior | `--explain <CODE>` flag / `explain <CODE>` subcommand; early-exit vs ordinary option | **User:** `--explain <CODE>` (and `--explain=<CODE>`), an early-exit mode like `--version`: honoured wherever it appears, ignores remaining arguments, needs no `--profile` or file; unknown code → `usage error: unknown diagnostic code '<code>'`, exit 2; missing value → `usage error: --explain needs a value`, exit 2; codes match exactly (uppercase, no normalization or suggestions) | ✅ Resolved |
| 4 | UX | What `--explain` prints and where the text comes from | section + trigger + message template + help / trigger + help only; embed `spec/14-diagnostics.md` via `include_str!` / hand-maintained Rust table | **User:** the full registry entry (severity, section, trigger, message template, help) rendered from the normative registry embedded with `include_str!` — one source of truth, no drift | ✅ Resolved |
| 5 | Behavioral | Machine-readable spelling, value set, transport, and binary surface | `--diagnostics=json` NDJSON on stderr / JSON array on stdout / `--json-diagnostics` boolean | **User:** `--diagnostics=json` (the only accepted value; any other value is a usage error) on both binaries; one JSON object per line on stderr; stdout, exit codes, and the byte-identical human output when the flag is absent stay unchanged | ✅ Resolved |
| 6 | Data | JSON object schema | include the end position / start position only | **User:** fixed key order `code, severity, message, help, file, line, column, endLine, endColumn`; 1-based line/column; `null` where a field is absent; the end position included so editors can underline a range without a later breaking change | ✅ Resolved |
| 7 | Behavioral | Whether the driver's own errors (usage, environment, ICE) are machine-readable too | yes with `code: null` / compiler diagnostics only | **User:** yes — each becomes one object with `code: null`, `severity: "error"`, and the full sanitized message | ✅ Resolved |
| 8 | Naming | Canonical home of the TextMate grammar | `editors/clorraine.tmLanguage.json` / `tools/grammar/...` / keep `tools/demo/grammars/` | **User:** `editors/clorraine.tmLanguage.json` (new top-level `editors/` directory); the demo toolkit consumes that one file; no duplicate copy anywhere | ✅ Resolved |
| 9 | Technical | Grammar completeness source of truth and test home | EBNF productions vs lexical chapter; Rust suite in `make check` vs demo toolkit tests only | **User:** the `keyword` and `reserved_builtin` productions in `spec/grammar.ebnf.md`; a new Rust suite under `make check`; the existing Shiki test in the demo toolkit stays | ✅ Resolved |
| 10 | Scope | The 12 registry rows that carry no help text (`E90000`, `E90001`, `E90003`, `E90004`, `E90005`, `W10000`–`W10006`) | author the missing help (authorized spec edit) / print the trigger only where help is absent | **User:** author the 12 help texts — the drafted texts in 03-01 §Registry Help Texts are the confirmed wording; this is the batch's only spec modification | ✅ Resolved |
| 11 | Non-functional | Verify commands for the plan | `make check` + `make verify` + `make demo-check` | **User:** confirmed — `make check` (fast), `make verify` (full close-out), `make demo-check` (the toolkit's grammar path changes) | ✅ Resolved |
| 12 | Naming | Plan folder and test suite names | `plans/clorraine-dx/` with `dx_tooling_spec_test.rs` / `dx_tooling_impl_test.rs` | **User:** confirmed the defaults | ✅ Resolved |
| 13 | Edge cases | `--explain` combined with `--diagnostics=json`; `--explain` with files or a profile; unknown-but-similar codes | explain wins (early exit) / reject the combination; suggest nearest code / no suggestions | **User:** confirmed defaults — explain wins and ignores the rest; no nearest-code suggestions | ✅ Resolved |
| 14 | Technical | Registry parse robustness (markdown table shape, section titles, backticks, missing help) | heading-tracked runtime parse with guard tests / precompiled table | **User:** confirmed the AR-4 embed decision; mechanism: track the latest `##` heading as the section, parse `\| code \| trigger \| \`template\` \| help \|` rows (4 or 5 cells), strip code-span backticks, derive severity from the `E`/`W` prefix, omit the `help:` line when absent; tests pin the parsed code count (112) and representative rows | ✅ Resolved |
| 15 | Technical | JSON stream integrity (escaping so one diagnostic is always one line) | hand-rolled escaped writer / rely on message content | **User:** confirmed defaults; mechanism: escape `"`, `\`, and control characters (`\n`, `\r`, `\t`, other < 0x20) in every string field; tests cover a message containing a quote and a newline | ✅ Resolved |

### Resolution Notes

**AR-1 (user):** the scope baseline was presented and confirmed ("Confirm all"). Excluded
explicitly: LSP, VSCode extension packaging, the fetch/detect SDK installer, watch mode, `--help`
for `clc`, vim syntax changes, grammar generation from the EBNF, any other registry/spec edit, and
any output format other than `human`/`json`.

**AR-4 (user):** the registry is embedded with `include_str!("../../spec/14-diagnostics.md")` (from
`compiler/src/diag/`), so the normative text ships inside the binary; a malformed registry edit
fails `make check` through the parse-guard tests.

**AR-10 (user):** authorized spec edit — exactly the 12 help cells below are added; nothing else
in `spec/14-diagnostics.md` changes:

| Code | Help text |
| ---- | --------- |
| E90000 | keep the program inside the v1 surface; a staged feature names its own code when it has one |
| E90001 | compute the value at run time; `comptime` is staged for a later release |
| E90003 | model the hand-off with explicit moves and `deleter` functions in v1 |
| E90004 | allocate each value separately and hand ownership off explicitly in v1 |
| E90005 | build the value in steps with intermediate `let` bindings |
| W10000 | remove the binding or use it |
| W10001 | declare the parameter without `borrow`; a duplicable value copies |
| W10002 | end the case with `break` or `return` |
| W10003 | remove the statement or restructure the control flow |
| W10004 | move large locals off the stack or split the function |
| W10005 | remove the table once the generated contract surface covers the target |
| W10006 | remove the import or use the name |

**AR-14 (user):** every registry row must be reachable from `--explain`; the parse treats a row
whose help cell is absent or empty as "no help" and renders the entry without a `help:` line, so a
future registry edit cannot break the mode.

**AR-15 (user):** the escaping rule is what keeps NDJSON parseable when a message embeds a newline
(for example a usage error echoing an argument) or the internal-compiler-error note text.

### Complexity check

No complexity escalation: the batch adds no dependency, no framework, no harness beyond ordinary
Rust tests in the existing suites, and no cross-cutting refactor. The two new modules (registry
parse, JSON render) are leaf code inside the existing `diag` and CLI paths. `Approved complexity:
None`.
