# Requirements: VS Code Marketplace Extension and the v1.3.0 Release

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)
> **Source**: GitHub issue #38, revised by the Ambiguity Register (this document owns the requirements)

## Feature Overview

Publish the Clorraine VS Code extension to the Microsoft VS Code Marketplace, make both installers
install and update it from there, make the starter project portable, and release v1.3.0 with the
extension, the Windows installer, and the Amiga crash-safety fixes.

## Functional Requirements

### Must Have

- [ ] **R1 Listing-ready extension** — PNG icon, user-facing listing README, extension-only
  changelog, listing metadata, `"preview": true`, `extensionKind: ["workspace"]`, limited
  untrusted-workspace support; one universal VSIX whose version is the compiler version
  (AR-3, AR-4, AR-18, AR-36, AR-38).
- [ ] **R2 Publish workflow** — `.github/workflows/publish-vscode.yml` builds the VSIX from the
  dispatched ref and publishes it with Trusted Publishing; dry runs never publish; the publish job
  runs only from a `v*` tag whose name equals the compiler and manifest versions (AR-5, AR-11–13,
  AR-29, AR-30, AR-41, AR-42).
- [ ] **R3 Driver command** — the extension contributes `clorraine.driverPath`, hidden from the
  Command Palette, returning the resolved driver; with no driver it shows the install-guide error
  and cancels (AR-8, AR-32).
- [ ] **R4 Install guide and drift notice** — the three missing-tool errors offer "Open install
  guide"; a non-blocking notice appears once per activation when the language server is older than
  the extension (AR-9, AR-16, AR-35).
- [ ] **R5 Portable starter** — `clorraine new` writes tasks that use
  `${command:clorraine.driverPath}`; `launcher_for` is removed (AR-8, AR-25).
- [ ] **R6 Installers use the Marketplace** — both installers detect `code` and `code-insiders`
  only; install by ID with today's consent when absent; uninstall then install by ID when present
  unless opted out; bound the editor call at 300 s; warn and continue on failure; no VSIX pins
  (AR-7, AR-14, AR-15, AR-26, AR-27, AR-33, AR-43).
- [ ] **R7 Four-asset candidate** — `make dist` writes the Linux archive, the Windows zip,
  `install.sh`, `install.ps1`, and `SHA256SUMS`; no VSIX (AR-6, AR-34).
- [ ] **R8 Acceptance proves the Marketplace extension** — Linux and Windows acceptance install
  the extension with the installer and record its ID, version, publish run, and VSIX SHA-256 plus a
  manual editor confirmation; `acceptance-check` requires the candidate version (AR-28, AR-44).
- [ ] **R9 Docs and runbook** — every current-release claim reflects v1.3.0, the Marketplace, and
  Windows; `RELEASE.md` is the current runbook; `RELEASE-NOTES.md` holds the v1.3.0 notes
  (AR-34, AR-37).
- [ ] **R10 Release v1.3.0** — version bump, `make verify`, tag, publish, `make dist`, both
  acceptance runs, GitHub pre-release, fast-forward `main`, issue closures and a follow-up issue,
  announcement draft (AR-1, AR-10, AR-17, AR-19, AR-20, AR-23, AR-31, AR-39, AR-40).

### Should Have

- None.

### Won't Have (Out of Scope)

- Everything in AR-21: Open VSX, VSCodium, Cursor and other editors; platform-targeted VSIXs;
  bundling `clorraine-lsp`; macOS; a verified-publisher domain; the Marketplace pre-release
  channel; the Windows merge-note follow-ups; the open critical compiler issues.
- An offline-install section and multi-editor installs (optional coverage items, strict scope).

## Technical Requirements

### Performance

- The installer's editor call is bounded at 300 s (AR-33).

### Compatibility

- VS Code `^1.85.0` stays the engine floor; implicit command activation relies on it (AR-32).
- Existing starter projects keep working unchanged: their absolute launcher paths follow upgrades.
- The extension works with an older `clorraine-lsp` through the existing capability checks; the
  drift notice only informs (AR-16).

### Security

- No stored publishing credential; only the publish job can request an OIDC token; no npm cache;
  `npm ci --ignore-scripts` in the publish job; actions pinned by SHA; refs passed through `env:`
  (AR-5, AR-12, AR-30, AR-29).
- The publish job re-verifies the built VSIX's SHA-256 before publishing (AR-30).
- Trust for the extension moves from the release's pinned hash to the Marketplace; acceptance
  records the published VSIX SHA-256 and run ID as provenance (AR-6, AR-28).

## Scope Decisions

The decisions, options, and rationale are owned by the register; see AR-1..AR-44.

> **Traceability:** Every scope decision references the Ambiguity Register entry that resolved it.

## Acceptance Criteria

1. [ ] Every ST-case in `07-testing-strategy.md` passes.
2. [ ] `make check`, `make vscode-check`, `make dist-check`, and `make site-check` pass; the
   PowerShell installer suites pass on the Windows host.
3. [ ] A dry run of `publish-vscode.yml` from `main` builds, audits, and stops before publishing.
4. [ ] `make verify` passes on the release commit (AR-20).
5. [ ] `openlorraine.clorraine` 1.3.0 is listed on the Marketplace, published by the workflow from
   tag `v1.3.0`.
6. [ ] Linux and Windows acceptance records for the exact candidate pass `make acceptance-check`,
   including the extension entry and the manual editor confirmation.
7. [ ] GitHub pre-release `v1.3.0` carries exactly the four assets and `SHA256SUMS`; `main` points
   at the release commit and the site shows v1.3.0.
8. [ ] Issues #38, #57, #59, #60, #65 are closed with notes; the Open VSX follow-up issue exists;
   an announcement draft is handed to the user.
