# Release docs gate

> **Document**: 00-index.md
> **Status**: Planning Complete
> **Implements**: T-06 (user request, 2026-11-27; no RD)
> **Created**: 2026-11-27 19:13
> **CodeOps Artifact Schema**: 1

## Overview

Every release gets a non-negotiable docs gate: a manifest-driven checker,
`make release-docs-check VERSION=X.Y.Z`, that runs statically on the release
commit before the tag and with `LIVE=1` after the site deploys, plus a
two-section human review record that the maintainer confirms. The static check
blocks the tag; the live check blocks close-out (announcement, issue closing,
any move off pre-release or to Latest) (AR-8). The plan ends with a real run
against v1.3.0.

## Planning Coverage

> **Coverage**: 4/4 clusters · 12/12 categories · 45 candidates (29 distinct) · 3 residual limits · Effort: high/high/high/xhigh→inherited · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-27; artifacts RELEASE.md, Makefile, site/, docs/, README.md, RELEASE-NOTES.md, CHANGELOG.md, tools/vscode/, .github/workflows/

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 | Product & scope, UX | RELEASE.md, site sources, v1.3.0 miss commits, changelogs | Deployed site not fetched | 9 | Platform-coverage gap is the key finding | high |
| C2 | Behavior & edges | Notes/changelog states, rendered dist, live probes | No fresh VitePress build | 13 | GitHub HTML 504s may be host-specific | high |
| C3 | Technical & data | Makefile/CI wiring, Python conventions, sitemap + 83 links | Docker Python not checked | 12 | Text-extraction and matching semantics | high |
| C4 | Risk & stakeholders | Ordering, fix-forward, attestation, network policy | Marketplace unpublished-version behaviour | 11 | Two design-level conflicts (AR-8, AR-9) | high |

**Source note:** AR-8..AR-18 merge the CV ids listed per row; AR-19..AR-38 are delegated technical details.

## Documents

| Document | Purpose |
|----------|---------|
| [00-ambiguity-register.md](00-ambiguity-register.md) | Zero-Ambiguity Gate: 38 decisions |
| [01-requirements.md](01-requirements.md) | Requirements and scope |
| [02-current-state.md](02-current-state.md) | What exists today |
| [03-01-static-check.md](03-01-static-check.md) | Manifest, rules, static mode, review-record validation |
| [03-02-live-check.md](03-02-live-check.md) | Live mode: sitemap, rendered text, probes, network policy |
| [03-03-flow.md](03-03-flow.md) | Make targets, CI, template, RELEASE.md and AGENTS.md |
| [07-testing-strategy.md](07-testing-strategy.md) | Specification tests ST-1..ST-30 |
| [99-execution-plan.md](99-execution-plan.md) | Phases and tasks |

## Specialist Agents

None. The work is a std-only Python checker plus docs; the catalog
correctness and security reviewers cover it (network probes and `gh` use).
No framework, DSL or protocol a generic executor would rediscover.
