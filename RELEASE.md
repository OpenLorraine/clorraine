# Release Runbook: Clorraine v1.2.0

This document owns the v1.2.0 candidate, its evidence and the publication gate.
It records what may be built locally, what the two manual workflows may upload
as drafts, and what the user must approve before anything becomes public.
Publication is not implied by a green build, a draft upload or a private
acceptance run.

## Release decision

| Item | Decision |
| ---- | -------- |
| Version | `1.2.0` |
| Release target | Native Linux x86_64: installer, three compiler binaries, support, editor payload |
| Status | Candidate. Drafts may be prepared; publication waits for explicit approval |
| Licence | Apache License 2.0 with the Clorraine Output Exception |
| Distribution tags | `v1.2.0`, then `v1.2.0-rN` for packaging-only or support-only revisions |
| Component namespace | `components-<identity>` |
| Support namespace | `support-<identity>` |
| Source identity | The reviewed compiler commit and a reviewed support lock |
| Distribution assets | `install.sh`, `install.pyz`, `manifest.json` |
| Publication approval | Explicit user approval bound to the reviewed manifest hash and evidence |

Support and component releases are immutable payload stores. They are created as
drafts, are never marked latest, and are excluded from both default and exact
installer selection. The installable distribution is the only release the
installer selects by tag.

`v1.2.0-rN` may reuse unchanged compiler binaries and the exact prior component
version while a changed helper, VSIX or support set advances the distribution
identity. Changed bytes always need a new identity; a retry may only verify and
reuse identical existing bytes.

## Version records

The version of record is `compiler/Cargo.toml`, mirrored in `Cargo.lock`. The
editor version is `tools/vscode/package.json`, mirrored in
`tools/vscode/package-lock.json`. All four record `1.2.0` for this release.
`tools/demo/`, `site/` and unrelated npm dependencies keep their own package
versions.

## Local candidate assembly

Run as an ordinary user from the repository root. These commands never publish,
create tags, install a candidate or fetch a missing dependency.

1. Build the three Linux x86_64 binaries on the Ubuntu 22.04-compatible
   baseline:

   ```sh
   cargo build --release --workspace
   ```

2. Archive the core and its five first-party notice files:

   ```sh
   python3 tools/distribution/package_core.py archive \
       --binaries target/release --output build/release/components/core.tar
   ```

3. Package the current editor payload and copy it to the component directory:

   ```sh
   make vscode-package
   cp build/vscode/clorraine-1.2.0.vsix build/release/components/clorraine.vsix
   ```

4. Place the nine support inputs in one directory: `support-lock.json`,
   `amiga.tar`, `amiga-audit.json`, `preparation.tar`,
   `preparation-audit.json`, `cp310.tar`, `cp310-audit.json`, `cp312.tar`,
   `cp312-audit.json`. The support workflow creates the same set from the
   locked build.

5. Assemble the candidate. `--support-inputs` is the directory from step 4:

   ```sh
   python3 tools/distribution/package_release.py assemble \
       --release v1.2.0 --components <component-id> --support <support-id> \
       --core build/release/components/core.tar \
       --vsix build/release/components/clorraine.vsix \
       --support-inputs build/release/support-inputs \
       --output build/release/distribution
   ```

   The output contains exactly `install.sh`, `install.pyz` and `manifest.json`.
   Assembly is deterministic, validates every input before publishing and
   refuses an existing output.

6. Check reusable component bytes before upload:

   ```sh
   python3 tools/distribution/package_release.py verify-component \
       --components <component-id> \
       --core build/release/components/core.tar \
       --vsix build/release/components/clorraine.vsix \
       --existing build/release/component-existing
   ```

   `--existing` is optional and holds the assets downloaded from an existing
   `components-<identity>` release. Identical bytes report `reuse`; changed
   bytes under the same identity fail and require a new identity.

7. Bind the release specification to the actual local candidate:

   ```sh
   cd tools/distribution/tests
   CLORRAINE_RELEASE_CORE=$PWD/../../../build/release/components/core.tar \
   CLORRAINE_RELEASE_VSIX=$PWD/../../../build/release/components/clorraine.vsix \
   CLORRAINE_RELEASE_CORE_SHA256=<reviewed-core-sha256> \
   CLORRAINE_RELEASE_VSIX_SHA256=<reviewed-vsix-sha256> \
   python3 -m unittest -v test_release_spec
   ```

