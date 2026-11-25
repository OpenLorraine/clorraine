# Method and Evidence: Independent Assessment of Clorraine

> **Document**: 03-01-method-and-evidence.md
> **Parent**: [Index](00-index.md)

## Overview

This document owns how the assessment works: ground rules, evidence standard, severity, probe
layout, the finding template, measurement rules, the escalation path, agent routing, and the
verification protocol. The workstream checklists (03-02, 03-03) and the report (03-04) cite it.

## Ground rules

| Rule | Detail | Ref |
|------|--------|-----|
| Read-only | No edit to any tracked file outside `plans/independent-assessment/`, except `plans/00-roadmap.md` as below; new files may be added only under `tools/assessment/` and `.opencode/skills/clorraine-assessment/` (AR #29) | AR #12, AR #29 |
| Roadmap | This plan's own row (stage, status, progress) is updated through the roadmap skill in any phase, as exec-plan does; exec-plan blocker `↳ DEF` rows are allowed in any phase; finding-derived DEF rows are added only in Phase 10 | AR #13, PF-003 |
| No gate reruns | `make check`, `make verify`, `make asan`, `make ubsan` are never run, including through a commit or phase verification | AR #3, PF-006 |
| Allowed runs | One baseline run each of the bench harness, compile-time harness, and flag audit, each with `--clc` set to the pinned `clc` and `--out` set to a new, uniquely named `build/perf/assessment-baseline-*.json` (the harnesses confine output to `build/perf/`; an existing result file is never overwritten), then copied to `build/tmp/assessment/baseline/`; the Phase 0 measurement script; single-probe sanitizer builds; `vamos`; FS-UAE for a named probe | AR #14, AR #6, PF-008, PB-001 |
| Baseline | Baseline = commit X recorded in `findings/WS0-baseline.md`. **Drift check** (`probes/WS0/drift-check.sh`), run at every gate and before every reproduction: `git diff --check`; `git diff --quiet X HEAD -- . ':(exclude)plans' ':(exclude)tools/assessment' ':(exclude).opencode/skills/clorraine-assessment'` and an empty `git status --porcelain` over the same pathspec; and no committed or working-tree change under `plans/` other than `plans/independent-assessment/` and `plans/00-roadmap.md` since X (`git diff --name-only X HEAD` and `git status --porcelain` over `plans` with those two paths excluded, both empty). "Reproduces at the baseline" means: at HEAD, with the drift check passing | AR #7, PF-005, PB-006 |
| Pinned tools | Phase 0 copies the release `clc` to `build/tmp/assessment/baseline/clc` with its sha256 and records the Docker image ID and the amitools commit; every probe and harness run uses that `clc`; the Docker image is not rebuilt during the assessment; bench and compile-time harness runs pass `--compiler-git X` (the flag audit has no such option, so X is recorded beside its output) | PF-005, PB-001 |
| Expectations first | Probes and expectations are committed at the end of the last Step N.1 writing task, before the red check runs anything; the red-check log is committed at the phase gate; probes added later are committed before they run; an amended expectation keeps the original value and the reason | AR #10, PF-005, PB-004 |
| Bounded probes | Each command runs under `timeout` with `ulimit -f 1048576`; outputs go to `build/tmp/assessment/<WS>/` | AGENTS.md §Testing |
| Container | Toolchain commands run through `make -C docker exec CMD="..."`: no nested quotes, and `$` written as `$$`; longer sequences go into a probe script under `probes/<WS>/` | AGENTS.md §Commands, PF-025 |
| Sanitizer builds | ASan: `-fsanitize=address -no-pie` with `ASAN_OPTIONS=detect_leaks=1`; UBSan: `-fsanitize=undefined -fno-sanitize-recover=all -no-pie` | AGENTS.md §Testing, PF-025 |
| vbcc | A first invocation failing with "No config file!" in a fresh container is retried once; that message is never a finding | PF-025 |
| FS-UAE | Ad-hoc probes run host-side through `docker/scripts/run.sh` (compose `emulator` service, private Xvfb), one emulator at a time; the KS1.3 ROM is decrypted into `build/tmp/assessment/` and deleted on exit, as the gates do; `build/run/out.txt` is copied to `build/tmp/assessment/<WS>/` after each run; no licensed media is copied into `plans/` | AR #6, PF-024 |
| Effort | Per phase, up to `xhigh`; `max` is never requested | AR #4, AR #19 |
| GitHub | Reading issues and labels is allowed in any phase; the label set already exists (created at planning on the user's instruction); creating issues or comments happens only in Phase 10, after the user approves the full list (03-04 §GitHub issues); commits are pushed only when the user asks | AR #24, AR #27 |

### Independence

An auditor first forms a view from the code and `spec/`. Only then does it read earlier plans,
their findings files, the DEF ledger, and `docs/known-limitations.md`, for three purposes: to
detect a regression of a closed defect, to classify recurrence, and to avoid filing a
documented limitation as new. The toolchain quirks in §Ground rules are known up front and are
not subject to this delay. A "done" claim in an earlier plan is not evidence. A conclusion an
earlier plan measured is reopened only with a new measurement that contradicts it.

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
| Reproduction | Committed probe path, exact command, expected outcome with its source, observed outcome |
| Measurement | Command, profile, flags, CPU, baseline SHA, raw output path, both numbers of every comparison |
| Code reading | `file:line` range plus the reasoning chain; allowed only for `probable` or `hypothesis` confidence |

### Expectation sources

An expected outcome cites one of: a `spec/` chapter and section; a Phase 0 C reference or a
benchmark golden; the C99 standard clause; an NDK autodoc or include prototype; a documented
harness invariant. Anything else is a spec question (PF-018).

### Confidence

`verified` is only this confidence label; it never means "passed Phase 9" (PF-002).

| Label | Meaning |
|-------|---------|
| `verified` | Reproduced from committed probe sources at the baseline (drift check passing), or measured |
| `probable` | Strong code-reading evidence, no reproduction yet |
| `hypothesis` | Plausible from reading; never ranked above Minor (AR #15) |

A finding is **surviving** when its Phase 9 verdict is `confirmed`, `downgraded`, or
`upgraded`. Only surviving findings reach the report and the DEF ledger (AR #13 as clarified
by PF-002).

### Falsifiability

A probe that supports a finding or a `clean` outcome must be shown to discriminate: either a
**contrasting input** that must behave differently does, or a **seeded defect** (a deliberate
violation of the property under test) is detected. A mutated expectation only proves a
comparator works and is accepted for comparator scripts alone. When a detector cannot be made
to fire (for example `vamos` not raising a 68000 address error), the checklist item is a
`limit`, not `clean`. The falsifiability log has one line per probe naming the form used
(PF-010).

### Checklist outcomes

Every checklist ID ends in exactly one outcome:

| Outcome | Required evidence |
|---------|-------------------|
| Finding IDs | The findings themselves |
| `clean` | Falsifiable probe IDs and/or the `file:line` site list that was examined |
| `limit` | The concrete reason (tool, access, time), and what would remove it |

Each workstream file also keeps a **coverage table** — one row per source file or function
group with its status — updated as each group finishes. It is the resume point inside a task
and is checked at the WS5 and WS6 gates (PF-015).

## Severity

| Severity | Definition | Ref |
|----------|------------|-----|
| 🔴 Critical | An accepted program miscompiles, is memory-unsafe, or emits invalid C; a crash or hang on a real Kickstart; data loss | AR #15 |
| 🟠 Major | Latent undefined behavior in emitted C; a shim leak or missing failure path; a wrongly rejected valid program; ≥10% net m68k instructions or `.text` against the equivalent C reference on a shipped 68000 build (§Measurement rules) | AR #15, PF-009 |
| 🟡 Minor | Edge-case divergence with low impact; maintainability debt that has already caused a recorded DEF | AR #15, PF-020 |
| 🟢 Opportunity | A measured improvement with no defect behind it below the Major threshold, or any gap measured only on a 68020 probe build | AR #15, PF-009 |

A `hypothesis` is never above Minor. A `clc` crash or hang on any input is Critical; a compile
time or peak RSS above 10× the Phase 0 `bench-compile` value for the same input size is Major
(PF-020). For emitted C (WS3-11), a C-compiler crash on valid emitted C, or a C compile time
above 10× the Phase 0 C compile time of the nearest-size emitted program, is Major (PB-005).

## Probe layout

```
plans/independent-assessment/probes/
├── WS0/ref/<benchmark>.c          # equivalent C references (AR #5, §Measurement rules)
├── WS0/ref/calibration.c          # C calibration program
├── WS0/measure.py                 # the Phase 0 measurement script (§Measurement rules)
├── WS0/drift-check.sh             # the drift check (§Ground rules)
│                                  # both take the plan folder and the baseline SHA as
│                                  # arguments; no run path is written into them (AR #29)
├── WS<n>/<NNN>-<slug>.clo         # Clorraine probe
├── WS<n>/<NNN>-<slug>.c           # C probe or patched emitted C (diff form preferred)
├── WS<n>/<NNN>-<slug>.expected    # expected outcome and its source, committed before the run
└── WS<n>/<NNN>-<slug>.sh          # optional runner; no network, bounded, writes under build/tmp
```

Probe sources stay small and self-contained. Patched emitted C is stored as a unified diff
against the baseline emission. Outputs, binaries, and traces stay under
`build/tmp/assessment/` and are not committed (AR #11).

## Finding template

Each workstream file `findings/WS<n>-<slug>.md` has these sections: `## Scope and limits`,
`## Dispatch record` (one line per agent dispatch: role, effort, date), `## Coverage table`,
`## Checklist outcomes`, `## Findings`, `## Spec questions`, `## Falsifiability log`,
`## Escalations`. A finding is written as:

```markdown
### ASM-WS<n>-<NNN> — <short title>

- **Severity**: Critical | Major | Minor | Opportunity
- **Confidence**: verified | probable | hypothesis
- **Lens**: host | A500 | A1200 shipped | A1200 68020 probe | all
- **Location**: `path:line[-line]` at <baseline SHA>
- **Checklist item**: <03-02/03-03 item ID, or `WS0` for a baseline observation>
- **Author session**: <session identifier>
- **Evidence**: <probe path + command + expected (source) + observed, or measurement record>
- **Impact**: <what goes wrong, for which programs, how often>
- **Root cause**: <mechanism, not symptom>
- **Recommendation**: <fix direction; alternatives if more than one is viable>
- **Effort**: S | M | L | XL
- **Related**: <DEF-n, earlier plan, other ASM IDs, known-limitations entry>
- **Verdict**: <filled in Phase 9: verdict, verifier session, reason>
```

Effort scale: S ≤ ½ day, M ≤ 2 days, L ≤ 1 week, XL > 1 week. A finding whose impact differs
per lens states every lens; its severity comes from the shipped lenses only.

## Measurement rules

**Ownership.** Phase 0 produces the whole measurement matrix; WS4 and WS7 cite its records
and add only their own probes (patched C, scaling, frame timing) (PF-008).

**Lenses.**

| Lens | Profile and flags |
|------|-------------------|
| Host | `host`, shipped release and size flags |
| A500 | `hosted-amiga-13` (nix13, `-m68000`), shipped `-O2` and `-Os` release flags |
| A1200 shipped | `hosted-amiga` (nix20, `-m68000`), shipped `-O2` release flags; the same binary an A1200 runs |
| A1200 68020 probe | `hosted-amiga` emitted C rebuilt with `-m68020`, otherwise shipped `-O2` flags; always labelled "68020 probe build" |

`make bench` numbers are labelled nix20 and are never presented as A500 numbers.

**Measurement script.** `probes/WS0/measure.py` only calls the existing `tools/perf/bench.py`
functions (`emit`, `build_step`, `run_host`, `run_vamos`, `trace_vamos`, `size_sections`,
`normalize_output`) with explicit flag arrays per lens. The Host cell is an output check only;
host time comes from the WS7-01 runner (PB-003). It adds no new metric and no comparison logic beyond
subtraction, writes only under `build/tmp/assessment/`, and is a probe runner within the probe
layout, not a new harness (AR #23).

**Equivalent C references.** Each benchmark's C reference uses the same algorithm, the same
integer widths, the same data layout and allocation pattern, the same output routine, and the
checks the spec requires written by hand (bounds, null, overflow where the spec mandates
them). The cost of those checks is measured separately by the WS4-03 patched-C probes, not by
an unchecked reference (PF-009).

**Calibration.** `probes/WS0/ref/calibration.c` prints the same line as
`benchmarks/calibration` through the same libnix stdio path. Each side is netted against its
own calibration in every lens cell; raw counts are reported alongside. Per-benchmark `.text`
is compared as the amount above each side's calibration `.text`; the fixed runtime cost is
reported once, under WS4-07.

**Output comparison.** Outputs are compared after `bench.normalize_output` (PF-030).

**Other rules.**
- Instruction counts and `.text` compare like with like: same lens, same compiler, same
  calibration basis. Flag findings (WS4-08, WS7-03) compare two flag sets and state both.
- If `vamos` cannot run a 68020, the 68020 probe lens reports static section sizes only and
  the dynamic numbers become a named limit (PF-023).
- Optimizer estimates patch the emitted C in `build/tmp`; output stays identical after
  normalization, and the diff is committed as the probe (AR #17).
- Every measured quantity in a finding or the report (instructions, `.text`, time, RSS,
  footprint) cites its record in `findings/WS0-baseline.md` or the workstream file.

## Escalation

A Critical is escalated at once in two cases: its author records it at confidence `verified`
or `probable`, or a Phase 9 verdict results in a Critical (verifier escalates). The escalating
session records the entry under `## Escalations` with a timestamp from
`date '+%Y-%m-%d %H:%M'` and the exact message text, and tells the user in the session with the
ID, one-line impact, and reproduction command. When Phase 9 later refutes or downgrades an
escalated Critical, the verdict is recorded beside the entry and the user is told at that time.
No fix is made (AR #12, PF-013).

## Agent routing

The executing session authors every probe, findings file, and ledger entry. Review agents
author nothing (PF-011).

| Use | Who | Checklist brief |
|-----|-----|-----------------|
| Authoring WS1–WS8 | The executing session | the workstream checklist |
| Phase 9 verification | A fresh `general` subagent per workstream group, read-only, writing only under `build/tmp`; groups follow tasks 9.2.1–9.2.5 | WS0, WS1, WS6 (9.2.1): `.opencode/agents/semantics-reviewer.md` and `correctness-reviewer.md`; WS2 (9.2.2): `memory-safety-auditor.md`; WS3, WS4 (9.2.3): `emission-integrity-auditor.md` and the WS4 checklist; WS5 (9.2.4): `codeops/specialists/amiga-platform-reviewer.md` and `memory-safety-auditor.md`; WS7, WS8 (9.2.5): the WS7 checklist and `correctness-reviewer.md` |
| Report review (10.3.1) | `correctness-reviewer` agent — the report is a real diff | its own brief |

Every verification packet states: whole component at the baseline, pre-existing defects in
scope, finding IDs in the `ASM-` scheme, read-only, scratch only under `build/tmp`. The lead
records each verdict with the verifier's session identifier (ST-9: "issued by").

## Verification protocol (Phase 9)

For every finding, the verifier:

1. Re-runs the reproduction or measurement from the committed sources with the drift check
   passing; for every `probable` Major or Critical it attempts a reproduction.
2. Tries to refute it: checks `spec/`, `docs/known-limitations.md`, earlier plans, and
   whether the observed behavior is intended.
3. Checks the severity against §Severity and the confidence against §Confidence.
4. Checks for duplicates across workstreams and merges them under the lowest ID.
5. Records one verdict: `confirmed`, `refuted`, `downgraded <from→to>`, `upgraded <from→to>`,
   or `merged into <ID>`, with a one-line reason.
6. Escalates any verdict that results in a Critical, per §Escalation.

The verifier also re-checks at least two `clean` outcomes per workstream, preferring items on
the A500 lens and memory-safety checklist items (WS2, WS5-03, WS6-05), and records a verdict
for each. Findings raised by a verifier are
added to the workstream file and receive their verdict from a different verification session.
Refuted findings stay in `05-findings.md` with their verdict; they never reach the report's
ranked list.

## Error Handling

| Error Case | Handling Strategy | Ref |
| ---------- | ----------------- | --- |
| A probe cannot be built or run | Record a `limit` with the error; never infer the result | §Checklist outcomes |
| FS-UAE or licensed media unavailable | Record the limit; the finding stays at `probable` | AR #6 |
| A measurement is noisy across runs | Use instruction counts (deterministic); host time only with ≥5 runs and the spread | AR #14 |
| The drift check fails | Stop; ask the user whether to re-pin; never mix baselines | AR #7, PF-005 |
| A finding needs a source change to be proven | Prove it with a probe or a patched emitted-C copy under `build/tmp`; never edit the tree or patch compiler source | AR #12, AR #17 |
| A session ends mid-task | Resume from the execution plan and the workstream file's coverage table | PF-015 |
