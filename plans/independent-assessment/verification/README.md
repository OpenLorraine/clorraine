# Phase 9 verification packets

One packet per workstream group (03-01 §Agent routing): `packet-9.2.1.md` (WS0, WS1, WS6),
`packet-9.2.2.md` (WS2), `packet-9.2.3.md` (WS3, WS4), `packet-9.2.4.md` (WS5),
`packet-9.2.5.md` (WS7, WS8). Each holds the finding texts (the template fields, without
the author's self-refutation notes), the clean outcomes to sample, the checklist briefs,
the verification protocol, severity and confidence rules, and the scope statement.

## Red check (task 9.1.2, 2026-11-26)

Every probe path and command path in the five packets was resolved against `git ls-files`
at HEAD with the drift check passing: all resolve. Two references needed reading: the WS1
range `probes/WS1/058`–`064` (seven committed probes and their data files) and the WS3
probe folder `probes/WS3/342-generic-key-collision/` (committed files). No finding went
back to its author for a missing file.
