# Method and Evidence: Independent Assessment of Clorraine

> **Document**: 03-01-method-and-evidence.md
> **Parent**: [Index](00-index.md)

## Overview

This document owns how the assessment works: ground rules, evidence standard, severity, probe
layout, the finding template, measurement rules, the escalation path, and the verification
protocol. The workstream checklists (03-02, 03-03) and the report (03-04) cite it.

## Ground rules

| Rule | Detail | AR Ref |
|------|--------|--------|
| Read-only | No edit to `compiler/`, `stdlib/`, `spec/`, `tools/`, `docker/`, `examples/`, or any other tracked file outside `plans/independent-assessment/`; `plans/00-roadmap.md` changes only at synthesis | AR #12, AR #13 |
| No gate reruns | `make check`, `make verify`, `make asan`, `make ubsan` are not run | AR #3 |
| Allowed runs | One baseline run each of `make bench`, `make bench-compile`, `make bench-flags`; targeted harness reruns; single-probe builds with `-fsanitize=address` or `-fsanitize=undefined`; `vamos`; FS-UAE for a named probe | AR #14, AR #6 |
| Pinned baseline | Every `file:line` and every number refers to the SHA in `findings/WS0-baseline.md` | AR #7 |
| Bounded probes | Each command runs under `timeout` with `ulimit -f 1048576`; outputs go to `build/tmp/assessment/<WS>/` | AGENTS.md §Testing |
| Container | Toolchain commands run through `make -C docker exec CMD="..."` with no nested quotes; longer sequences go into a probe script under `probes/<WS>/` | AGENTS.md §Commands |
| Effort | Per phase, up to `xhigh`; `max` is never requested | AR #4, AR #19 |

### Independence

An auditor first forms a view from the code and `spec/`. Only then does it read earlier plans,
`05-findings.md` files of earlier plans, the DEF ledger, and `docs/known-limitations.md`, for
three purposes: to detect a regression of a closed defect, to classify recurrence, and to
avoid filing a documented limitation as new. A "done" claim in an earlier plan is not evidence.
A conclusion an earlier plan measured is reopened only with a new measurement that contradicts
it.

### Spec oracle

`spec/` is the oracle. Code that disagrees with `spec/` is a finding against the code. When
the spec itself looks wrong, ambiguous, or silent, the item is recorded under
`## Spec questions` in the workstream file, never as a code finding, and goes to the user for
a ruling. No spec file is edited.

### Documented limitations

A behavior listed in `docs/known-limitations.md` is not a new defect. It may still be a
finding when its stated impact is wrong or its cost is measured to be higher than the
document says; the finding cites the limitation.

## Evidence standard

Every finding carries all fields of §Finding template. Evidence is one of:

| Kind | Required content |
|------|------------------|
| Reproduction | Committed probe path, exact command, expected outcome (with its source), observed outcome |
| Measurement | Command, profile, flags, baseline SHA, raw JSON or log path, both numbers of every comparison |
| Code reading | `file:line` range plus the reasoning chain; allowed only for `probable` or `hypothesis` confidence |

### Confidence

