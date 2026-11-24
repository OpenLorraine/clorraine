# Publication handoff

> **Status**: Local candidate complete; no publication approved or performed.
> **Candidate**: `build/dist`
> **Evidence**: `build/acceptance/20261124T225355Z/evidence.json`
> **Approval**: a publication tag/release/upload needs explicit user approval
> bound to the hashes below and a separate release plan.

## Candidate identity

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

- No `v1.2.0` tag, release, upload, or publication exists. Existing tags stop
  at `v1.1.2`.
- `make dist` and `make acceptance` are local; the network installer path is
  assembled for a future release and must not be exercised before approval.
- After approval: tag/release, upload the archive, VSIX, installer, and
  `SHA256SUMS`, verify the public bytes and links, and run an anonymous install
  check. Any byte change requires a new candidate and fresh approval.
