# Release Runbook

How a Clorraine CLI release is built, accepted, and published. `X.Y.Z` is the
new version; `RELEASE-NOTES.md` is its GitHub release body. Nothing here runs
by itself: every step that publishes, pushes, or changes an outside system
needs the maintainer's explicit go-ahead at that step.

A release has four assets and one checksum file:

| Asset | Contents |
| ----- | -------- |
| `clorraine-X.Y.Z-x86_64-linux.tar.gz` | `bin/clc`, `bin/clorraine`, `bin/clorraine-lsp`, release documents |
| `clorraine-X.Y.Z-x86_64-windows.zip` | the three `.exe` files and the release documents |
| `install.sh` | Linux installer, pinned to the archive's size and SHA-256 |
| `install.ps1` | Windows installer, pinned to the zip's size and SHA-256 |
| `SHA256SUMS` | one line per asset |

The VS Code extension is not a release asset. It is published to the Visual
Studio Marketplace as `openlorraine.clorraine`, with the same version, by
`.github/workflows/publish-vscode.yml`; the installers install it from there.

## Marketplace publishing setup

Done once; recorded here so it can be checked or rebuilt.

- Publisher `openlorraine` (display name OpenLorraine) at
  <https://marketplace.visualstudio.com/manage/publishers/openlorraine>.
- Trusted Publishing policy on that publisher: owner `OpenLorraine`,
  repository `clorraine`, workflow `publish-vscode.yml`, environment
  `marketplace`. No token or secret is stored anywhere.
- GitHub environment `marketplace`: deployments limited to tags matching `v*`;
  no required reviewers (dispatching the workflow is the approval); no secrets.
- The workflow defaults to a dry run. The build job packages and audits the
  extension without any token; the publish job runs only from a release tag
  with `dry_run` unticked, re-checks the archive's SHA-256, and publishes with
  `vsce publish --oidc`. `tools/vscode/scripts/release-version.mjs` refuses a
  tag that differs from the compiler and extension versions.

## Procedure

Run gates with `TMPDIR` and `CODEOPS_TMPDIR` under `build/tmp`, and the FS-UAE
legs of `make verify` on a private Xvfb display.

1. **Freeze `main`.** Agree that nothing else is pushed to `main` until step 10.
2. **Version.** Set `X.Y.Z` in `compiler/Cargo.toml`, `Cargo.lock`,
   `tools/vscode/package.json`, and `tools/vscode/package-lock.json`; add the
   `CHANGELOG.md` and `tools/vscode/CHANGELOG.md` entries; write
   `RELEASE-NOTES.md`; move the install commands in `README.md`, `docs/`, and
   `site/` to `vX.Y.Z`, and make every page that states the current release
   (the status page, the project brief, the quick start, the roadmap, the
   known limitations, the home page and preview bar, the README, and both CLI
   guides) describe this release.
3. **Gate.** `make verify` on the release commit.
4. **Tag.** Push tag `vX.Y.Z` on the release commit.
5. **Dry run.** Dispatch `publish-vscode` from the tag with `dry_run` ticked;
   the summary must show version `X.Y.Z` and a VSIX SHA-256.
6. **Publish the extension.** Dispatch again from the tag with `dry_run`
   unticked. Record the run ID and the VSIX SHA-256 from its summary, and
   check that the Marketplace lists `openlorraine.clorraine` `X.Y.Z`.
7. **Candidate.** `make dist` and `make dist-verify`. Write the asset table
   from `build/dist/SHA256SUMS` into a working copy of `RELEASE-NOTES.md` in
   place of its `PENDING-CHECKSUMS` line, for `--notes-file` in step 10. Do not
   commit it: the tag and `main` keep the placeholder, and the published body
   has the table.
8. **Linux acceptance.** Run
   `make acceptance CANDIDATE=build/dist TOOLCHAIN_ARCHIVE=… KICKSTART=… PUBLISH_RUN=… PUBLISH_SHA256=…`.
   It writes the evidence and stops for the in-person check: install the
   candidate for yourself (`sh build/dist/install.sh --archive build/dist/clorraine-X.Y.Z-x86_64-linux.tar.gz --checksums build/dist/SHA256SUMS --vscode`),
   check the language server and Build/Run in VS Code, then rerun with
   `EDITOR_CONFIRMED=1` and run `make acceptance-check EVIDENCE=… CANDIDATE=build/dist`.
9. **Windows acceptance.** On a Windows 11 x64 host with VS Code, as a standard
   account in the signed-in desktop session, with rc24 not yet registered:
   `powershell -NoProfile -ExecutionPolicy Bypass -File tools\setup\acceptance.ps1 -Candidate DIR -Kickstart ROM -Output FILE -PublishRun ID -PublishSha256 HEX`;
   confirm the UAC prompt and the editor check; then
   `make acceptance-check EVIDENCE=… CANDIDATE=…` on the Linux side. The
   standard account may be kept between releases; afterwards uninstall rc24
   and remove `C:\ProgramData\Clorraine`, so the next run is a clean install.
10. **Publish the release.** `gh release create vX.Y.Z --prerelease --title "vX.Y.Z developer preview" --notes-file RELEASE-NOTES.md`
    with the four assets and `SHA256SUMS` (drop `--prerelease` for a stable
    release). Then fast-forward `main` to the release commit; the site deploys
    from `main`.
11. **Check.** Install from the published release with the documented Linux
    and Windows commands.
12. **Close out.** Close the fixed issues
    with a link to the release, announce the release on the `#clorraine`
    channel of the AmigaDev Discord, and move the release off pre-release or
    to Latest when that is wanted.

## When a step fails

- **Before step 6:** delete the `vX.Y.Z` tag on GitHub and locally (nothing has
  been published under it), fix on the release branch, and restart at step 3.
- **After step 6:** a Marketplace version cannot be published twice and a
  published tag is never moved. Fix on the branch, bump to the next patch
  version, and restart at step 2 with the new tag.
- **A docs defect found after step 10** (site, `README.md`, `docs/`): fix it
  forward on `main` with the maintainer's approval. The tag and the assets
  stay as they are.
- **A defect in the published release body:** `gh release edit vX.Y.Z
  --notes-file …` with the maintainer's approval.
- **A defect in the extension's README or CHANGELOG, or in an asset:** these
  ship in published bytes, so fix forward with a patch release.
- **Publish job failed at the token exchange:** nothing was published; check
  the Trusted Publishing policy fields and re-dispatch.
- **Publish job reports the version already exists:** check the listing; do
  not re-run with a different archive.
