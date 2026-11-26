# Report and Handoff: Independent Assessment of Clorraine

> **Document**: 03-04-report-and-handoff.md
> **Parent**: [Index](00-index.md)

## Overview

Owns the structure of `05-findings.md` and `10-assessment-report.md`, the DEF-row rule, the
GitHub issue hand-offs, and the remediation roadmap handed to the user (R8, R9, R11; AR #8,
AR #13, AR #21, AR #24–#27).

## `05-findings.md` — merged ledger

Built in Phase 9 from the workstream files.

| Section | Content |
|---------|---------|
| Header | Baseline SHA, date, counts per severity and verdict |
| Ledger table | One row per finding: ID, title, severity, confidence, lens, author session, verdict, verifier session, DEF ID and GitHub issue number (filled in Phase 10) |
| Clean sample | The `clean` outcomes re-checked in Phase 9 with their verdicts |
| Merges | Duplicate IDs and the surviving ID |
| Refuted | Refuted findings with the reason (never deleted) |
| Spec questions | All spec questions from every workstream, for the user's ruling |
| Escalations | Every escalation with its timestamp, message text, and later Phase 9 verdict (03-01 §Escalation) |
| Report review | The Phase 10 `correctness-reviewer` findings and their resolution (kept apart from the ledger table) |

The detailed finding text stays in the workstream file; the ledger links to it.

## `10-assessment-report.md` — report structure

1. **Executive summary** — one page: overall verdict, the top ten surviving findings, the answer
   to the optimizer question (WS4-10), the headline overhead numbers vs. C per lens.
2. **Scope, method, and limits** — baseline SHA, pinned tool versions, workstreams, effort per
   phase, every `limit` outcome, the Phase 9 clean-sample size and results, retracted
   escalations, independence status.
3. **Scorecard** — one row per area with a grade (A–E), the count per severity, and a one-line
   justification. Areas map by checklist prefix: semantic core (WS1), ownership (WS2), codegen
   correctness (WS3), code quality and optimizer (WS4), shims (WS5), stdlib (WS6), runtime
   performance (WS7-01..05), compile performance (WS7-06..07), test integrity (WS8).
   Grades: A — no Major or Critical; B — Majors only, each with effort S or M; C — any Major
   with effort L or XL, or any Major in a recurring class (WS8-07); D — exactly one Critical;
   E — two or more Criticals, or a systemic class (≥2 Criticals sharing one WS8-08 root cause).
4. **Findings by severity** — surviving findings ranked Critical → Opportunity; each with ID,
   title, lens, one-paragraph summary, effort, and a link to the workstream file.
5. **Root-cause themes** — WS8-08 classes plus any theme spanning several findings, each
   naming the surviving findings it explains.
6. **Performance** — the Phase 0 matrix, WS4 attribution, flag results, 68020 probe gains,
   compile-time scaling.
7. **Quick wins** — surviving findings with effort S and severity ≥ Minor, or Opportunities
   with ≥5% measured gain in net instructions or `.text`.
8. **Remediation roadmap** — proposed plans: slug, goal, the findings it closes, dependencies,
   effort, and suggested order (Criticals first, then root-cause themes before their
   symptoms, then performance). Each entry is written so `make-plan` can take it as input and
   links its theme parent issue (§GitHub issues).
   Subsection **Unranked recommendations** holds items with no severity: WS6-06 observations,
   the WS7-03 68020 profile-variant question, and machinery excluded from this assessment
   (00-index §Minimum-Sufficient Baseline) that the evidence suggests building.
9. **Spec questions** — copied from the ledger for the user's ruling.
10. **Appendix** — measurement records, probe index, DEF and GitHub issue mapping.

Every measured quantity in the report cites its measurement record (ST-12). No finding
appears in sections 1, 4, 5, or 7 unless it is surviving (ST-10).

## DEF rows (AR #13, AR #21)

After the report review (task 10.3.1), each surviving Critical or Major finding gets the next
free `DEF-n` identifier (after the highest in `plans/00-roadmap.md` at that time). Rows are
inserted as tracker `↳` rows directly below the PLAN-47 row, newest first, following the
existing ledger (other plans' rows are not reordered):

```markdown
| ↳ DEF-NNN | <finding title> | — | [<PLAN>](<PLAN>/00-index.md) | Backlog | ⬜ | <date> | 🔴/🟠 <severity>; ASM-WS<n>-<NNN>; confidence <label>; <one-line impact>; issue #<n> (added when the issue is created) |
```

The DEF ID is written back into the ledger row, the finding, and the report appendix. Minor
and Opportunity findings get no DEF row. Blocker DEF rows that exec-plan adds during
execution are not finding-derived and are outside this rule.

## GitHub issues (AR #24–#27)

Surviving findings become public issues on `OpenLorraine/clorraine`, each a self-contained
hand-off a coding agent can start from. Issues are created only in Phase 10, after the report
review and DEF allocation, and only after the user approves the full list (AR #27).

### Issue kinds

| Kind | Created for | Title |
|------|-------------|-------|
| Finding issue | Every surviving Critical and Major; every surviving Opportunity listed under report §7 Quick wins | `[<tag>] <imperative summary>` |
| Minor tracking issue | One per workstream with surviving Minor findings; a checklist with one line per finding naming its theme issue | `[<tag>] Minor findings from the independent assessment: <workstream>` |
| Theme parent issue | One per remediation-roadmap entry (report §8); its finding issues are GitHub sub-issues (`--parent`); its Minor items are named in the tracking issues | `[<tag>] <theme goal>` |

All findings are filed publicly, Criticals included (AR #25). Other Opportunities and the
unranked recommendations stay in the report only.

Title tags follow the repository's existing vocabulary: `[bug]` for Critical/Major defects,
`[correctness]` for semantic or ownership divergences, `[performance]` for performance and
Opportunity items, `[test]` for test-integrity items, `[language]` for spec questions that
reach an issue; shim and Amiga-library items take the tag of their nature and the existing
`amiga-sdk` label; the domain is a label.

When an open issue already covers a finding (for example #15, #17, #20, #22, #23 at planning
time), the draft proposes a link or a comment on it instead of a new issue; the approval list
shows each overlap.

**Redaction.** Public bodies use repository-relative paths only; licensed media appears only
as a placeholder such as `<licensed Kickstart 1.3 ROM>`; no home directories, ROM names or
bytes, keys, or local environment details. Drafts are grepped for `~/`, `/home/`, `.rom`, and
`rom.key` before the approval list is shown.

### Labels

The label set below was created on `OpenLorraine/clorraine` at planning time on the user's
instruction; Phase 10 creates only labels found missing and never edits existing ones.

| Label | Values | Rule |
|-------|--------|------|
| `assessment` | — | On every issue this plan creates |
| `kind:<kind>` | `finding`, `tracking`, `theme` | The issue kind |
| `severity:<level>` | `critical`, `major`, `minor`, `opportunity` | The finding's surviving severity; tracking issues take `minor`; theme issues take their highest child severity |
| `priority:<P>` | `P0`–`P3` | P0 every Critical; P1 a Major in a recurring class (WS8-07) or with effort S/M; P2 other Majors and quick-win Opportunities; P3 tracking issues; a theme takes its highest child priority |
| `domain:<area>` | `semantics` (WS1), `ownership` (WS2), `codegen` (WS3), `optimizer` (WS4), `shims` (WS5), `stdlib` (WS6), `perf` (WS7), `tests` (WS8) | From the finding's workstream; a theme takes one label per child domain; `WS0` findings take the domain of the component they concern, with `domain:perf` for flags and measurement and `domain:tests` for harnesses and gates; the WS0 tracking issue takes `domain:tests` |
| Existing | `tier:core` (A500 lens), `tier:advanced` (A1200 lenses), `bug` (Critical/Major defects), `performance` (performance findings), `enhancement` (Opportunities, with `performance` where it applies) | Added where they apply |

Rank is the finding's position in the report's ranked list and the theme's position in the
remediation roadmap; it is written in the body, not as a label.

### Finding issue body

Drafted under `plans/<PLAN>/issues/<ASM-ID>.md` before creation:

```markdown
## Summary
<one paragraph: what is wrong and who it affects>

| Field | Value |
|---|---|
| Finding | ASM-WS<n>-<NNN> · DEF-<n> (Critical/Major only) |
| Severity / Priority / Rank | <severity> / <P> / <rank> of <total> |
| Domain / Lens | <domain> / <lens> |
| Confidence / Verdict | <confidence> / <Phase 9 verdict> |
| Effort | S / M / L / XL |
| Baseline | <commit X> |
| Theme | #<theme parent issue> (the only field filled in after approval) |

## Evidence
<minimal reproduction inline: the probe source (up to ~80 lines; longer probes are trimmed to the
relevant part), the exact repository-relative command, the expected outcome with its source,
and the observed outcome; plus permalinks to the probe at the pushed commit when the plan
commits were pushed>

## Root cause
<mechanism; every affected `file:line` at the baseline>

## Recommended fix
<direction; viable alternatives with trade-offs>

## Acceptance criteria
- <spec-derived expectation, citing the spec chapter and section>
- A regression test that fails without the fix and passes with it
- <performance findings: the measured target on the stated lens>

## Verification
- AGENTS.md gates: `make check`; `make verify` legs that cover the change; the sanitizer or FS-UAE leg named here
- Review dispatch per AGENTS.md for this change surface: <reviewers>

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments;
  restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
<other ASM IDs, DEF rows, earlier plans, known-limitations entries, dependencies>
```

Tracking issues carry the same table per finding in condensed form; theme parent issues carry
the goal, the findings they close, dependencies, order, and effort from report §8.

### Cross-links

The DEF row's last column names the issue (`#<n>`); the finding, the ledger row, and the report
appendix record the issue number; the issue names the ASM and DEF IDs.

## Repeatability (AR #29)

`tools/assessment/` holds the reusable assets; this run's folder keeps its own copies as
evidence and is never rewritten.

| Asset | Content |
|-------|---------|
| `README.md` | Runbook. **Full:** copy `template/` to `plans/assessment-<YYYY-MM-DD>/`, fill the placeholders, preflight it, run `exec-plan`. **Scoped:** the same, deleting the workstream phases not wanted and their check-point rows. **Delta:** `git diff --name-only <previous report baseline>..HEAD` mapped through the README's path-to-workstream table selects the workstreams; previous surviving findings are re-verified first. Each mode lists the GitHub label set and how to file issues |
| `drift-check.sh`, `measure.py` | Copies of the run's scripts, taking the plan folder and baseline SHA as arguments |
| `ref/` | The equivalent C references and `calibration.c` |
| `template/` | `00-index.md`, `00-ambiguity-register.md` (the user's settled rulings as standing defaults, open only for run-specific items), `01`–`03-04`, `07`, and `99` with every checkbox cleared; placeholders `<BASELINE>`, `<DATE>`, `<PLAN>` replace run-specific values; the drift check excludes `tools/assessment` and the skill path; lessons from this run folded in, limited to what Phase 9 confirmed |

`.opencode/skills/clorraine-assessment/SKILL.md` (≤40 lines, force-tracked with `git add -f`
because `.opencode/skills/` is excluded locally) only triggers the runbook: it names the three
modes and points at `tools/assessment/README.md`; it carries no method content. If a CodeOps
install overwrites or removes it, restore it with `git checkout`.

## Handoff

The plan ends when the user has the report and the approved issues exist. Phase 10 closes with
the user choosing which remediation plans to create; each goes through `make-plan`, starting
from its theme parent issue. This plan creates none of them.

## Error Handling

| Error Case | Handling Strategy | Ref |
| ---------- | ----------------- | --- |
| A finding has no Phase 9 verdict at synthesis | It stays out of the report; synthesis is blocked until Phase 9 covers it | AR #16 |
| The roadmap gained DEF rows since planning | Use the next free identifier at allocation time | AR #13 |
| A remediation proposal would change the spec | List it under spec questions; no plan is proposed until the user rules | AGENTS.md (spec is immutable) |
| The report review changes the surviving set | DEF allocation happens after the review, so no row needs rework | PF-027 |
| `gh` is unauthenticated or GitHub is unreachable | Stop before creating anything; record a blocker and ask the user; drafts stay in `issues/` | AR #27 |
| Issue creation fails partway | Record every created number at once; resume with the remaining drafts; never create a duplicate (search by title and the `assessment` label first) | AR #24 |
| The user rejects or edits part of the list | Create only the approved issues; record the rest as not filed with the reason | AR #27 |
