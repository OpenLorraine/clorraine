# Clorraine Phase 0: hosted walking skeleton

Hand-written C that represents what the Clorraine compiler will eventually
emit for a hosted AmigaOS program. There is no Clorraine compiler yet; the
point of this skeleton is to validate, before the language exists:

* exec/dos **ownership contracts** (`returns-owns`, `consumes`, `borrows`)
* releases that run on **every exit path** and are **null-safe**
* **exactly-once** release semantics, verified by counters
* a **strict common C subset** accepted by both GCC and vbcc
* the shared **run matrix**: `vamos`, FS-UAE directory drive, FS-UAE ADF

Two cleanup shapes are implemented so Phase 0 can choose the canonical
emitter pattern:

| Shape | Description |
|---|---|
| `single` | one `goto cleanup`, releases in reverse declaration order |
| `scoped` | each resource released at the end of the scope that acquired it |

Each shape runs three variants: `happy`, `alloc-fail` (AllocVec asked for an
impossible size) and `open-fail` (Open on a missing directory).

## Build

```sh
make build        # GCC -m68000, GCC -m68020, vbcc; -Wall -Wextra -Werror
```

## Run

```sh
make run-vamos    # fast path; no emulator, no ROM
make run          # FS-UAE directory drive, output captured to out.txt
make run-adf      # FS-UAE bootable ADF, output extracted from the image
make verify       # diff vamos output against FS-UAE output
```

The FS-UAE runs need the SDK image and `docker/.env` (`make -C docker init`).

## Expected output

See `expected-output.txt`:

```
phase0: single happy      ok
phase0: single alloc-fail ok
phase0: single open-fail  ok
phase0: scoped happy      ok
phase0: scoped alloc-fail ok
phase0: scoped open-fail  ok
phase0: acquisitions=12 releases=12
phase0: all scenarios ok
```

Total acquisitions equal total releases: nothing leaked, nothing released
twice.

## Notes

* `ffi_contracts.h` is the seed of the runtime shim and the contract-table
  format that becomes the Clorraine annotation syntax.
* Message ports, IO requests, pools and the message ownership transfers
  (`GetMsg`/`PutMsg`/`ReplyMsg`) are deliberately out of scope for Phase 0.