## The two manual workflows

Both workflows are dispatch-only, validate every input as literal data, pin
their actions, keep default permissions read-only and give only the final upload
job `contents: write`.

### `support-release.yml`

Inputs are the exact source commit and a new immutable support identity. On a
self-hosted runner labelled `clorraine-release` with verified source staging and
the inspected offline image, it validates the lock, runs the locked support and
wheel producers, assembles the sdk/preparation/cp310/cp312 payloads, creates the
four audited archives, and uploads them with their audits and lock as a draft
`support-<identity>` release. The preparation asset is built from the locked
script/inputs and the SDK helpers, not copied from the SDK payload. The workflow
also uploads the same asset set as a run artifact for the distribution workflow;
the distribution build consumes that artifact, while the support upload job alone
resolves its existing draft by tag with the write token to complete a rerun. It
does not mark the release latest or publish it.

Runner variables:

| Variable | Meaning |
| --- | --- |
| `CLORRAINE_RELEASE_SOURCES` | Verified read-only source staging directory matching the lock |
| `CLORRAINE_RELEASE_IMAGE_ID` | Full immutable `sha256:` identity of the inspected offline build image |

### `distribution-release.yml`

Inputs are the exact compiler commit, distribution tag, new component identity,
exact support identity and its successful support-workflow run id, prior
component identities for unchanged core and VSIX bytes (or `none`), and the
approved manifest hash (or `none` for drafts only). It builds or reuses core and
VSIX independently, validates their metadata, downloads the support artifact
from the recorded support run, assembles the distribution, checks the same bytes
and executes all three binaries on Ubuntu 22.04 and 24.04, and creates draft
`components-<identity>` and `<release>` releases. The distribution build handoff
does not read draft support releases through the tag API; the upload job resolves
its own existing drafts by tag with the write token so it can complete a partial
upload idempotently. The support
artifact is retained for seven days, so dispatch the distribution workflow
within that window or retain the nine exact inputs separately.

A non-`none` approved hash must equal the assembled `manifest.json` before any
promotion. A separate read-only job runs the local `promotion` command over the
exact downloaded assets. The upload job then publishes support, component and
distribution in that order, but only after every manifest URL has been fetched
anonymously and compared to its declared hash. Support and component releases are
patched to `make_latest=false`; the distribution may become latest only after
approval.

### Promotion eligibility

The local check is:

```sh
python3 tools/distribution/package_release.py promotion \
    --manifest build/release/distribution/manifest.json \
    --assets build/release/approval-assets \
    --approval build/release/approval.json
```

`approval.json` has exactly `schema`, `release`, `manifest_sha256`, `approved`,
`unresolved_major` and `st56`. Promotion requires a matching manifest hash,
`approved: true`, zero unresolved major findings and every referenced local
payload present with matching size, hash and archive inventory. The upstream NDK
is referenced by its locked URL and is not repackaged here.

ST-56 is post-publication evidence. `st56: pending` keeps pre-publication
promotion eligible while `final_completion_ready` stays false. `st56: passed`
clears final completion; `st56: failed` exits nonzero.

The approved manifest hash covers the eleven referenced payload assets.
`install.sh` and `install.pyz` are not entries in that manifest, so the workflow
does not claim the approval hash binds them; it requires any existing draft entry
bytes to compare identical before reuse. Verify their exact identities through
the downloaded draft evidence.

## Candidate acceptance

Candidate-green means ST-53, ST-54 and every applicable earlier case, not a
successful build alone:

- ST-53: clean Ubuntu 22.04 and 24.04 installation from the shipped bootstrap
  with only prerequisites, no repository or original SDK access and a cleared
  `PYTHONPATH`; host and both Amiga-profile smoke, update/failure recovery and
  relocation.
- ST-54: native FS-UAE and the full legacy Amiga gates on private displays;
  `make verify`, `make vscode-check` and `make site-check` pass.

