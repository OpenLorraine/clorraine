# Design: editor and legacy migration

> **Requirements**: R11, R13, Q5, Q7
> **Depends on**: `03-01-toolchain.md`, `03-02-fs-uae.md`,
> `03-03-install-release.md`

## Editor discovery

The installer default prefix is `~/.local`, so the driver lands at
`~/.local/bin/clorraine` and the server at `~/.local/bin/clorraine-lsp`.
Discovery keeps this order:

1. Explicit `clorraine.driver.path` / `clorraine.lsp.path` (machine scope).
2. `CLORRAINE_LSP` for the server.
3. `~/.local/bin/<name>`.
4. Trusted workspace `target/{debug,release}/<name>`.
5. Absolute directories on `PATH`.

`~/.clorraine/bin` is removed from discovery. The old native install is
superseded; a user with it must install 1.2.0 again.

## Settings

| Setting | Action |
|---|---|
| `clorraine.driver.path` | Keep. |
| `clorraine.lsp.path` | Keep. |
| `clorraine.toolchain.path` | Add. Upstream root containing `bin/m68k-amigaos-gcc`. Empty uses the CLI config. |
| `clorraine.sdk.path` | Remove from `package.json`; read once if `toolchain.path` is empty, pass as `--toolchain`, and show a warning. The new key always wins; never append two `--toolchain` flags. |
| `clorraine.fsUae.path` | Keep; pass as `--fs-uae`. |
| `clorraine.kickstart.path` | Keep; pass as `--kickstart`. |
| `clorraine.profile` | Keep. |

The legacy warning is shown once per VS Code profile using the persistent
`context.globalState` key `clorraine.legacySdkPathWarningShown`. The message
says the old SDK layout is no longer used and asks the user to select the
upstream toolchain root in `clorraine.toolchain.path`. The warning never blocks
the action; the CLI reports the actual validation error. The test mocks are
extended with a persistent `globalState` map.

The machine-only scoping rule stays: workspace settings cannot select an
executable or override the toolchain/ROM paths.

## Actions

For a non-host profile:

- if `clorraine.toolchain.path` is nonempty, append
  `--toolchain <path>`;
- else if legacy `clorraine.sdk.path` is nonempty, append
  `--toolchain <legacy path>` and warn once;
- otherwise append nothing and let the CLI use `CLORRAINE_TOOLCHAIN` or its
  config;
- never append `--sdk-root` or two `--toolchain` flags.

For `clorraine.runInFsUae`, append `--emulator` and the FS-UAE/ROM settings as
today. `clorraine.run` stays the non-emulator (vamos) action; the README states
that it needs `vamos` on `PATH`, while `Run in FS-UAE` is the verified product
path.

## Documentation

Update:

- `tools/vscode/README.md`: install prefix, settings table, toolchain root vs
  old `sdk/amiga`, FS-UAE/ROM requirement, `vamos` caveat, the VSIX install
  command, and the one-time warning.
- `README.md`: Linux install, toolchain pin, KS3.1 requirement, quick start.
- `docs/`: a short Linux CLI page and `docs/known-limitations.md` entries for
  AROS manual-only, no macOS/Windows, no bundled media, and Ubuntu 24.04's
  3.1.66 FS-UAE package being below the 3.2.0 requirement.
- `site/guide/quick-start.md`, `site/status.md`, `site/reference/cli.md` and
  related site pages: remove `~/.clorraine`, `install.pyz`, Ubuntu 22.04,
  bundled SDK/`vamos`, installer-placed VSIX, default AROS, and `--root`
  claims.
- `RELEASE-NOTES.md`, `CHANGELOG.md`, and the superseded `RELEASE.md`: replace
  runnable commands for removed producers/workflows and state the new path.
- `AGENTS.md`: command table and layout changes after the removal.
- Run `make site-check` and a claims grep for the removed names/versions.

## Migration: remove the old pipeline

Remove only after the local acceptance in Phase 4 passes.

### Remove

| Path | Reason |
|---|---|
| `tools/setup/install_native.py`, `native_archive.py`, `native_state.py` | Old native installer/archive/state. |
| `tools/setup/tests/native_*`, `test_native_*` | Tests for the removed native installer. |
| `tools/setup/install.sh` old body | Replaced by the new Linux bootstrap (same path). |
| `tools/distribution/*.py`, `*.sh`, `support-lock.json`, `tests/`, `README.md` | Superseded support/release producers. |
| `.github/workflows/distribution-release.yml`, `support-release.yml` | Old distribution pipeline. |
| `Makefile` `distribution-check` target and `make check` call | Replaced by `dist-check`. |

### Keep

| Path | Reason |
|---|---|
| `tools/setup/install_sdk.py`, `tools/setup/tests/support.py`, and the existing `test_cli.py`, `test_detect.py`, `test_fetch.py` | Docker SDK detection/preparation used by `make setup`; not the public CLI installer. |
| `tools/distribution/inputs/` | Licence/origin evidence referenced by `THIRD_PARTY_NOTICES.md`. |
| `docker/` | Developer SDK image and existing Amiga gates. |
| `tools/vscode/` | Product editor client. |

`make setup-check` stays as the Docker SDK installer test suite. `make check`
runs it and the new hermetic `dist-check`. The names are documented so the two
installers are not confused.

### Makefile and CI

- Remove `distribution-check` from `.PHONY` and `make check`. Its `make check`
  call was already removed in Phase 3 when the new installer replaced the
  bootstrap template those tests pinned; the target and files are removed here.
- Add `dist-check` to `.PHONY` and `make check`.
- Keep `setup-check` for `install_sdk.py` tests.
- Update `.github/workflows/check.yml` to `runs-on: ubuntu-24.04` and replace
  the raw installer step with `make dist-check` plus `make setup-check` (or run
  the same commands explicitly).
- Do not add FS-UAE, ROM, toolchain, network, or acceptance steps to CI.

### Notices and references

- Keep `tools/distribution/inputs/libamiga.a` and the notices that reference it.
- Add a removal test that the retained inputs exist, every
  `tools/distribution/inputs/...` path named in `THIRD_PARTY_NOTICES.md`
  resolves, and retained `tools/distribution/inputs/README.md` does not point
  at the removed `support-lock.json`.
- Run a repository-wide text search for the removed names and update
  `AGENTS.md`, `README.md`, `docs/`, `site/`, `RELEASE-NOTES.md`,
  `CHANGELOG.md`, `RELEASE.md`, and `Makefile` help text.
- The removal commit is separate from the feature commits so it can be reviewed
  and reverted independently.

## Tests

| Test | Covers |
|---|---|
| `tools/vscode/test/discovery.spec.test.ts` | New prefix, explicit paths, legacy warning, no old prefix, and the optional-path argument list. |
| `tools/vscode/test/actions.spec.test.ts`, `actions.review.spec.test.ts` | `--toolchain` argument, no `--sdk-root`, FS-UAE/ROM pass-through, both profiles, new-key precedence. |
| `tools/vscode/test/fixtures/action-host.cjs`, `discovery-host.cjs` | `toolchain.path` default, legacy `sdk.path`, persistent `globalState`. |
| `tools/vscode/test/manifest.test.ts` | Removed/added settings in `package.json`. |
| `tools/setup/tests/test_retained_inputs.py` (new) | Retained inputs, notices, and retained README references resolve after removal. |
| `make vscode-check` | Type-check, unit tests, package audit, packaged client. |

The editor specs are extended first (red), then the implementation changes; the
existing VS Code mock fixtures are extended with `globalState`. They do not
launch a real editor or emulator.
