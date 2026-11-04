# Requirements: Clorraine DX Tooling

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

The DX slice of PLAN-05 ("Amiga-developer-first DX"). Three deliverables that make the toolchain
friendlier without touching language semantics: a `--explain` mode that turns a diagnostic code
into its registry entry, machine-readable diagnostics (`--diagnostics=json`) for editors and CI,
and a canonical, complete TextMate grammar for editor highlighting. PLAN-05's remaining surfaces
(LSP, VSCode extension packaging, SDK installer, watch mode) stay backlog (AR-1).

## Functional Requirements

### Must Have

- [ ] R1 — `clc --explain <CODE>` and `clorraine --explain <CODE>` (and the `--explain=<CODE>`
  joined form) print the code's registry entry to stdout and exit 0, without requiring a profile
  or an input file (AR-2, AR-3, AR-4).
- [ ] R2 — An unknown code prints `usage error: unknown diagnostic code '<code>'` on stderr and
  exits 2; a missing value prints `usage error: --explain needs a value` and exits 2 (AR-3).
- [ ] R3 — `--diagnostics=json` on both binaries renders every diagnostic as one JSON object per
  line on stderr (AR-5). Any other value is a usage error.
- [ ] R4 — Each object carries `code, severity, message, help, file, line, column, endLine,
  endColumn` in that order, 1-based positions, `null` where a field is absent (AR-6).
- [ ] R5 — Under JSON mode, every driver-level error (usage, environment, internal compiler
  error) is one object with `code: null` and `severity: "error"` (AR-7).
- [ ] R6 — Without the flags, the human output, stream choice, and exit codes are byte-identical
  to today (AR-5).
- [ ] R7 — `spec/14-diagnostics.md` gains the twelve approved help texts; no other spec edit
  (AR-10).
- [ ] R8 — `editors/clorraine.tmLanguage.json` is the canonical grammar; the demo toolkit
  consumes that file and no duplicate exists (AR-8).
- [ ] R9 — The grammar covers every `keyword` and `reserved_builtin` term of
  `spec/grammar.ebnf.md` and keeps `scopeName: source.clorraine` and `fileTypes: ["clo"]`
  (AR-9).

### Should Have

- [ ] R10 — Both CLIs' help surfaces document the new flags (`clorraine --help`; `clc` has no
  help text today and gains none) (AR-1, AR-3).
- [ ] R11 — The JSON schema is additive-only: consumers ignore unknown keys, and a future
  breaking change would introduce a new `--diagnostics` value rather than reshaping an object
  (data-and-migration lens).

### Won't Have (Out of Scope)

- LSP server, VSCode extension packaging, SDK fetch/detect installer, watch mode (AR-1).
- Output formats other than `human` and `json` (AR-5); nearest-code suggestions (AR-13).
- `--help` for `clc`; vim syntax changes; grammar generation from the EBNF (AR-1).
- Any edit to the compiler pipeline, checker rules, codegen, or emitted ABI.

## Technical Requirements

### Compatibility

- The default human renderer and every existing exit code stay unchanged (R6); the new flags are
  additive.
- The JSON object's key set and key order are fixed (AR-6); escaping keeps one diagnostic on one
  line (AR-15).
- A registry edit that breaks the parse must fail `make check` (AR-14) — the parse-guard tests
  pin the code count and representative rows.

### Security

- No new input surface beyond CLI arguments: the JSON writer escapes `"`, `\`, and control
  characters so no diagnostic content can break the stream; driver messages are sanitized before
  rendering as they are today (AR-15).
- Authentication, authorization, rate limiting, secrets management, and encryption in transit or
  at rest are `N/A`: the toolchain has no server, network, credential, or stored-data surface
  (invalid-argument handling and the parse-guard tests cover the only untrusted input, AR-3,
  AR-14).

### Performance

- No measurable effect: the registry parse and JSON render run once per process invocation on
  data smaller than 100 KB.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Deliverables | three-deliverable DX slice / full PLAN-05 | DX slice | Bounded, user-confirmed; installer/LSP stay backlog | AR-1 |
| Explain binaries | both / `clorraine` only / `clc` only | both | `clc` owns the registry; `clorraine` is the entry point | AR-2 |
| Explain invocation | flag / subcommand | flag | Matches the roadmap wording; early-exit like `--version` | AR-3 |
| Explain text source | embedded registry / Rust table | embedded registry | Single source of truth; no drift | AR-4 |
| Machine-readable | `--diagnostics=json` NDJSON / array on stdout / boolean | NDJSON on stderr | stdout stays clean; exit codes untouched | AR-5 |
| JSON fields | with / without end position | with end position | Editors underline ranges; avoids a later breaking change | AR-6 |
| Driver errors | JSON with null code / human only | JSON with null code | Uniformly parseable stream | AR-7 |
| Grammar home | `editors/` / `tools/grammar/` / keep demo | `editors/` | One canonical file; demo consumes it | AR-8 |
| Grammar truth | EBNF productions / lexical chapter | EBNF productions | Machine-extractable normative source | AR-9 |
| Grammar test | Rust suite / demo only | Rust suite in `make check` + demo test stays | Main verification covers it | AR-9 |

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved
> it. See `00-ambiguity-register.md`.

## Acceptance Criteria

1. [ ] All specification and implementation tests pass; `make check`, `make verify`, and
   `make demo-check` are green.
2. [ ] `clorraine --explain E10034` and `clc --explain=E10034` render the committed registry text
   for that code; unknown codes exit 2.
3. [ ] `--diagnostics=json` emits exactly the schema in 03-02 for errors, warnings, and driver
   errors; default output is byte-identical.
4. [ ] Every `keyword`/`reserved_builtin` term appears in `editors/clorraine.tmLanguage.json`;
   the demo toolkit loads it from that path.
5. [ ] All 114 registry rows carry a help text.
6. [ ] Documentation updated (AGENTS.md commands and layout).
7. [ ] Phase reviews recorded; roadmap synced.
