# Testing strategy

> **Rule**: Specification tests are written first and must fail for the right
> reason. A test that passes with the behavior removed is not evidence.

## Test pyramid

| Layer | Location | Runs in |
|---|---|---|
| Public CLI/toolchain contract | `compiler/tests/toolchain_setup_spec_test.rs` | `cargo test --workspace` / `make check` |
| FS-UAE runner contract | `compiler/tests/fs_uae_spec_test.rs` | `cargo test --workspace` / `make check` |
| Internal parsing/staging | `#[cfg(test)]` in `setup.rs`, `environment.rs`, `emulator.rs`, `cli.rs` | `cargo test --workspace` |
| Installer shell contract | `tools/setup/tests/install.spec.test.sh`, `install.impl.test.sh` | `make dist-check` |
| Producer contract | `tools/setup/tests/dist.spec.test.sh` | `make dist-check` |
| Installer acceptance checker | `tools/setup/tests/acceptance_spec.py` | `make dist-check` |
| Editor contract | `tools/vscode/test/*.spec.test.ts`, `*.impl.test.ts` | `make vscode-check` |
| Real toolchain build | `make toolchain-check TOOLCHAIN=...` | local only |
| Real FS-UAE guest | `make fs-uae-check KICKSTART=...` | local only |
| Candidate acceptance | `make acceptance`, `make acceptance-check` | local only, Phase 4 |

The fake-toolchain and fake-FS-UAE seams keep CI hermetic. The real toolchain,
ROM, X display, and media are never CI requirements.

## Scenario table

| ID | Requirement | Scenario | Oracle | Must fail without |
|---|---|---|---|---|
| ST-1 | R2 | Fake `uname` reports non-Linux/non-x86_64 | exit 1, capability message | The arch/OS check is removed. |
| ST-2 | R2 | Fake `getconf` reports glibc 2.35 | exit 1 before any CLI execution; names 2.38 | The glibc check is removed. |
| ST-3 | R2 | PATH missing `tar`/`gzip`; missing `curl` in network mode only | exit 1, lists the missing command and hint; local mode works without `curl` | The command check is removed or `curl` is required offline. |
| ST-4 | R3 | Network archive wrong hash or byte count | no extraction, exit 1 | The hash/size check is removed. |
| ST-5 | R3 | Local exact bytes; wrong hash; missing/duplicate line | exact installs and records size; others fail | Local mode bypasses the checksum. |
| ST-6 | R14 | Interrupted activation, live/stale/concurrent lock, PATH quoting/replacement, uninstall/purge with/without `XDG_CONFIG_HOME` | old `current` remains usable; one lock owner; one quoted block; config/versions preserved unless `--purge` | Activation moves the old tree, PID-only lock, duplicated/unanchored PATH block, or wrong config path. |
| ST-7 | R4 | Missing dir, missing compiler, escaping symlink, incomplete tree | exit 2, names the missing path | Layout validation is removed. |
| ST-8 | R4, R6 | Complete fake toolchain both crt probes; wrong `--version` | setup succeeds for rc24; wrong version rejects | The `nix13` probe or version check is skipped. |
| ST-9 | R4 | Probe failure, then valid setup | failed setup leaves old config byte-identical | Config is written before validation. |
| ST-10 | R5 | Flag/env/config precedence; invalid selected value; marker-gated legacy PATH; PATH-only host | flag wins; no fallback after error; marker uses handoff; non-marker PATH errors | Precedence falls through, or a user host bypasses setup. |
| ST-11 | R5 | (a) fake compiler records absolute path/flags/probes; (b) local rc24 builds hello, four corpus, freestanding | both pass | The compiler path is not absolute, a flag changes, or freestanding regresses. |
| ST-12 | R7 | FS-UAE too old/missing; 256 KB ROM; both accepted ROM sizes; invalid timeout | exit 2 with the specific reason | Any readable ROM or old emulator is accepted. |
| ST-13 | R7, R8 | Wrapper staging and config | `runner`, `program`, `CNEXT:runner`; wrapper compile flags | The two-line echo script is used. |
| ST-14 | R8 | Wrapper writes `0\n` + `ok\n` | trims newline; exit 0, exact stdout | The runner returns an emulator/known-bad status or mishandles the newline. |
| ST-15 | R9 | Guest rc 1/5/9, empty/partial rc or status, malformed markers | exit 1, rc named, output retained | A return 1–9 can pass, or an empty/partial marker is read as complete. |
| ST-16 | R8, Q3 | Timeout; invalid timeout; partial rc; read/capture failure; unreaped child | failure; result retained; bounded kill/wait | The directory is removed before read, or wait blocks forever. |
| ST-17 | R10 | `--emulator --capture` | `out.txt` exists before cleanup; path printed | Capture is rejected or written after cleanup. |
| ST-18 | R1, R12 | Archive layout, release README/notice links, hostile symlink/hardlink/absolute member | only CLI binaries and release docs; hostile members reject; no forbidden files | A ROM/toolchain/extra file sneaks in, a hostile member extracts, or a link is dangling. |
| ST-19 | R12 | Producer: version mismatch, extra file, stale pin, SHA256SUMS coverage, VSIX presence | each fails or passes as intended | `make dist`/`dist-verify` only checks happy-path bytes. |
| ST-20 | R1, R4, R7 | Fresh HOME: local install, setup, hello, four `hosted-amiga` corpus, one `hosted-amiga-13` corpus, one rc-1 negative, FS-UAE | outputs match; negative exits 1; evidence written | Any step is skipped, the negative passes, or the candidate changes. |
| ST-21 | R11 | Editor discovery and legacy warning | `~/.local/bin`, explicit path, one warning via `globalState` | The extension still searches `~/.clorraine/bin` or warns every activation. |
| ST-22 | R11 | Editor actions | `--toolchain`, `--fs-uae`, `--kickstart`, no `--sdk-root`, one toolchain flag | The action passes the old flag or two toolchain flags. |
| ST-23 | R11 | Removed/added settings and fixtures | `toolchain.path` present, `sdk.path` gone, review spec/fixtures updated | The manifest or review spec keeps the old key. |
| ST-24 | R12 | `dist-check` in `make check` | fake producer/installer tests pass without network/Docker | The target needs the real archive. |
| ST-25 | R12 | Acceptance validator | rejects wrong toolchain/FS-UAE/hello/corpus/VSIX/candidate hash and a missing/failed negative | A macOS or wrong-output record passes, or the negative proof is missing. |
| ST-26 | R13, Q4 | Removal keeps inputs/notices/README refs | retained paths resolve; no old names remain | A blanket delete removes inputs or leaves dangling refs. |
| ST-27 | R13 | Removal CI/Makefile | `make check` runs `dist-check`/`setup-check`; no `distribution-check` | A stale target is referenced. |
| ST-28 | Q7 | Docs claims | glibc 2.38, rc24, KS3.1, wrapper oracle, AROS manual-only; site/release notes/changelog | Docs still say Ubuntu 22.04, bundled SDK, or installer-placed VSIX. |

