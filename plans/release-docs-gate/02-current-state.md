# Current state

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

- **Release flow:** `RELEASE.md` §Procedure steps 1–12 (freeze, version, gate,
  tag, dry run, publish extension, candidate, acceptance, publish release,
  check, close out). Docs edits live in step 2; nothing checks them. Step 7
  keeps `PENDING-CHECKSUMS` in the committed `RELEASE-NOTES.md`; the
  published body comes from a working copy.
- **Site:** VitePress 1.6.4, `site/.vitepress/config.ts` (base `/clorraine/`,
  `cleanUrls`, `srcExclude` README, sitemap at
  `https://openlorraine.github.io/clorraine/sitemap.xml`, 33 pages; edit
  links on every page). `docs.yml` deploys only from `main` and only on
  `site/**`, `docs/amiga-sdk/**`, `demo/poster.png`, or the workflow file.
- **Release-facing claims today:** `site/project/brief.md:9` (`v1.3.0 ·
  Updated 27 November 2026`), `site/status.md` release state,
  `site/.vitepress/theme/Layout.vue` preview bar (every page),
  `HomePage.vue` eyebrow, terminal tag and both install commands,
  `site/guide/quick-start.md` `## Install on Linux` / `## Install on Windows`,
  sidebar items in `config.ts`, `README.md`, `docs/{linux,windows}-cli.md`,
  `RELEASE-NOTES.md`, `CHANGELOG.md`, `tools/vscode/{README,CHANGELOG}.md`.
- **Historical mentions:** about 37 older `v`-tokens across the scan set
  (v1.1.2 latest stable, "since v1.2.0", Copperline v1.1.0, changelog
  history), plus third-party `v`-tokens (`v16.2`, `v3.0`).
- **Conventions:** host-side std-only Python with `unittest`
  (`tools/setup/acceptance.py`, `tests/acceptance_spec.py`), run from
  `Makefile` `dist-check`; `make check` runs host suites and Docker suites;
  `check.yml` is host-only (Python 3.12 on ubuntu-24.04). Exit 2 is the
  usage convention. The version guard `tools/vscode/scripts/release-version.mjs`
  checks the tag only at publish time.
- **Probes observed (coverage hunt):** GitHub HTML 504 from this host, GitHub
  downloads 302 → `release-assets.githubusercontent.com`; an invalid Discord
  invite returns 200 through the web URL and 404 through the API; the
  Marketplace returns 404 to HEAD and 200 to GET; Pages `max-age=600`.
- **Tests that read docs:** `compiler/tests/docs_guide_spec_test.rs` resolves
  every relative link in `docs/README.md`; `tools/setup/tests/test_retained_inputs.py`
  scans `RELEASE.md` and `AGENTS.md` for forbidden words.
