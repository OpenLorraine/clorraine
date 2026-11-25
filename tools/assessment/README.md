# Clorraine independent assessment — runbook

This folder holds what is needed to run the independent assessment of the
compiler, its emitted C, the runtime shims, the standard library, and
performance again. An assessment is a read-only audit: it changes no compiler,
runtime, standard-library, or specification source. Its outputs are committed
probe sources, per-workstream findings, an independently verified findings
ledger, a report with a remediation roadmap, and GitHub issue hand-offs.

> **Status: stub.** This version documents only the full mode, so a repeat is
> possible even if the first run stalls. The scoped and delta modes, the
> path-to-workstream table, the reusable scripts and C references, and the
> plan template are added when the first run finishes.

## Full mode

A full run assesses every workstream from a fresh baseline.

1. **Settle the tree.** Land or stash pending work. The run pins one commit,
   and any later change outside the run's own paths fails its drift check.
2. **Create the run folder.** Copy the plan documents of the most recent
   assessment run into a new folder, keeping only the planning set:

   ```sh
   new=plans/assessment-$(date +%Y-%m-%d)
   prev=plans/independent-assessment   # the most recent run
   mkdir -p "$new/probes/WS0"
   cp "$prev"/00-index.md "$prev"/00-ambiguity-register.md \
      "$prev"/01-requirements.md "$prev"/02-current-state.md \
      "$prev"/03-0*.md "$prev"/07-testing-strategy.md \
      "$prev"/99-execution-plan.md "$new"/
   cp -r "$prev"/probes/WS0/ref "$prev"/probes/WS0/drift-check.sh \
      "$prev"/probes/WS0/measure.py "$prev"/probes/WS0/emit-all.sh \
      "$new"/probes/WS0/
   ```

   Do not copy `findings/`, `05-findings.md`, `10-assessment-report.md`,
   `issues/`, the preflight report, or any other workstream probes: they are
   the previous run's evidence.
3. **Reset progress.** In the new `99-execution-plan.md`, clear every task
   checkbox to `[ ]`, set `Progress` to `0/<total>`, clear every
   `Phase baseline tree` line, and replace every
   `plans/independent-assessment` path with the new folder. In
   `00-index.md`, set the status and dates. In `02-current-state.md`, mark the
   recorded facts as the previous run's; Phase 0 re-records them.
4. **Keep the standing rulings.** The previous run's ambiguity register holds
   the user's rulings (scope, lenses, no gate reruns, effort ceiling, severity
   rubric, issue filing). Keep them as standing defaults; reopen only those
   that the new run must decide again (the baseline, the commit and push
   mode).
5. **Add the roadmap row.** Add a `PLAN-<n>` row for the new run in
   `plans/00-roadmap.md` at the `Plan Created` stage.
6. **Preflight.** Run the `preflight` skill on the new folder and resolve every
   critical and major finding.
7. **Execute.** Run `exec-plan assessment-<date>`. Phase 0 asks for the
   settled-tree confirmation, records the baseline in
   `findings/WS0-baseline.md`, and from then on every gate runs

   ```sh
   bash plans/assessment-<date>/probes/WS0/drift-check.sh plans/assessment-<date>
   ```

   The measurement matrix is produced inside the SDK container with

   ```sh
   make -C docker exec CMD="python3 plans/assessment-<date>/probes/WS0/measure.py plans/assessment-<date> <baseline-sha> --clc build/tmp/assessment/baseline/clc"
   ```

## Ground rules that every mode keeps

- Never run `make check`, `make verify`, `make asan`, or `make ubsan` as part of
  the assessment; measurement runs only.
- Probe outputs, binaries, and traces stay under `build/tmp/assessment/`.
- Licensed Kickstart and Workbench media never enters the repository.
- Issues are created on GitHub only after the user approves the full list.
