---
name: clorraine-assessment
description: Run the independent assessment of the Clorraine compiler, emitted C, runtime shims, standard library, and performance again — full, scoped, or delta mode. Use when the user asks to repeat, re-run, or update the assessment or audit.
---

# Clorraine assessment

This skill only starts the runbook; the method lives in the repository.

1. Read `tools/assessment/README.md` and follow it.
2. Pick the mode with the user:
   - **Full** — every workstream from a fresh baseline.
   - **Scoped** — the full plan with some workstreams removed.
   - **Delta** — only the workstreams whose paths changed since the previous report's
     baseline (the README's path-to-workstream table), re-verifying the previous
     surviving findings first.
3. Create the run folder from `tools/assessment/template/`, preflight it with the
   `preflight` skill, and execute it with `exec-plan`.
4. Read `tools/assessment/template/LESSONS.md` before writing probes.

If a CodeOps install overwrites or removes this file, restore it with
`git checkout -- .opencode/skills/clorraine-assessment/SKILL.md`.
