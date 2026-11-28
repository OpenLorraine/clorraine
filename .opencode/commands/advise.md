---
description: "Run the planning-time advisors: graphics-games-advisor and/or systems-os-advisor. Usage: /advise [graphics|os] (no argument runs both). Writes dated reports to plans/advisory/."
---

Run the Clorraine planning-time advisors. Requested target: "$ARGUMENTS"
(empty means both; `graphics` means only `graphics-games-advisor`; `os` means
only `systems-os-advisor`; anything else: ask which one was meant and stop).

Facts gathered for this run:

- Date: !`date '+%Y-%m-%d'`
- Clorraine commit and version: !`git log -1 --format='%h %s' && git describe --tags --always 2>/dev/null`
- Working tree: !`git status --short | head -20`
- LorraineOS core repository: !`cd ../../core 2>/dev/null && pwd || echo "not found at ../../core"`
- Earlier advisory reports: !`ls plans/advisory/ 2>/dev/null`

Steps:

1. Dispatch the requested advisor subagents in parallel, read-only. Give each
   one the facts above, the repository root, and for `systems-os-advisor` the
   LorraineOS core path. Ask each to read `plans/advisory/README.md` for the
   report format and to return the report text only.
2. When a report comes back, write it unchanged to
   `plans/advisory/<date>-graphics.md` or `plans/advisory/<date>-os.md`. If
   that file exists, add `-2`, `-3`, and so on; never overwrite a report.
3. Show me, per advisor, the summary and the top five proposals (id, title,
   layer, value, effort), and the file path.
4. Stop there. Do not edit code, plans, the roadmap, or the known limitations,
   do not commit, and do not start `make-requirements`; I choose which
   proposals move forward.
