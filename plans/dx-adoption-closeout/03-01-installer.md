# SDK Installer: DX and Adoption Closeout

> **Document**: 03-01-installer.md
> **Parent**: [Index](00-index.md)

## Overview

One entry point answers "is my Clorraine SDK ready?" and, when asked, prepares
the public parts. It detects the host and the pinned image, reports licensed
media without touching it, and delegates every download to the existing Docker
build. Requirements: R1-R3, AR-27 to AR-32.

## Architecture

### Current Architecture

`docker/scripts/init-env.sh` creates `docker/.env` and scans `$HOME/amiga`.
`docker/Makefile` owns the image build, compose invocation, and smoke test.
There is no aggregate report and no `make setup`.

### Proposed Changes

```
tools/setup/
├── install_sdk.py          # CLI, detection, fetch plan, report
├── checks.py               # pure predicates over a Probe result
└── tests/
    ├── test_detect.py
    ├── test_fetch.py
    └── test_cli.py
```

`install_sdk.py` is a single executable module. It reads configuration, builds
a `Probe` (all observed facts), turns it into a `Report` (component rows), and
prints either text or JSON. Fetch runs only after the report and only through
`run_command`.

## Implementation Details

### CLI

```text
install_sdk.py [--root DIR] [--amiga-data DIR] [--fetch] [--dry-run]
               [--yes] [--require-media] [--json]
```

| Flag | Meaning |
| ---- | ------- |
| `--root DIR` | Repository root; default is the parent of `tools/setup/` |
| `--amiga-data DIR` | Media root; overrides `.env` and `$AMIGA_DATA` |
| `--fetch` | Build the image and create `.env` when missing |
| `--dry-run` | Print the report and fetch plan; run no state-changing command |
| `--yes` | Skip the interactive confirmation |
| `--require-media` | Treat a missing ROM or Workbench image as `missing` |
| `--json` | Print one JSON object instead of the table |

Exit codes: `0` ready (or a successful dry run), `1` an unmet required
component or failed verification, `2` a usage error.

### Probe

A `Probe` is a dataclass with booleans and strings only:

| Field | Source |
| ----- | ------ |
| `docker_cli` | `docker version --format {{.Client.Version}}` |
| `docker_daemon` | `docker version --format {{.Server.Version}}` |
| `compose` | `docker compose version --short` |
| `image_id` | `docker image inspect <IMAGE> --format {{.Id}}` |
| `env_file` | `docker/.env` exists |
| `media_root` | `--amiga-data`, else `.env` `AMIGA_DATA`, else `$AMIGA_DATA`, else `$HOME/amiga` |
| `roms` | `*.rom` directly under `<media>/roms` |
| `adfs` | `*.adf` directly under `<media>/adf` |
| `hdfs` | `*.hdf` directly under `<media>/hdf` |
| `host_cc` | `cc` or `gcc` on `PATH` |
| `tools` | one `docker run --rm --entrypoint sh <IMAGE> -c ...` result for `m68k-amigaos-gcc`, `m68k-amigaos-gdb`, `vc`, `vamos` |
| `ndk` | the same probe checks `/opt/amiga/m68k-amigaos/ndk-include` is a non-empty directory |
| `provenance` | the image's `/opt/amiga/BUILD-PROVENANCE.txt` `amiga-gcc commit:` line |

The image probe runs only when the image exists. Every probe command has a
10-second timeout; the image build and `make init` in a fetch have no timeout.
A missing command is data, not an exception.

`.env` is read as `KEY=value` lines. A `#` at the start of a trimmed line is a
comment; a value is taken literally, with one pair of matching single or double
quotes stripped. Nothing is evaluated as shell.

### Report

Rows are emitted in this fixed order: docker, compose, image, env, toolchain,
ndk, host-cc, roms, workbench. State is `ok`, `missing`, or `optional`.
`--require-media` changes only the last two rows and the exit code.

Text output:

```text
Clorraine SDK
  [ok]      docker          27.5.1
  [ok]      image           clorraine-sdk:dev (sha256:...)
  [missing] env             docker/.env not found; run: make init
  [optional] kickstart     0 ROM(s) in /home/me/amiga/roms
```

JSON output has stable top-level keys: `ready`, `image`, `media_root`,
`components` (array of `{name, state, detail, hint}`), `actions` (array of
strings), and `plan` (the commands a fetch would run). No file contents are
included.

### Fetch plan

`--fetch` computes a minimal ordered plan:

1. If the image is missing: `make -C <root>/docker image`.
2. If `docker/.env` is missing: `make -C <root>/docker init`.
3. Re-probe and verify.

No other action exists. ROMs, ADFs, HDFs, and the NDK are never downloaded or
copied by the installer. The report states that the image build is the
downloader and that upstream checksum verification applies where upstream
publishes checksums (AR-29).

### Verification

After a fetch, or on an already-installed image with `--fetch`, the installer
fails closed when:

- the image probe fails;
- the provenance commit differs from `AMIGA_GCC_COMMIT` in
  `docker/versions.env`;
- the NDK directory is missing or empty;
- a required tool has no version line.

The failure names the component and prints the exact recovery command.

### Confirmation

A terminal run prints the plan and asks `Proceed? [y/N]`. A non-terminal run
without `--yes` exits 2 with `use --yes in a non-interactive run (or
--dry-run)`. `--dry-run` never asks and never changes state.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Docker absent | Report `missing`; exit 1 only if a fetch was requested or the user requires a ready SDK | AR-27 |
| Image absent | Report `missing`; fetch builds it when asked | AR-28 |
| Compose absent | Report `missing`; no fetch action | AR-27 |
| Media root absent | Report `optional`; never create it | AR-31 |
| `docker/.env` unreadable | Report `missing`; do not overwrite | AR-30 |
| Provenance mismatch | Exit 1; no retry | AR-29 |
| Build failure | Print the failing command and recovery; exit 1 | AR-29 |
| Non-interactive without `--yes` | Exit 2 with the exact flag to add | AR-30 |
| Unknown flag | Exit 2 with usage | AR-27 |

## Testing Requirements

Unit tests use `tempfile.TemporaryDirectory`, a fake `docker` executable on
`PATH`, and a monkeypatched `run_command`. No test touches `~/amiga`,
`docker/.env`, the real image, or the network. See ST-1 to ST-12 in
[Testing Strategy](07-testing-strategy.md).
