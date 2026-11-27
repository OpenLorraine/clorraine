# Requirements: release docs gate

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Goal

"A non negotiable check and flow … do a deep check and update the docs website
for each release, like the project status, project brief, and such." In
v1.3.0 the maintainer found stale 1.2.0 wording, a missing Windows install
command on the home page, and missing sidebar install items only after the
release went live.

## Functional requirements

| ID | Requirement | AR |
|----|-------------|----|
| FR-1 | `make release-docs-check VERSION=X.Y.Z` checks the release-facing files statically and exits non-zero on any finding | AR-1, AR-19, AR-27 |
| FR-2 | Every Clorraine-style `vX.Y.Z`/`vX.Y` token, release URL and asset name is the release version or allowlisted (file + phrase + reason + `reviewed_for`) | AR-2, AR-3, AR-12, AR-14 |
| FR-3 | Required claims per page name the release; dated lines are 0–14 days old | AR-4, AR-25, AR-35 |
| FR-4 | Each supported platform's install command at vX.Y.Z appears on every install surface; the sidebar and quick start carry an install entry per platform | AR-11, AR-34 |
| FR-5 | Banned wording fails; the committed release notes carry exactly one `PENDING-CHECKSUMS` line | AR-13 |
| FR-6 | Old changelog sections are exempt from the version rule by structure | AR-18 |
| FR-7 | VERSION is validated against Cargo, the extension manifest, both changelogs and the notes title | AR-26 |
| FR-8 | A review record with the template's exact items, `Before tag` all ticked, is required statically; `After deploy` all ticked is required live | AR-5, AR-9, AR-32, AR-33 |
| FR-9 | `LIVE=1` checks every sitemap page's rendered text, the published release body, and every project link with meaningful probes | AR-6, AR-13, AR-16, AR-21..AR-24 |
| FR-10 | Unreachable and not-yet-deployed results are distinct, blocking exit codes | AR-17, AR-24, AR-27 |
| FR-11 | `RELEASE.md` and `AGENTS.md` state what each check blocks, who signs, and the fix-forward rules | AR-8, AR-10, AR-15, AR-31 |
| FR-12 | Hermetic unit tests run in `make check` and CI | AR-29 |
| FR-13 | Proof: static and live runs against v1.3.0 pass, with a confirmed v1.3.0 record | AR-7 |

## Out of scope

Wiring the gate into `make dist`, `make verify` or `publish-vscode.yml`
(AR-31); probing third-party links beyond the project links (AR-6); checking
historical notes (`docs/findings-*`, ergonomics, `spikes/`, `plans/`) (AR-1);
machine-verifying who ticked the record (AR-10).
