# Report and Handoff: Independent Assessment of Clorraine

> **Document**: 03-04-report-and-handoff.md
> **Parent**: [Index](00-index.md)

## Overview

Owns the structure of `05-findings.md` and `10-assessment-report.md`, the DEF-row rule, and
the remediation roadmap handed to the user (R8, R9; AR #8, AR #13).

## `05-findings.md` — merged ledger

Built in Phase 9 from the workstream files.

| Section | Content |
|---------|---------|
| Header | Baseline SHA, date, counts per severity and verdict |
| Ledger table | One row per finding: ID, title, severity, confidence, lens, verdict, verifier session, DEF ID (filled at synthesis) |
| Merges | Duplicate IDs and the surviving ID |
| Refuted | Refuted findings with the reason (never deleted) |
| Spec questions | All spec questions from every workstream, for the user's ruling |
| Escalations | Every critical escalation with its timestamp (AR #12) |

The detailed finding text stays in the workstream file; the ledger links to it.

## `10-assessment-report.md` — report structure

1. **Executive summary** — one page: overall verdict, the top ten findings, the answer to the
   optimizer question (WS4-10), the headline overhead numbers vs. C on both lenses.
2. **Scope, method, and limits** — baseline SHA, workstreams, effort per phase, every residual
   limit from the workstream files, independence status.
3. **Scorecard** — one row per area (semantic core, ownership, codegen correctness, code
   quality, shims, stdlib, runtime performance, compile performance, test integrity) with a
   grade (A–E), the count per severity, and a one-line justification.
   Grades: A no Major or Critical; B Majors only, each S/M effort; C Majors with an L/XL fix or
   a recurring class; D at least one Critical; E multiple Criticals or a systemic Critical class.
4. **Findings by severity** — confirmed findings ranked Critical → Opportunity; each with ID,
   title, lens, one-paragraph summary, effort, and a link to the workstream file.
5. **Root-cause themes** — WS8-08 classes plus any theme spanning several findings, each
   naming the findings it explains.
6. **Performance** — the WS7-01 matrix, WS4 attribution, flag results, A1200 probe gains,
   compile-time scaling.
7. **Quick wins** — confirmed findings with effort S and severity ≥ Minor, or Opportunity with
   ≥5% measured gain.
8. **Remediation roadmap** — proposed plans: slug, goal, the findings it closes, dependencies,
   effort, and suggested order (Criticals first, then root-cause themes before their
   symptoms, then performance). Each entry is written so `make-plan` can take it as input.
9. **Spec questions** — copied from the ledger for the user's ruling.
10. **Appendix** — measurement records, probe index, DEF mapping.

Every number in the report cites its measurement record (ST-12). No finding appears in the
report without a `confirmed`, `downgraded`, or `upgraded` verdict (ST-9).

## DEF rows (AR #13)

At synthesis, each verified Critical or Major finding gets the next free `DEF-n` identifier
(after the highest in `plans/00-roadmap.md`, `DEF-581` at planning time). Row form, following
the existing ledger:

```markdown
| ↳ DEF-582 | <finding title> | — | [independent-assessment](independent-assessment/00-index.md) | Backlog | ⬜ | <date> | 🔴/🟠 <severity>; ASM-WS<n>-<NNN>; <one-line impact> |
```

The DEF ID is written back into the ledger row and the finding. Minor and Opportunity
findings get no DEF row.

## Handoff

The plan ends when the user has the report. Phase 10 closes with the user choosing which
remediation plans to create; each goes through `make-plan`. This plan creates none of them.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A finding has no verification verdict at synthesis | It stays out of the report; synthesis is blocked until Phase 9 covers it | AR #16 |
| The roadmap gained DEF rows since planning | Use the next free identifier at synthesis time | AR #13 |
| A remediation proposal would change the spec | List it under spec questions; no plan is proposed until the user rules | AR #1 |
