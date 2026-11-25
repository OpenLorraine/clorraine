# Publication handoff

> **Status (2026-11-25)**: public prerelease `v1.2.0` published; `v1.1.2`
> remains Latest. No stable v1.2.0 exists.
> **Candidate**: `build/dist` (rebuilt first-run candidate)
> **Evidence**: `build/acceptance/20261125T112110Z/evidence.json`
> **Release**: `https://github.com/OpenLorraine/clorraine/releases/tag/v1.2.0`

## Revision 3 candidate identity (published)

| Asset | Bytes | SHA-256 |
|---|---:|---|
| `clorraine-1.2.0-x86_64-linux.tar.gz` | 6,530,816 | `547d1bdd42e8406f4be2a096055a7b6b638283ebb279c4c8901fbec063b7fdbb` |
| `clorraine-1.2.0.vsix` | 26,435 | `3918ca3fc54b9cfd30022e84e70f3a1207a31b58656b9cf6cf8afeaddd2fe180` |
| `install.sh` | 42,958 | `2239e2442dcc7b110cb2c69837d8f0dc1837e3a9efc380b0519ceb7d60b43e05` |

The archive contains only `bin/{clc,clorraine,clorraine-lsp}` and the five
release documents. The installer verifies and installs the pinned rc24
toolchain from AmigaPorts, offers the pinned VSIX, runs the host hello, and runs
the guest hello when FS-UAE and a licensed ROM are available. It never mirrors
or bundles the toolchain, ROM, or emulator.

## Revision 1 candidate identity (historical, not published)

| Asset | Bytes | SHA-256 |
|---|---:|---|
| `clorraine-1.2.0-x86_64-linux.tar.gz` | 6,530,342 | `7c43a3303e74347c8036e8b2108cd0b8d39478a8510f3108a1feb2d990ca80f0` |
| `install.sh` | 20,193 | `f9924eb5a9f8dd3c524cce187ff6c4972b163bbcdfa9b145ab9df256cd9d2478` |
| `clorraine-1.2.0.vsix` | 26,435 | `9385b8701924125f011a280808108a12de8801ddffefe285c457565a69b36904` |

The archive contains only `bin/{clc,clorraine,clorraine-lsp}` and the five
release documents. It contains no toolchain, Kickstart ROM, FS-UAE binary,
Python helper, or licensed media.

## Evidence

- `make acceptance` installed the exact candidate into a fresh HOME, verified
  the pinned rc24 archive before extraction, and ran setup, hello, the four
  hosted-amiga corpus programs, the hosted-amiga-13 file-copier, and the rc-1
  negative on FS-UAE 3.2.35 with the licensed 524,299-byte KS3.1 A1200 ROM
  (`541f7467…1bb0`).
- `make acceptance-check` reports `ok` on the recorded evidence.
- The evidence directory is self-contained: archive, VSIX, installer,
  `SHA256SUMS`, `commands.txt`, raw outputs/logs, and `evidence.json`.
- `make check`, `make dist-check` (37 tests), `make dist-verify`,
  `make vscode-check`, `make site-check`, and `make verify` pass. `make verify`
  printed `verify: all legs passed` on a private Xvfb; the desktop Xwayland
  authentication failure at the existing paddle capture is recorded in the
  plan and is not a code failure.

The checker proves the record's format and the candidate/corpus bytes. It does
not authenticate the run provenance. Keep the raw acceptance directory, and
treat publication as a separate approval.

## Publication boundary

- The public `v1.2.0` prerelease exists (`isPrerelease=true`,
  `isDraft=false`); `v1.1.2` remains Latest. No stable v1.2.0 exists.
- Docs and website remain frozen until the user's test. The README and release
  body describe VS Code, the toolchain, the persistent hello examples, and the
  FS-UAE guest hello.
- If the user's test fails, do not delete or rewrite automatically. Choose to
  delete the prerelease and tag (a public history change) or publish a fixed
  prerelease under a new tag.
- Any byte change requires a new candidate, fresh acceptance evidence, and a
  recreated prerelease.
