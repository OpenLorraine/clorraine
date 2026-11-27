# Findings Ledger: amiga-shim-crash-safety

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MINOR | `docker/scripts/a1200-addons-smoke.sh` | fixed |
| RV-001 | 2 | MINOR | `examples/robustness/crash-safety/clip-*.clo` | fixed |
| RV-002 | 2 | MINOR | `plans/amiga-shim-crash-safety/03-04-gates-and-docs.md` | fixed |
| RV-003 | 2 | MINOR | `docker/scripts/a600-addons-smoke.sh` | fixed |
| SR-001 | 2 | MINOR | `examples/robustness/crash-safety/clip-*.clo` | fixed |
| SR-002 | 2 | MINOR | `docs/amiga-sdk/registry.tsv` | fixed |
| SR-003 | 2 | MINOR | `compiler/runtime/shim_hosted_amiga_optional_services.c` | fixed |
| RV-001 | 3 | MINOR | `examples/robustness/crash-safety/region-free.clo` | fixed |
| RV-002 | 3 | MINOR | `compiler/tests/amiga_crash_safety_spec_test.rs` | fixed |

Phase 1 review evidence (2026-11-27): correctness reviewer (generic packet; the `correctness-reviewer`
role is not dispatchable in this host) — 1 minor; `amiga-platform-reviewer` — no findings (open
questions answered: the alert success path is residual limit 6; the V36 check guards the 2.0 profile).

Phase 2 review evidence (2026-11-27): correctness reviewer (generic packet) — 3 minor;
`amiga-platform-reviewer` — 3 minor, open questions answered (expunge requester documented in
Phase 5; the clip-file question is not blocking; the empty-unit assumption stays under AR-17's
reopen trigger, since ST-6 does not assert raw read counts). RV-002/SR-002 fixed in the Phase 5
doc rows; their docs land in Phase 5.

Phase 3 review evidence (2026-11-27): correctness reviewer (generic packet) — 2 minor (ST-7 extended
with the update-time paths, expected line `region: ok same 3`; ST-3 gains the guide positive);
`amiga-platform-reviewer` — no findings, vbcc builds of the new code clean; its exit-order question
is covered by AR-8 (the accepted install of NULL during an update applies at exit too).
