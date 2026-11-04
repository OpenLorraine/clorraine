# Findings: Clorraine DX Tooling

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Done

This ledger records the pre-execution plan refresh, the red/green evidence for each phase, and the
phase reviews. A finding is closed only when its evidence is written here and the named check is
green.

## Pre-execution refresh (2026-11-03)

The plan was authored on 2026-10-10. Before execution it was re-grounded against the current tree.
All findings below are factual corrections; none changes a resolved user decision.

| # | Finding | Evidence | Severity | Resolution |
|---|---------|----------|----------|------------|
| D1 | The live registry has 114 rows, not 112 | `spec/14-diagnostics.md`; `E10308` and `E10309` were added after commit `2537a82` | Major (planned guard would fail) | Counts updated in `01`, `02`, `03-01`, `07`, `99`, AR-14 |
| D2 | `clc` has five diagnostic render sites, not four | `driver.rs:263`, `:292`, `:299`, `:323`, `:337`; the parse-only site at `:292` post-dates the plan | Major (JSON could leak human text) | `02-current-state`, `03-02`, and the implementation route every site through one dispatcher |
| D3 | `clorraine` has fourteen user-facing error prints, not four | `mod.rs:45`, `:69`, `:140`, `:162`, `:172`, `:193`, `:216`, `:254`, `:264`, `:269`, `:274` plus `run.rs:52`, `:65`, `:82` | Major (R5 under-routing) | `02-current-state`, `03-02`; implementation routes every print through one formatter, including `run.rs` |
| D4 | No end-to-end case covered the R5 environment or ICE categories | `07-testing-strategy.md` had ST-10/11 (usage) and ST-15 (build diagnostics) only | Major (untested contract) | ST-20 and ST-21 added; both drive the real binary |
| D5 | ST-1 pinned an old E10034 help text | Registry now says "a `borrow T?` slot may hold its own incoming value, a static view, or a newly owned value" | Minor | ST-1 updated |
| D6 | ST-10 put the `clorraine` flag before the command word, which the CLI rejects | `cli.rs:123-130` requires a command first | Minor | ST-10 invocation corrected to `clorraine run --diagnostics=json` |
| D7 | The EBNF has twelve reserved built-ins, not eleven | `spec/grammar.ebnf.md` §10.4 | Minor | `02-current-state`, `03-03` updated |
| D8 | The roadmap close-out said "Plan Created", which was already true | `plans/00-roadmap.md` PLAN-05 | Minor | DX task 4.1.3 now sets PLAN-05 to Done |
| D9 | The JSON fixtures omitted the required `module` declaration, so the control case reported E11105 and the `null` local only warned | `parse/stmt.rs` requires a module first; `spec/tests/compile-fail/E10012_null_where_nonnull.clo` shows the valid shape | Major (tests would pin the wrong program) | Fixtures now declare a module; `E10012` reports on the `let` at 4:5 and `W10000` on the local at 4:9 |
| D10 | ST-9 expected `"help":"remove the binding or use it"` for W10000, but the compiler's warning carries no help line | `own/statements.rs:999` emits W10000 without help; the human render has none; 03-02 defines JSON `help` as "the help line, when present" | Major (oracle contradicted the design) | ST-9/ST-14 now expect `"help":null`; the registry help remains reachable through `--explain W10000` |

## Red phase

### Phase 1 (ST-1 – ST-7)

`make -C docker exec CMD="cargo test --test dx_tooling_spec_test"` on 2026-11-03:
**0 passed, 7 failed**. Every case failed with `usage error: unknown option '--explain'`; the two
exit-code-only cases (ST-4, ST-5) also failed their exact-stderr assertions. The implementation
module did not exist yet.

### Phase 2 (ST-8 – ST-15, ST-20 – ST-21)

`make -C docker exec CMD="cargo test --test dx_tooling_spec_test"` on 2026-11-03:
**7 passed, 10 failed**. The seven Phase 1 cases passed. Every Phase 2 case failed: nine with
`usage error: unknown option '--diagnostics=json'`, and ST-12 with `E11105` because its fixture
still lacked the required module declaration (D9). ST-8's first run also exposed D10 (the W10000
help assumption). Both fixture/oracle defects were corrected before the implementation; after
that correction ST-12 is the deliberate human control that passes on the base tree.

### Phase 3 (ST-16 – ST-19)

`make -C docker exec CMD="cargo test --test dx_tooling_spec_test"` on 2026-11-03:
**20 passed, 4 failed**. All four grammar cases failed: `editors/clorraine.tmLanguage.json` did not
exist, the demo-owned copy still existed, and the old grammar missed EBNF terms and the arrow
surface. The three support-scanner unit tests passed, confirming the test infrastructure itself
compiled and ran.

## Phase reviews

### Phase 3 review (2026-11-03)

One independent read-only correctness review covered the Phase 3 diff. It found **no critical or
major findings** and one minor test-integrity gap:

