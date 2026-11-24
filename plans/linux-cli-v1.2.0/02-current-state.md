# Current state

> **Code basis**: `0ebb3fb8` (docs-only commit on top of `8f454eae`)
> **Evidence**: `spikes/07-linux-cli-fs-uae/README.md`

## What already works

| Area | Fact |
|---|---|
| Emitted C | The upstream rc24 `m68k-amigaos-gcc` builds the current translation units for `hosted-amiga` (`nix20`) and `hosted-amiga-13` (`nix13`) with the existing flags. |
| Corpus | `file-copier`, `string-parser`, `buffer-container`, and `volatile-mmio` build and produce their expected output on A1200/KS3.1. |
| Guest boot | FS-UAE 3.2.35 boots an A1200 directory drive with a licensed KS3.1 A1200 ROM and writes files back to the host. |
| Legacy SDK path | `--emulator` without a native SDK prints the host handoff; the existing handoff and capture tests pin this behavior. The new native runner must not change it. |
| Editor actions | The extension already selects saved sources, builds tasks, and passes FS-UAE/ROM settings for both Amiga profiles. |
| Freestanding | A `print` probe builds with the upstream `-ffreestanding -fno-builtin -nostdlib -lg` line; it was not run because there is no LorraineOS target. |
| Host tests | The existing Rust workspace tests and the A500/A1200 SDK gates remain the regression net. |

## What blocks the product path

| Area | Current code | Gap |
|---|---|---|
| Toolchain selection | `mod.rs:145-147` checks the SDK marker; `environment.rs:41-70` selects the old native assembly, and `environment.rs:100-106` requires the paired `sdk/vamos` venv for a non-emulator run. The native assembly needs `<root>/sdk/amiga`, `installation.json`, and `.ndk-prepared.json`. | The upstream rc24 root (`bin/m68k-amigaos-gcc`, `m68k-amigaos/`) is rejected. |
| CLI surface | `cli.rs:104` is the help text for `--sdk-root`; the parser is `cli.rs:280-283`. `cli.rs:120` says native emulator runs return the emulator status, not the guest status. | The product needs a toolchain selector and a guest result. |
| Build | The compiler path comes only from the old native SDK assembly | `--toolchain`/config/env cannot select the upstream compiler. |
| FS-UAE staging | `emulator.rs:129-167` stages only `program` and `program\n`; `emulator.rs:152` hard-codes `amiga_model=A1200`; `emulator.rs:70-125` runs the emulator and `:123` cleans storage after it exits | No success marker, no output capture, no status distinction, and the result would be deleted before inspection. |
| Capture | `mod.rs:105-107` rejects `--capture` with a native emulator run | Scripts cannot collect guest output. |
| ROM | `emulator.rs:52-67` checks only a readable ordinary file | A 256 KB KS1.3 ROM fails later with an opaque emulator error. |
| glibc | No host check in the CLI or installer | An Ubuntu 22.04 host gets `GLIBC_2.38 not found` instead of a clear requirement. |
| Installer | `tools/setup/install.sh` downloads a Python helper; the old native installer and support pipeline sit in `tools/setup/` and `tools/distribution/` | Neither installs the new CLI archive nor validates the upstream toolchain. |
| Editor discovery | `discovery.ts:92` searches `~/.clorraine/bin`; `actions.ts:241-243` passes `--sdk-root` | The new install prefix and toolchain contract are not discoverable. |
| CI | `.github/workflows/check.yml:17` runs `ubuntu-latest`; `Makefile:63-64` runs `setup-check` and `distribution-check` | The host minimum and the superseded installer tests need a deliberate change. |

## Reusable code

| File | Reuse |
|---|---|
| `compiler/src/clorraine/toolchain.rs:61-64` | Existing `nix20`/`nix13` flag tables; keep the flags. The tool definitions are `:186-200`. |
| `compiler/tests/native_driver_support/`, `native_gate_spec_test.rs` | Old `sdk/amiga` fixtures; rewrite for the new toolchain root or remove obsolete cases. |
| `compiler/src/clorraine/emulator.rs:70-125,129-167,180-241` | Run loop, staging, private storage, isolated HOME/XDG paths, literal argv, and the A1200 config skeleton. Replace the startup script and add the wrapper/result loop. |
| `compiler/src/clorraine/artifacts.rs` | `out.txt` location and build-tree rules for `--capture`. |
| `tools/vscode/src/actions.ts:236-255` | Existing saved-source task construction and literal argument checks. Change the toolchain flag and keep FS-UAE/ROM pass-through. |
| `tools/vscode/src/discovery.ts:57-111` | Existing executable/PATH discovery. Add the new install prefix and keep explicit user settings authoritative. |
| `docker/scripts/run.sh:75-100` | Proven startup-sequence capture pattern; the product runner adopts the same idea without the Docker wrapper. |

## Constraints

- `spec/` is frozen. This plan does not change language behavior, emitted C, or
  existing compiler flags.
- The toolchain is user-supplied and never bundled. The spike pin is rc24,
  `69,868,548` bytes, SHA-256 `b93b070a…25c100`.
- The host minimum is glibc 2.38 because the rc24 host binaries require it.
  Ubuntu 24.04 (2.39) works; Ubuntu 22.04 (2.35) fails at the loader.
- FS-UAE needs an X display. The acceptance runner uses `xvfb-run`; the product
  does not bundle or install FS-UAE.
- The A500/Kickstart 1.3 path stays in the existing SDK gates. The CLI's
  `hosted-amiga-13` profile is built and may run on the A1200 recipe, but that is
  not a 1.3 compatibility proof.
- The repository currently has unrelated modified agent briefs in
  `.opencode/agents/`. This plan does not touch them.

## Gap summary

| Gap | Owning design | Tests |
|---|---|---|
| Upstream toolchain validation and selection | `03-01-toolchain.md` | ST-7–ST-11 |
| Verified FS-UAE guest run | `03-02-fs-uae.md` | ST-12–ST-17 |
| Installer, archive, acceptance | `03-03-install-release.md` | ST-1–ST-6, ST-18–ST-20, ST-24–ST-28 |
| Editor and legacy removal | `03-04-editor-migration.md` | ST-21–ST-23, ST-26–ST-28 |
