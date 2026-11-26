# Clorraine independent assessment — runbook

This folder holds what is needed to run the independent assessment of the compiler, its
emitted C, the runtime shims, the standard library, and performance again. An assessment
is a read-only audit: it changes no compiler, runtime, standard-library, or specification
source. Its outputs are committed probe sources, per-workstream findings, an independently
verified findings ledger, a report with a remediation roadmap, and GitHub issue hand-offs.

The first run is `plans/independent-assessment/` (baseline `e24a8752`, report
`10-assessment-report.md`). Its folder is evidence and is never rewritten; start new runs
from the template here.

## Contents

| Path | What it is |
|------|------------|
| `template/` | The plan: `00-index.md`, `00-ambiguity-register.md` (the user's rulings as standing defaults), `01-requirements.md`, `02-current-state.md` (the previous run's facts; Phase 0 re-records them), `03-01`…`03-04`, `07-testing-strategy.md`, `99-execution-plan.md` with every checkbox cleared, and `LESSONS.md`. Placeholders: `<PLAN>` (the run folder name), `<BASELINE>` (the commit), `<DATE>` |
| `drift-check.sh` | `drift-check.sh <plan-folder> [<baseline-sha>]` — fails when anything outside `plans/`, `tools/assessment/`, and the assessment skill changed since the baseline, or anything under `plans/` other than the run folder and the roadmap |
| `measure.py` | The Phase 0 measurement matrix: `measure.py <plan-folder> <baseline-sha> --clc <clc> --out <dir>`; every benchmark, Clorraine and C, on every lens, with output checks and `vamos` instruction traces |
| `emit-all.sh` | The Phase 0 emission sweep over the corpus, benchmarks, and `examples/amiga/` |
| `ref/` | The equivalent C references and `calibration.c` (`ref_io.h` is their output helper) |

## Full mode

Assesses every workstream from a fresh baseline.

1. **Settle the tree.** Land or stash pending work; the run pins one commit.
2. **Create the run folder** and fill the placeholders:

   ```sh
   new=assessment-$(date +%Y-%m-%d)
   mkdir -p plans/$new/probes/WS0
   cp tools/assessment/template/*.md plans/$new/
   cp -r tools/assessment/ref tools/assessment/drift-check.sh \
         tools/assessment/measure.py tools/assessment/emit-all.sh plans/$new/probes/WS0/
   sed -i "s/<PLAN>/$new/g; s/<DATE>/$(date +%Y-%m-%d)/g" plans/$new/*.md
   ```

   `<BASELINE>` stays until task 0.1.1 records the commit. Read `LESSONS.md` before writing
   probes; the first run's probes under `plans/independent-assessment/probes/` (for example
   the shared runner `probes/WS1/probe.py`, the FS-UAE runner `probes/WS5/54x-fs-uae.sh`,
   the walker, and the classifiers under `probes/WS8/`) may be copied as starting points.
3. **Standing rulings.** The register's rulings apply unchanged; settle only AR #7 (the
   settled tree and baseline), AR #30 (commit and push mode), and AR #28 (the label set).
4. **Roadmap row.** Add a `PLAN-<n>` row in `plans/00-roadmap.md` at `Plan Created`.
5. **Preflight** the folder with the `preflight` skill; resolve every critical and major
   finding.
6. **Execute** with `exec-plan <new>`. Every gate runs
   `bash plans/<new>/probes/WS0/drift-check.sh plans/<new>`.

## Scoped mode

The full mode with fewer workstreams. After step 2, delete the phases of the workstreams
not wanted from `99-execution-plan.md`, their rows in `07-testing-strategy.md` §Check
points, and their checklists in `03-02`/`03-03`; keep Phase 0 (baseline and matrix),
Phase 9 (verification), and Phase 10 (report and issues). Record the chosen scope in the
register as a run-specific ruling.

## Delta mode

Assesses only what changed since the previous report.

1. Take the previous run's baseline (its `findings/WS0-baseline.md`) and list the changed
   paths: `git diff --name-only <previous-baseline>..HEAD`.
2. Map them through the table below to the workstreams to run (scoped mode).
3. Before new work, re-verify the previous run's surviving findings in the selected
   workstreams against the new baseline: a fixed finding closes its DEF row and issue; a
   surviving one carries over with its ID.

| Changed path | Workstreams |
|--------------|-------------|
| `compiler/src/lex`, `parse`, `ast`, `load`, `resolve`, `sema`, `types`, `consts`, `analysis.rs`, `driver.rs`, `diag*` | WS1 |
| `compiler/src/own*`, `borrow.rs` | WS2 (and WS8 for its DEF history) |
| `compiler/src/ir*`, `codegen*`, `abi.rs`, `ffi.rs`, `init_graph.rs` | WS3, WS4 |
| `compiler/src/profile.rs`, `stdlib.rs` | WS1 (gating), WS6 (embedding) |
| `compiler/runtime/*` | WS5 (and WS6 for wrappers over the changed functions) |
| `stdlib/*` | WS6 (and WS5 for `std.amiga.*` shims) |
| `benchmarks/`, `tools/perf/`, release flags in `compiler/src/clorraine/toolchain.rs` | WS4, WS7 |
| `compiler/tests/`, `tools/fuzz/`, `docker/scripts/*smoke*`, `docs/amiga-sdk/*.png` | WS8 |
| `spec/` | every workstream whose chapters changed (re-check the oracle first) |
| `compiler/src/lsp/`, `compiler/src/clorraine/` (other than flags), `tools/demo/`, `tools/vscode/`, `site/` | out of scope (AR #9) |

## Labels and issue filing

Issues are created only after the user approves the full list (AR #27), on
`OpenLorraine/clorraine`, with these labels (create only missing ones; never edit existing):
`assessment`; `kind:finding|tracking|theme`; `severity:critical|major|minor|opportunity`;
`priority:P0`–`P3`; `domain:semantics|ownership|codegen|optimizer|shims|stdlib|perf|tests`;
plus the existing `tier:core`, `tier:advanced`, `bug`, `performance`, `enhancement`,
`amiga-sdk` where they apply. Order: theme parent issues, then finding issues with
`gh issue create --parent <theme#>`, then one Minor tracking issue per workstream. Search
open issues for overlaps first and propose a link or comment instead of a duplicate. Draft
bodies under `plans/<run>/issues/` and grep them for `~/`, `/home/`, `.rom`, and `rom.key`
before showing the list. The body template is in `template/03-04-report-and-handoff.md`.

## Ground rules that every mode keeps

- Never run `make check`, `make verify`, `make asan`, or `make ubsan` as part of the
  assessment; measurement runs only (AR #3).
- Probe outputs, binaries, and traces stay under `build/tmp/assessment/`; scratch never
  goes to `/tmp`.
- Licensed Kickstart and Workbench media never enters the repository; decrypted ROMs live
  under `build/tmp` only for the duration of a run.
- FS-UAE runs use a private Xvfb display, never the user's desktop.
- A probe's expectation is committed before the probe runs.
- Nothing is fixed during an assessment; Criticals are escalated to the user at once.
