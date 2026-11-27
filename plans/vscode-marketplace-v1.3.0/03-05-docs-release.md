# Docs and Release: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 03-05-docs-release.md
> **Parent**: [Index](00-index.md)

## Overview

Bring every current-release statement to v1.3.0, the Marketplace, and Windows; replace the release
runbook and notes; then run the release (AR-1, AR-10, AR-17, AR-19, AR-20, AR-23, AR-31, AR-34,
AR-37, AR-39, AR-40).

## Docs sweep (AR-34)

Update (current-release claims):

- **Linux one-liners and version text → v1.3.0**: `README.md`, `docs/linux-cli.md`,
  `docs/README.md`, `site/guide/quick-start.md`, `site/.vitepress/theme/components/HomePage.vue`,
  `site/.vitepress/theme/Layout.vue`, `site/status.md`, `site/project/brief.md`,
  `site/guide/roadmap.md`, `site/guide/known-limitations.md`.
- **Windows**: replace `vX.Y.Z` with `v1.3.0` and remove "next release"/"first release after
  v1.2.0" wording in `README.md`, `docs/windows-cli.md`, `docs/README.md`,
  `docs/known-limitations.md` §17, `site/guide/quick-start.md`, `site/status.md`,
  `site/guide/known-limitations.md`, `site/guide/roadmap.md`, `site/project/brief.md`,
  `site/project/contributing.md`, and the two theme components.
- **Windows install command (AR-46)**: the primary command in `README.md`, `docs/windows-cli.md`,
  `site/guide/quick-start.md`, and `RELEASE-NOTES.md` becomes
  `powershell -ExecutionPolicy ByPass -c "irm https://github.com/OpenLorraine/clorraine/releases/download/v1.3.0/install.ps1 | iex"`;
  the "verify the download first" section and the `-File` form for options and scripted installs
  stay; `docs/windows-cli.md` notes that the `powershell -c` wrapper is needed (the installer's
  final `exit` would otherwise close an open PowerShell window).
- **Extension**: replace "not in a marketplace" and "installed from the release" with the
  Marketplace install (`code --install-extension openlorraine.clorraine`, link to
  `https://marketplace.visualstudio.com/items?itemName=openlorraine.clorraine`) in
  `docs/known-limitations.md`, `docs/linux-cli.md`, `docs/windows-cli.md`, `site/status.md`,
  `site/guide/known-limitations.md`, `site/guide/roadmap.md`, `site/project/brief.md`,
  `site/project/contributing.md`; remove `codium` mentions (`docs/linux-cli.md`,
  `docs/windows-cli.md`) and state that VS Code (Microsoft build) is the supported editor (AR-14).
  Document the installer's VS Code behaviour (install by ID, update by reinstall, 300 s bound) in
  the two CLI guides' flag sections.
- **Repo docs**: `AGENTS.md` (`make dist` writes four assets; `dist-verify` wording; new workflow
  in Layout `.github/workflows/`; acceptance inputs), `plans/windows-cli/08-merge-notes.md`
  ("At release" now points at this plan; mark it superseded).
- **v1.2.0 upgraders**: the release notes and the Linux guide say: rerun the installer to move
  the extension to the Marketplace version, which then updates automatically (AR-26).

Keep (historical): `CHANGELOG.md` `[1.2.0]` and older, test fixtures, tool inputs.

Verification: after the sweep,
`grep -rnI 'vX\.Y\.Z\|next release\|not in a marketplace\|codium' README.md docs site --include=*.md --include=*.vue`
(excluding `node_modules` and `.vitepress/dist`) returns nothing, and `make site-check` passes.

## Release text (AR-36, AR-37)

- `CHANGELOG.md`: `## [1.3.0] - <release date>` with Added (Windows 11 x64 CLI and installer;
  Marketplace extension; portable starter tasks; install-guide action; version notice), Changed
  (installers install/update the extension from the Marketplace; release has four assets; no
  VSIX asset), Fixed (the four crash-safety fixes, linking #57, #59, #60, #65).
- `tools/vscode/CHANGELOG.md`: the `1.3.0` entry (03-1).
- `RELEASE-NOTES.md`: v1.3.0 developer-preview notes — highlights, install commands for Linux and
  Windows, the extension, upgrade note for v1.2.0 users, known limitations link, checksums
  section filled at release. It is the `gh release` body.
- `RELEASE.md`: rewritten as the current runbook — the procedure below in reusable form (with
  `X.Y.Z`), the gates, the approvals, the fix-forward rule, and the Trusted Publishing setup
  facts (publisher, policy fields, environment rule). The superseded v1.2.0 design is removed
  (git history keeps it).

## Version bump

`compiler/Cargo.toml`, `Cargo.lock`, `tools/vscode/package.json`, `tools/vscode/package-lock.json`
(two fields) → `1.3.0`. Tests that pass their own version fixtures are not changed.

## Release procedure (Phase 7)

Each step names who acts. "Approval" means the agent stops and asks the user.

1. **Agent**: confirm the user holds other pushes to `main` for the release window (AR-40 risk).
2. **Agent**: rebase `release-1.3.0` onto `origin/main` locally; run `make verify` with the
   Windows-capable image on the private Xvfb display (AR-20). Failures stop the release.
3. **Agent — approval**: push tag `v1.3.0` on the rebased release commit.
4. **Agent — approval**: dispatch `publish-vscode` from `v1.3.0` with `dry_run=true`; check the
   summary (version 1.3.0, SHA-256).
5. **Agent — approval**: dispatch from `v1.3.0` with `dry_run=false`; record the run ID and
   SHA-256; confirm the gallery query returns 1.3.0.
6. **Agent**: `make dist`, `make dist-verify`.
7. **Agent**: Linux acceptance (`make acceptance … PUBLISH_RUN=… PUBLISH_SHA256=…`), then the
   user confirms the language server and Build/Run in VS Code → rerun with `EDITOR_CONFIRMED=1`;
   `make acceptance-check`.
8. **Agent — approval**: create the standard Windows test account over `ssh winbox` (AR-19);
   push the candidate; run `acceptance.ps1` in the desktop session; the user confirms the UAC
   prompt and the editor; `make acceptance-check`; remove the account and the pushed files.
9. **Agent — approval**: `gh release create v1.3.0 --prerelease --title "v1.3.0 developer preview"
   --notes-file RELEASE-NOTES.md` with the four assets and `SHA256SUMS`.
10. **Agent — approval**: push the release commit to `main` (fast-forward); delete
    `origin/release-1.3.0`; confirm the site deploy shows v1.3.0.
11. **Agent**: one fresh install from the published release on Linux; on Windows, the one-liner
    from `docs/windows-cli.md` run in the desktop session.
12. **Agent — approval**: close #38 (note: Marketplace listing, decisions, release link), #57,
    #59, #60, #65 (note: fixed in v1.3.0); open "Publish to Open VSX for VSCodium and other
    editors" (`enhancement`, `deferred`) (AR-39).
13. **Agent**: draft the AmigaDev Discord announcement for the user to post (only after steps 9–10,
    AR-31); update the roadmap row T-05 and this plan.

**Fix forward (AR-31):** if any step after 5 fails in a way that needs a code change, never move
the tag: fix on the branch, bump to 1.3.1, and restart at step 2.