| Label | Meaning |
|-------|---------|
| `verified` | Reproduced from committed probe sources at the baseline SHA, or measured |
| `probable` | Strong code-reading evidence, no reproduction yet |
| `hypothesis` | Plausible from reading; not ranked above Minor (AR #15) |

### Falsifiability

Before a probe's result counts, its author shows the probe can fail: either a mutated
expectation is reported as a mismatch, or a variant of the input that must behave differently
does. This applies AGENTS.md's test-integrity rule ("an assertion must fail without the fix")
to evidence. The falsifiability record is one line per probe in the workstream file.

## Severity

| Severity | Definition | AR Ref |
|----------|------------|--------|
| 🔴 Critical | An accepted program miscompiles, is memory-unsafe, or emits invalid C; a crash or hang on a real Kickstart; data loss | AR #15 |
| 🟠 Major | Latent undefined behavior in emitted C; a shim leak or missing failure path; a wrongly rejected valid program; ≥10% net m68k instructions or `.text` on a benchmark versus the stated comparison | AR #15 |
| 🟡 Minor | Edge-case divergence with low impact; maintainability debt that already caused or will plausibly cause defects | AR #15 |
| 🟢 Opportunity | A measured improvement with no defect behind it, below the Major threshold | AR #15 |

A `hypothesis` is never above Minor. A finding whose impact differs per target lens takes the
higher severity and states both lenses.

## Probe layout

```
plans/independent-assessment/probes/
├── WS0/ref/<benchmark>.c          # hand-written C references (AR #5)
├── WS<n>/<NNN>-<slug>.clo         # Clorraine probe
├── WS<n>/<NNN>-<slug>.c           # C probe or patched emitted C (diff form preferred)
├── WS<n>/<NNN>-<slug>.expected    # expected outcome and its source, written first
└── WS<n>/<NNN>-<slug>.sh          # optional runner; no network, bounded, writes under build/tmp
```

Probe sources stay small and self-contained. Patched emitted C is stored as a unified diff
against the baseline emission, not as a full copy. Outputs, binaries, and traces stay under
`build/tmp/assessment/` and are not committed (AR #11).

## Finding template

Each workstream file `findings/WS<n>-<slug>.md` has these sections: `## Scope and limits`,
`## Checklist outcomes`, `## Findings`, `## Spec questions`, `## Falsifiability log`,
`## Escalations`. A finding is written as:

```markdown
### ASM-WS<n>-<NNN> — <short title>

- **Severity**: Critical | Major | Minor | Opportunity
- **Confidence**: verified | probable | hypothesis
- **Lens**: host | A500 (68000/KS1.3) | A1200 (68020/AGA) | all
- **Location**: `path:line[-line]` at <baseline SHA>
- **Checklist item**: <03-02/03-03 item ID>
- **Evidence**: <probe path + command + expected (source) + observed, or measurement record>
- **Impact**: <what goes wrong, for which programs, how often>
- **Root cause**: <mechanism, not symptom>
- **Recommendation**: <fix direction; alternatives if more than one is viable>
- **Effort**: S | M | L | XL
- **Related**: <DEF-n, earlier plan, other ASM IDs, known-limitations entry>
```

Effort scale: S ≤ ½ day, M ≤ 2 days, L ≤ 1 week, XL > 1 week.

## Measurement rules

- Use the harness and shipped flags (`02-current-state.md` §Toolchain flags) unless the
  finding is about flags; then state both flag sets.
- Instruction counts are net of the harness calibration, as `tools/perf/bench.py` reports.
- `.text` compares like with like: same profile, same flags, same compiler.
- A1200 numbers come from a `-m68020` rebuild of the same emitted C, run under `vamos` with a
  68020 CPU; they are always labelled "68020 probe build" (AR #17). If `vamos` lacks the CPU
  option, Phase 0 records the fallback (FS-UAE A1200 timing).
- Optimizer estimates patch the emitted C in `build/tmp`; the program output must stay
  byte-identical, and the diff is committed as the probe (AR #17).
- Every number in a finding or the report cites its record in `findings/WS0-baseline.md` or
  the workstream file.

## Escalation

When a finding reaches `verified` Critical, its author: records it under `## Escalations`
with a timestamp from `date '+%Y-%m-%d %H:%M'`; tells the user in the session at once with the
ID, one-line impact, and reproduction command; and continues the workstream. No fix is made
(AR #12).

## Agent routing

| Workstream | Primary | Supporting |
|------------|---------|------------|
| WS1 Semantic core | `semantics-reviewer` | `correctness-reviewer` |
| WS2 Ownership/borrow/drop | `memory-safety-auditor` | `semantics-reviewer` |
| WS3 IR and codegen correctness | `emission-integrity-auditor` | `memory-safety-auditor` |
| WS4 Code quality and optimizer | `performance-auditor` | `emission-integrity-auditor` |
| WS5 Runtime shims | `amiga-platform-reviewer` | `memory-safety-auditor` (host shim) |
| WS6 Standard library | `correctness-reviewer` | `amiga-platform-reviewer` |
| WS7 Performance | `performance-auditor` | — |
| WS8 Test integrity and defect history | `correctness-reviewer` | — |
| Verification (Phase 9) | a fresh session of the primary role for each workstream, never the author session | the supporting role |
| Report review (Phase 10) | `correctness-reviewer` | — |

The executing session may do the work itself when an agent is unavailable; independence is
then preserved by Phase 9 running in a different session from the author (AR #16).

## Verification protocol (Phase 9)

For every finding, the verifier:

1. Re-runs the reproduction or measurement from the committed sources at the baseline SHA.
2. Tries to refute it: checks `spec/`, `docs/known-limitations.md`, earlier plans, and
   whether the observed behavior is intended.
3. Checks the severity against §Severity and the confidence against §Confidence.
4. Checks for duplicates across workstreams and merges them under the lowest ID.
5. Records one verdict: `confirmed`, `refuted`, `downgraded <from→to>`, `upgraded <from→to>`,
   or `merged into <ID>`, with a one-line reason.

Refuted findings stay in `05-findings.md` with their verdict; they never reach the report's
ranked list.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A probe cannot be built or run in the container | Record a limit under `## Scope and limits` with the error; never infer the result | AR #10 |
| FS-UAE or licensed media unavailable | Record the limit; leave the finding at `probable` | AR #6 |
| A measurement is noisy across runs | Report the instruction count (deterministic) rather than host wall time; host time only with ≥5 runs and the spread | AR #14 |
| The baseline SHA changes mid-assessment | Stop; ask the user whether to re-pin; never mix SHAs | AR #7 |
| A finding needs a source change to be proven | Prove it with a probe or a patched copy under `build/tmp`; never edit the tree | AR #12 |
| An agent session crashes mid-workstream | Resume from the execution plan and the on-disk findings file | AR #16 |
