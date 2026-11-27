# Findings Ledger: amiga-shim-crash-safety

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| RV-001 | 1 | MINOR | `docker/scripts/a1200-addons-smoke.sh` | fixed |

Phase 1 review evidence (2026-11-27): correctness reviewer (generic packet; the `correctness-reviewer`
role is not dispatchable in this host) — 1 minor; `amiga-platform-reviewer` — no findings (open
questions answered: the alert success path is residual limit 6; the V36 check guards the 2.0 profile).