| # | Severity | Finding | Disposition |
|---|----------|---------|-------------|
| RV-001 | Minor | `word_bounded` split on `\` but kept the following `b`, so a standalone `\bdeleter\b` pattern would not be seen as the term `deleter` | Fixed: boundary escapes are normalized before tokenizing, and the unit test now covers `\bdeleter\b` |
| P-1 | Minor (inherited) | The moved grammar listed `i64`/`u64`, which the frozen EBNF does not define as scalar types | Fixed: both names are removed; the EBNF completeness test is unaffected |

The review independently parsed the EBNF, confirmed all 63 terms, checked that removing `do`
fails ST-16, that breaking `word_bounded` fails its unit tests, and that the base tree fails all
four grammar cases. The demo suite passed 78/78 (1 skipped) in the Node 22 container.

Re-verification after the fixes: spec 24/24, impl 12/12, demo 78/78, `make check` green.

## Close-out (2026-11-03)

| Gate | Result |
|------|--------|
| `make check` | exit 0; `./spec/check.sh` OK (27 files, 111 compile-fail tests); 91 `test result: ok` groups |
| `make verify` | exit 0; `verify: all legs passed` (check, differential, corpus-fs-uae, run-fault-amiga; the six runtime faults match) |
| `make demo-check` | exit 0; 78 passed, 1 skipped, 0 failed (`tsc --noEmit` and `node --test`) |
| DX suites | spec 24/24; impl 12/12; registry/diag unit tests 19/19 |
| `spec/14-diagnostics.md` re-review | **no findings**; all 12 help cells checked against their owning chapter; tables are valid four-column Markdown; status remains `stable (v1.0)` |
| `./spec/check.sh` after the edit | OK |

The host has no `node`/`npm`, so the exact `make demo-check` target was run with a temporary
Node 22.23.4 toolchain on `PATH`; the command and its assertions were unchanged. This is an
environment note, not a plan deviation.

The new source and test comments carry no plan or case identifiers; the case labels are plain
descriptions of the behaviour they pin.

### Phase 1 review (2026-11-03)

Two independent read-only reviews (correctness and semantics) covered the Phase 1 diff. Both
reported **no critical or major findings**. The six minor findings and their dispositions:

| # | Finding | Disposition |
|---|---------|-------------|
| RV-001 / SR-002 | The new `clorraine --explain` unknown-code path echoed the raw argument, bypassing the driver's `display::sanitize` convention | Fixed: the code is sanitized; an implementation test feeds an ESC byte and asserts one inert line |
| RV-002 | `write_fixture` was unused and produced a dead-code warning | Fixed: ST-7 now writes an invalid `main.clo` and proves explain never reads it |
| RV-003 | The red-phase run was not written down | Fixed: recorded above |
| RV-004 | The two edited registry tables still declared three columns, hiding the new Help cells in rendered Markdown | Fixed: both headers gained `| Help |`; the twelve cells keep their approved content |
| SR-001 | The approved E90003 help says `` `deleter` functions ``, but `deleter` is not a keyword; `freedBy` is | Fixed minimally: the cell now says `` `freedBy` deleters ``. The broader `stores` workaround wording is an accepted residual for the Phase 4 spec re-review |
| — | `clc`'s usage errors echo raw arguments (pre-existing convention) | Accepted residual; unchanged by this phase and outside the DX contract |

Re-verification after the fixes: focused suites green, `make check` green, `./spec/check.sh` OK.

### Phase 2 review (2026-11-03)

One independent read-only correctness review covered the Phase 2 diff. It found **one major
finding** and four minor findings. The major was fixed before the phase closed:

| # | Severity | Finding | Disposition |
|---|----------|---------|-------------|
| RV-001 | Major | `run.rs` printed its spawn, wait, and capture-write errors directly, so JSON mode could emit human text on the run path | Fixed: `run_program` takes the format and routes all three through `format_error`; both callers pass it |
| RV-002 | Minor | Explain's unknown-code error stayed human while its missing-value error honoured JSON | Fixed: both binaries report the unknown code in the requested format; an implementation test covers the combined flags |
| RV-003 | Minor | DEL and C1 control characters passed through raw; the generic `\u` branch was untested | Fixed: the guard uses `char::is_control`, with tests for U+0001, U+007F, and U+009B |
| RV-004 | Minor | A doc comment referenced a plan identifier | Fixed: the rationale is restated in plain language |
| RV-005 | Minor | The Phase 2 red run was not in this ledger | Fixed: recorded above |
| RV-006 | Minor | The first red-phase note said the ST-12 control passed, but it failed on the stale fixture | Fixed: the record now states 7 passed/10 failed and names ST-12's E11105 failure; ST-12 passes as the control after D9 |

The review confirmed all five `clc` render sites and all three `build::build` callers route through
the dispatcher, the human control is byte-identical, and ST-20/ST-21 reach the environment and ICE
paths rather than exiting earlier.

Re-verification after the fixes: focused suites green, `make check` green, `./spec/check.sh` OK.
