# Execution Plan: release docs gate

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-11-27 20:08
> **Progress**: 5/23 tasks (22%)
> **CodeOps Artifact Schema**: 1

## Overview

Build the manifest-driven docs checker (static, then live), wire it into the
release flow, and prove it on v1.3.0. Work happens on `main` (AR-37) with
`--auto-design` and `--auto-commit`; the maintainer confirms the v1.3.0
review record (AR-10).

**Verify (task)**: `make release-docs-test`
**Verify (gate)**: `make check` (+ `make site-check` when `site/` changes) (AR-36)

## Phase 1: Manifest, rules and static check

> **Reasoning**: high — new checker with matching semantics that tests must pin exactly
> **Reference**: 03-01 · ST-1..ST-23 · AR-1..AR-5, AR-12..AR-14, AR-18..AR-20, AR-25..AR-28, AR-32

### Session 1.1: Specification tests
- [x] 1.1.1 Write the fixture builder and ST-1..ST-23 in `tools/release/tests/docs_check_spec.py` ✅ (completed: 2026-11-27 20:05; fixture builder + ST-1..ST-23)
- [x] 1.1.2 Red phase: run the suite; every case fails because the checker does not exist ✅ (completed: 2026-11-27 20:06; suite fails to import: no docs_check module)

### Session 1.2: Implementation
- [x] 1.2.1 `tools/release/docs_rules.py`: manifest loader, `normalize()`, tokens, allowlist matching, dates, changelog sections, review-record parsing ✅ (completed: 2026-11-27 20:08)
- [x] 1.2.2 `tools/release/docs_check.py`: CLI, static rules, reporting, exit codes ✅ (completed: 2026-11-27 20:08)
- [x] 1.2.3 Green phase: ST-1..ST-23 pass ✅ (completed: 2026-11-27 20:08; ST-1..ST-23 pass; outputs spot-checked)

### Session 1.3: Implementation tests and hardening
- [ ] 1.3.1 `tools/release/tests/docs_check_impl.py`: normalization, offsets, loader errors, month table, CLI usage
- [ ] 1.3.2 Phase gate and review (correctness reviewer)

**Verify (gate)**: `make release-docs-test`

## Phase 2: Live check

> **Reasoning**: xhigh — network probes, a credentialed `gh`, and redirect policy
> **Reference**: 03-02 · ST-23..ST-30 · AR-6, AR-13, AR-16, AR-17, AR-21..AR-24, AR-38

### Session 2.1: Specification tests
- [ ] 2.1.1 Add ST-24..ST-30 with an injected fetcher and `gh` runner and local HTML/sitemap/API fixtures
- [ ] 2.1.2 Red phase

### Session 2.2: Implementation
- [ ] 2.2.1 `tools/release/docs_live.py`: freshness, page extraction, link collection, probes, published release, network policy
- [ ] 2.2.2 Green phase: ST-1..ST-30 pass

### Session 2.3: Implementation tests and hardening
- [ ] 2.3.1 Impl tests: HTML extraction, `Retry-After`, cross-page dedup, redirect host checks
- [ ] 2.3.2 Phase gate and review (correctness reviewer + security auditor)

**Verify (gate)**: `make release-docs-test`

## Phase 3: Release flow

> **Reasoning**: medium — runbook, targets and template text
> **Reference**: 03-03 · AR-8..AR-10, AR-15, AR-29..AR-31, AR-33

- [ ] 3.1.1 `Makefile` targets `release-docs-check` and `release-docs-test` (in `check`); a `check.yml` step
- [ ] 3.1.2 `docs/release-reviews/TEMPLATE.md`; `docs/README.md` links the folder; an impl test parses the real template
- [ ] 3.1.3 `RELEASE.md` steps 2, 3, 11, 12 and the fix-forward rows; `AGENTS.md` rule and Commands rows
- [ ] 3.1.4 Phase gate and review (correctness reviewer)

**Verify (gate)**: `make check`

## Phase 4: Proof on v1.3.0

> **Reasoning**: high — real findings must be fixed or allowlisted truthfully
> **Reference**: 07 §Proof · AR-7, AR-11, AR-34, AR-35

- [ ] 4.1.1 The real `tools/release/docs-manifest.json`: scan set, excluded files, platforms, install surfaces, required claims, banned wording, dated lines, changelogs, theme files
- [ ] 4.1.2 `make release-docs-check VERSION=1.3.0`; fix each real docs defect or add an allowlist entry with a true reason
- [ ] 4.1.3 **approval** Draft `docs/release-reviews/v1.3.0.md`; the maintainer walks the Before-tag items and confirms; tick them and fill `Reviewer`
- [ ] 4.1.4 Static pass; push (site fixes deploy); wait for the docs deploy
- [ ] 4.1.5 **approval** `make release-docs-check VERSION=1.3.0 LIVE=1`; the maintainer confirms the After-deploy items; tick them; live pass
- [ ] 4.1.6 Roadmap row T-06 Done; close the plan

**Verify (gate)**: `make check && make site-check && make release-docs-check VERSION=1.3.0 LIVE=1`
