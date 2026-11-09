# Requirements: Clorraine build/run driver (`clorraine`)

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

This plan implements no upstream RD, so this document is the owning
requirements doc (standalone full form).

## Feature Overview

`clorraine` is a developer-facing driver that builds and runs Clorraine
programs by orchestrating the existing emit-only compiler (`clc`) and the
profile's C toolchain and runner. Its purpose is to remove the C step from the
developer's mental model: a developer writes `.clo` source and runs one
command. The generated C is an implementation detail, surfaced only on an
internal compiler error or when explicitly requested.

The driver lives in the existing `compiler/` crate as a second binary and
reuses `clc`'s public library modules in-process. The compiler's own interface
and design are unchanged (AR-22).

## Functional Requirements

### Must Have

- [x] R1 — A `clorraine` binary exists in the `compiler/` crate; `clc` and the
  frozen specification are unchanged (AR-2, AR-22).
- [x] R2 — The commands are `run FILE…`, `build FILE…`, and `emit FILE…` (AR-3).
- [x] R3 — The profiles are `host` (default) and `hosted-amiga` (AR-4, AR-8).
- [x] R4 — Runners: `host` executes the binary natively; `hosted-amiga` uses
  `vamos` by default and FS-UAE behind `--emulator` (AR-5).
- [x] R5 — FS-UAE cannot be launched from the SDK `dev` container, so
  `--emulator` prints the exact host command and exits 0; no Docker socket is
  added (AR-6).
- [x] R6 — The emitted C and build artifacts live under
  `build/clorraine/<stem>/<profile>/` and are hidden by default; `emit` is the
  C escape hatch (AR-9, AR-12).
- [x] R7 — A C-compiler failure on the emitted C is reported as an internal
  compiler error with the C path and the target compiler's first error; exit 1
  (AR-10).
- [x] R8 — `run` streams the program's stdout (Amiga CR stripped) and exits
  with the program's own code; `--capture` writes it to `out.txt` instead
  (AR-11, AR-25).
- [x] R9 — The driver is an SDK-container tool: outside the container it prints
  guidance and exits 2, and a missing tool names the tool and the SDK remedy
  with exit 1 (AR-7, AR-23, AR-24).
- [x] R10 — The flag surface is `-p/--profile`, `--release`, `--emit-c`,
  `--emulator`, `--capture`, `-o` (emit only), `-h/--help`, `-V/--version`; an
  unknown flag is a usage error, exit 2 (AR-26).
- [x] R11 — Root `make run FILE=…` and `make build FILE=…` pass through to the
  driver inside the container (AR-14).

### Should Have

- [x] R12 — Debug is the default build (`-O0 -g`); `--release` selects `-O2`
  (AR-19).
- [x] R13 — One or more `.clo` files may be given; all are passed to `clc`, and
  the artifact stem is the first file's stem (AR-18).

### Won't Have (Out of Scope)

Project manifests (`clorraine.toml`); package/library builds; dependency
management; watch mode; LSP or debugger integration; ADF creation; a vbcc run
leg; non-Linux hosts; multiple output artifacts; and any change to `clc`'s CLI
(AR-21 of the MVP) or the frozen specification (AR-13, AR-22).

## Technical Requirements

### Performance

The driver is a development tool: it must feel instant for a small program. It
calls the compiler **in-process** (`clc::driver::run`) rather than spawning a
second binary, and performs no work beyond emit → compile → run (AR-17).

### Compatibility

The emitted C is the strict C99 subset of `spec/17-codegen.md` §1 and is built
with the §13 commands: host via `cc`, `hosted-amiga` via the GCC soft-float
68000 line (AR-16).

### Security

The driver treats source paths as untrusted data and invokes every external
tool with an **argument array**, never a shell string; it never resolves paths
from source; it writes only under the artifact directory and the `-o` target;
and it adds no network access, no credentials, and no Docker socket (AR-6,
AR-21, AR-22).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Driver boundary | separate bin / `clc` subcommands / separate crate | separate `clorraine` bin in `compiler/` | Keeps `clc`'s contract and no-spawn design; shares `profile`/`driver` | AR-1, AR-2, AR-22 |
| FS-UAE access | host handoff / Docker socket / host driver | host handoff (print command) | The `dev` container cannot launch a sibling container; smallest change | AR-6, AR-7 |
| C visibility | hide + `--emit-c` / always print | hide; `emit` escape hatch | The C is an implementation detail (P10) | AR-9, AR-12, AR-27 |
| C-build failure | ICE report / dump full C | ICE report (path + first error) | A target-C failure is the compiler's defect, not the user's | AR-10 |
| Input mode | flags only / manifest | flags only in v1 | Smallest viable; no config format to design yet | AR-13 |
| C command source | driver module / `clc::profile` / config | driver module | `clc` never spawns; the toolchain table belongs to the driver | AR-16 |

> **Traceability:** every row cites the Ambiguity Register entry that resolved
> it; see `00-ambiguity-register.md`.

## Acceptance Criteria

1. [x] `clorraine run hello.clo` builds and runs on `host` and prints the
   program's output, with no C toolchain detail shown.
2. [x] `clorraine run --profile hosted-amiga` builds with `m68k-amigaos-gcc` and
   runs under `vamos`; `--emulator` prints the FS-UAE host command.
3. [x] `clorraine emit [ -o OUT.c ] hello.clo` yields the same C that `clc`
   produces for the same inputs.
4. [x] A Clorraine diagnostic is shown unchanged and exits 1; a C-build failure
   is reported as an internal compiler error and exits 1.
5. [x] `run` propagates the program's exit code; `--capture` writes `out.txt`.
6. [x] The compiler's public behaviour and the specification are unchanged.
7. [x] `make check` and `make verify` stay green, including the new driver
   tests.
8. [x] `AGENTS.md` documents the driver; `plans/00-roadmap.md` carries the new
   `PLAN-02` row.