The real-emulator scenarios are ST-20 and the local `make fs-uae-check`; they
are not CI tests. The spike already ran the same wrapper recipe; acceptance
repeats it from the candidate archive.

## Test integrity checks

Before marking a scenario green:

- Remove the check (or revert the implementation) and confirm the test fails.
- For installer tests, verify the fake tools cannot make a bad archive pass,
  and that `uname`/`getconf` fakes exercise the host checks before extraction.
- For the FS-UAE tests, verify the fake emulator can write rc 1/5/9 and a
  partial status; confirm the runner fails and retains output.
- For the real guest, build a temporary rc-1 program and confirm the wrapper
  produces no status and the CLI exits 1.
- For the editor, verify the legacy warning key is persistent and a second
  activation does not warn; verify new-key precedence.
- For acceptance-check, remove or alter one required field/value and confirm
  rejection.

## Commands

| Command | Scope |
|---|---|
| `make check` | Spec check, SDK registry, Docker SDK setup tests, hermetic `dist-check` (after Phase 3), workspace tests, benchmark/fuzz fast legs |
| `make toolchain-check TOOLCHAIN=PATH` | Real rc24 build of hello, corpus, and freestanding |
| `make fs-uae-check KICKSTART=PATH` | Real A1200/KS3.1 hello, corpus, and rc-1 negative |
| `make dist VERSION=1.2.0` | Local candidate archive, VSIX, installer, checksums |
| `make dist-verify` | Pins, versions, asset hashes, forbidden contents |
| `make dist-check` | Hermetic producer/installer tests |
| `make acceptance CANDIDATE=... TOOLCHAIN_ARCHIVE=... KICKSTART=...` | Fresh-HOME end-to-end |
| `make acceptance-check EVIDENCE=... CANDIDATE=...` | Evidence validator |
| `make vscode-check` | Type-check, unit tests, package audit, packaged client |
| `make site-check` | Clean site install and production build |
| `make verify` | Full repository acceptance when the pinned SDK image and media are available |

`make check` stays the fast gate. `make verify` is the release gate and
includes the existing A500/A1200/RTG gates. If the pinned SDK image or media is
unavailable, the plan is not complete; record the blocking environment fact
rather than skipping the gate.

## Evidence retention

Acceptance writes under `build/acceptance/<timestamp>/` (gitignored):

- `evidence.json`;
- `install.log`, `setup.log`, `build.log`, `fs-uae.log`;
- `output.txt`, the corpus outputs, and the rc-1 negative rc/output;
- the exact `SHA256SUMS`, `install.sh`, and VSIX used.

The evidence records the archive, installer, VSIX, toolchain, ROM, FS-UAE, and
output hashes. No licensed ROM bytes or toolchain archive are committed.
