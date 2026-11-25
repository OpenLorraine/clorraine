# Report and Handoff: Independent Assessment of Clorraine

> **Document**: 03-04-report-and-handoff.md
> **Parent**: [Index](00-index.md)

## Overview

Owns the structure of `05-findings.md` and `10-assessment-report.md`, the DEF-row rule, and
the remediation roadmap handed to the user (R8, R9; AR #8, AR #13, AR #21).

## `05-findings.md` — merged ledger

Built in Phase 9 from the workstream files.

| Section | Content |
|---------|---------|
| Header | Baseline SHA, date, counts per severity and verdict |
| Ledger table | One row per finding: ID, title, severity, confidence, lens, author session, verdict, verifier session, DEF ID (filled at synthesis) |
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
   symptoms, then performance). Each entry is written so `make-plan` can take it as input.
   Subsection **Unranked recommendations** holds items with no severity: WS6-06 observations,
   the WS7-03 68020 profile-variant question, and machinery excluded from this assessment
   (00-index §Minimum-Sufficient Baseline) that the evidence suggests building.
9. **Spec questions** — copied from the ledger for the user's ruling.
10. **Appendix** — measurement records, probe index, DEF mapping.

Every measured quantity in the report cites its measurement record (ST-12). No finding
appears in sections 1, 4, 5, or 7 unless it is surviving (ST-10).

## DEF rows (AR #13, AR #21)

After the report review (task 10.3.1), each surviving Critical or Major finding gets the next
free `DEF-n` identifier (after the highest in `plans/00-roadmap.md` at that time). Rows are
inserted as tracker `↳` rows directly below the PLAN-47 row, newest first, following the
existing ledger (other plans' rows are not reordered):

```markdown
| ↳ DEF-NNN | <finding title> | — | [independent-assessment](independent-assessment/00-index.md) | Backlog | ⬜ | <date> | 🔴/🟠 <severity>; ASM-WS<n>-<NNN>; confidence <label>; <one-line impact> |
```

The DEF ID is written back into the ledger row, the finding, and the report appendix. Minor
and Opportunity findings get no DEF row. Blocker DEF rows that exec-plan adds during
execution are not finding-derived and are outside this rule.

## Handoff

The plan ends when the user has the report. Phase 10 closes with the user choosing which
remediation plans to create; each goes through `make-plan`. This plan creates none of them.

## Error Handling

| Error Case | Handling Strategy | Ref |
| ---------- | ----------------- | --- |
| A finding has no Phase 9 verdict at synthesis | It stays out of the report; synthesis is blocked until Phase 9 covers it | AR #16 |
| The roadmap gained DEF rows since planning | Use the next free identifier at allocation time | AR #13 |
| A remediation proposal would change the spec | List it under spec questions; no plan is proposed until the user rules | AGENTS.md (spec is immutable) |
| The report review changes the surviving set | DEF allocation happens after the review, so no row needs rework | PF-027 |
