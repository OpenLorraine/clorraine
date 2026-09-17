# Clorraine

Clorraine is the systems language of **LorraineOS** — a bare-metal Amiga OS for
the Motorola 68000 family, with an ARM64 port planned.

Clorraine is a safe, C-like language with Rust-style ownership and borrow
checking. It is designed for systems programming first and compiles to plain C,
so the generated code runs on the existing Amiga toolchains and, later, on
ARM64.

Like C and Unix — and later Oberon — the language and the OS are co-designed:
LorraineOS requirements drive Clorraine's feature set, and Clorraine exists to
make LorraineOS (and Amiga programs) safe and pleasant to write.

## Status

Early development. The reproducible Docker toolchain is ready; the language
specification and compiler come next.

| Component | State |
|---|---|
| Amiga SDK image (GCC 16.2, binutils 2.46, gdb, libnix, NDK 3.2, vbcc/vasm/vlink) | Ready |
| FS-UAE + `vamos` run/debug loop (`make run`, `make run-vamos`, ADF fallback) | Ready |
| Phase 0 hosted skeleton (ownership contracts, cleanup shapes, run matrix) | Complete |
| Language specification v0.3 — Phase A decisions locked ([ledger](spec/decisions.md)) and the Phase B grammar freeze ([grammar](spec/grammar.ebnf.md), [deltas](spec/grammar-deltas.md)); chapters next | In progress |
| Compiler MVP (C-like → C with ownership checks) | After the spec |
| LorraineOS bring-up (freestanding profile, m68k) | With/after the MVP |
| ARM64 port | Later |

## Repository layout

```
docker/    The Clorraine SDK: pinned, from-source Amiga toolchain in Docker
spec/      The language specification: decision ledger, grammar freeze, chapters
docs/      Findings and review records (Phase 0 toolchain, pre-Phase-B ergonomics)
```

## Quick start

```sh
cd docker
make image     # build the SDK image (first time only, compiles the toolchain)
make init      # create docker/.env and ~/amiga for ROMs/ADFs
make smoke     # verify the toolchain
make shell     # interactive development shell
```

See [docker/README.md](docker/README.md) for workflows, the program runner,
ROM policy, X11 troubleshooting, and how to update the toolchain.

## Organization

Developed under [OpenLorraine](https://github.com/OpenLorraine). LorraineOS
lives in its own repository.

## License

To be determined.