ST-55 (WSL2/Remote-WSL) is retired under AR-49. v1.2.0 is Linux x86_64 only and
makes no WSL or Windows support claim; their absence is not an acceptance gap.

The v1.2.0 runbook is not a claim that any of these have passed. Evidence is
recorded with the exact candidate identities in the release records.

### Candidate directory

Place the complete candidate in one ordinary directory. It must contain exactly:

```text
install.sh
install.pyz
manifest.json
core.tar
clorraine.vsix
support-lock.json
amiga.tar
amiga-audit.json
preparation.tar
preparation-audit.json
cp310.tar
cp310-audit.json
cp312.tar
cp312-audit.json
```

The distribution draft supplies `install.sh`, `install.pyz` and `manifest.json`;
the component release supplies `core.tar` and `clorraine.vsix`; the support release
supplies the lock, four archives and four audits. Verify their hashes against the
reviewed manifest before moving them into the directory.

### Staged acceptance command

The acceptance test is stage-aware. Pre-publication accepts pending ST-56;
post-publication requires passed ST-56.

```sh
cd tools/distribution/tests
CLORRAINE_ACCEPTANCE_CANDIDATE=/absolute/candidate-directory \
CLORRAINE_ACCEPTANCE_EVIDENCE=/absolute/evidence.json \
python3 -m unittest -v test_acceptance_spec.ActualCandidateAcceptance

# After approved publication and a fresh anonymous installation:
CLORRAINE_ACCEPTANCE_STAGE=post \
CLORRAINE_ACCEPTANCE_CANDIDATE=/absolute/candidate-directory \
CLORRAINE_ACCEPTANCE_EVIDENCE=/absolute/evidence.json \
python3 -m unittest -v test_acceptance_spec.ActualCandidateAcceptance
```

The evidence record has exactly `schema`, `release`, `candidate`, `st53`, `st54`
and `st56`. `candidate` binds `manifest_sha256`, `core_sha256`, `vsix_sha256`,
`install_sh_sha256` and `install_pyz_sha256`. `st53.ubuntu` is `["22.04",
"24.04"]` and `st53.python` is `{"22.04":"3.10","24.04":"3.12"}`. Every value
under `st53.checks` and `st54` is an explicit `true`: `cli`, `lsp`, `vsix`,
`host`, `vamos`, `exit7`, `profiles`, `update`, `failure`, `relocation`,
`hashes`; `private_display`, `make_verify`, `make_vscode_check`,
`make_site_check`, `native_fs_uae`, `legacy_gates`. `st56.state` is `pending`
before publication, and `passed` with `anonymous`, `links`, `hashes_match` and
`install` true after publication.

### ST-53: clean Ubuntu installation

Run on a fresh Ubuntu 22.04 and a fresh Ubuntu 24.04 x86_64 environment. The
repository, build tree and existing SDK must be inaccessible, `PYTHONPATH` must
be cleared, and only system prerequisites may be installed:

```sh
sudo apt install gcc libc6-dev python3 python3-venv curl ca-certificates
sh install.sh --release v1.2.0 --root "$HOME/clorraine-acceptance"
export PATH="$HOME/clorraine-acceptance/bin:$PATH"
clorraine --version
clorraine-lsp --version
```

Run a host program and both Amiga profiles through `vamos`; observe the
documented runtime-fault exit 7; perform an update, a failed-update recovery and
a whole-root relocation with the original prefix unavailable. Install the printed
VSIX command and verify the language server and editor Build/Run. Record exact
candidate/payload hashes, Ubuntu/Python versions, commands and observed output.

### ST-54: private-display native and legacy gates

