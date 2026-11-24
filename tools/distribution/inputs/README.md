# Retained distribution inputs

These files are retained licence and origin evidence from the former native
support/distribution pipeline. They are not a build pipeline and are not
installed by the Linux CLI.

- `libamiga.a` is the retained ordinary target library; its size and SHA-256
  are recorded in [`THIRD_PARTY_NOTICES.md`](../../../THIRD_PARTY_NOTICES.md).
- `licence-review.md`, `build-licences.md`, `python-licences.md`,
  `payload-origins.md`, `ndk-inputs.md`, and `wheel-builds.md` are provenance
  and licence reviews.
- `source-forms/` and `compiler-patches/` hold source-form evidence.
- `environment.Dockerfile`, `build-environment.sh`, and
  `ubuntu-packages.sha256` describe the reference build environment.
- `reference-support.sh`, `support-payload.sh`, and `wheel-payload.py` are
  historical reference scripts.

The supported development environment is built by `tools/setup/install_sdk.py`
and `make setup`. The Linux CLI installer is `tools/setup/install.sh`, and the
local candidate archive is built by `make dist`.