Start the SDK image's Xvfb on a private display (`Xvfb :N -screen 0
1280x1024x24 -ac +extension GLX -noreset`), export it only to the gate process,
and run:

```sh
make verify
make vscode-check
make site-check
```

The native FS-UAE A1200/KS3.1 action and the existing legacy Amiga gates must run
on that private display. No gate may open on the desktop or include licensed
media in published evidence. `make verify` must print `verify: all legs passed`.

### ST-56: post-publication anonymous install

Run only after the explicit publication approval. Start from a fresh login with
no repository, credentials or prior installation. Download the published
`install.sh` and every referenced asset through the public HTTPS URLs, compare
their bytes with the approved manifest hashes, install with `--release v1.2.0`,
and run the host, both `vamos` profiles and the Linux editor checks. Record the
public release identity, hashes, commands and observed output. Missing ST-56
blocks final completion, not an earlier approval request.

## Publication sequence

1. Complete candidate evidence, the evidence/hash-mismatch controls, the full
   `make verify`, `make vscode-check`, `make site-check` and independent
   acceptance review. Resolve every critical/major finding.
2. Request explicit user approval bound to the reviewed manifest hash, component
   identity and support identity. Any byte or evidence change invalidates it.
3. Dispatch `distribution-release.yml` with the approved hash. It rechecks the
   hash, runs `promotion`, publishes support first and then the component, both
   without latest.
4. Verify every referenced public URL and payload hash. A missing, private or
   mismatched payload blocks distribution promotion.
5. Publish the distribution, then perform ST-56 with fresh anonymous downloads
   and compare the public bytes to the approved hashes. Missing ST-56 blocks
   final completion.

No workflow run in this repository has performed step 3 or later for v1.2.0.
This planning delivery creates draft definitions and local producers only.

## Failure handling

| Failure | Action |
| --- | --- |
| A gate fails | Stop. A compiler or test failure needs a user ruling; an environmental fix may be retried once with the log retained |
| A payload hash does not match | Do not promote. Rebuild or select the exact reviewed bytes; never replace an existing asset |
| A referenced support/component payload is missing or private | Do not promote the distribution |
| A finding is critical/major | Stop for the user's ruling before continuing |
| Approval is absent or its hash changed | Do not publish; obtain new approval bound to the current candidate |
| ST-56 is missing | Final completion stays open; it does not authorise replacing the distribution |
| A release identity already exists | Verify identical bytes and reuse; changed bytes need a new identity |
| A draft is partially uploaded | Retry the missing upload; never delete or overwrite a released identity without a recorded ruling |

## Historical publication: v1.1.2

v1.1.2 was published on 2026-11-11 and was **published under the MIT License**.
The repository license changed to Apache License 2.0 with the Clorraine Output
Exception after publication; the published artifacts are unchanged and remain
available under MIT.

| Item | Historical record |
| --- | --- |
| Annotated tag | `v1.1.2`, tag object `5beec93`, commit `ed439ae` |
| Current licence at publication | MIT |
| Source archive | `clorraine-1.1.2-src.tar.gz`, 5,135,518 bytes, SHA-256 `44337730938d6dfa767370d325380aeae337fb350c4f6e476d286a6141139194` |
| Linux binary archive | `clorraine-1.1.2-linux-x86_64.tar.gz`, 4,489,640 bytes, SHA-256 `52f9e36461e27d85f06bdd7242f4ab0a4d3fe9ff07aa5eccabde9d9d33d38fbe` |
| Checksums | `SHA256SUMS`, 195 bytes, covering both archives |

The v1.1.2 binary archive contained `clc`, `clorraine`, `LICENSE`, `README.md`,
`RELEASE-NOTES.md`, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md` and
`docs/known-limitations.md`. It did not contain `clorraine-lsp`.

## Historical publications: v1.1.1, v1.1.0 and v1.0.0

The v1.1.1 release was published on 2026-11-09. Its source archive SHA-256 was
`a292933d163f317a7f95bc1a2ad6b163621ac9a28aa9130382fc5c5bbc9f281e`, and its
Linux binary archive SHA-256 was
`26847a3ed36f60c587f392fceb062e313973b7f5d287eecefeaef9c13acdd161`. Its binary
archive omitted `THIRD_PARTY_NOTICES.md` and `docs/known-limitations.md`, which
v1.1.2 fixed.

The v1.1.0 and v1.0.0 releases were published on 2026-11-08 and 2026-11-04.
Each used `clorraine-<version>-src.tar.gz`,
`clorraine-<version>-linux-x86_64.tar.gz` and `SHA256SUMS` with one
`CHANGELOG.md` entry and `RELEASE-NOTES.md`.
