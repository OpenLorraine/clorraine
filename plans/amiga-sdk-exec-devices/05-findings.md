# Findings: Amiga SDK Bundle 7 — Exec and Devices

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: In progress — implementation and closeout findings

## Implementation findings

These are target/implementation findings surfaced while executing the plan.
The plan's preflight report records the document-level findings (PF-001–PF-033);
this ledger records what the real 1.3 machine and the emitted C taught us.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| RV-1 | 🟠 Major | The 1.3 `keyboard.device` does not keep a `CMD_READ` pending for the abort proof: the first A500 run returned an immediate event/error (`exec: err 13`), and `CMD_FLUSH` is not a supported keyboard command (`exec: err 11`). | **Fixed** — the abort proof moved to a `console.device` `CMD_READ` on the demo's dedicated window, where no input is pending; the read is aborted and returns `IOERR_ABORTED` (-2). `keyboard.device` and `input.device` stay in the demo as open/close coverage. The final gate reports `key -2`. |
| RV-2 | 🟠 Major | The smoke's `grep -aq "exec: ok task"` matched the **program binary** on the ADF (the format-string literal), so it reported the file before the demo had written it. The ADF listing showed no `exec.txt`. | **Fixed** — the smoke matches the runtime value `exec: ok task 464` (not in the binary), and fails clearly with the ADF listing when `exec.txt` is missing. |
| RV-3 | 🟡 Minor | The hello gate's `430x22` crop cut the 50-character Exec line at the emulator's 1.5x scale; the first golden ended at `key`. | **Fixed** — the smoke crops `620x22` at `(160,88)`; the recaptured golden shows the full `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1` line. |
| RV-4 | 🟡 Minor | `#include <intuition/window.h>` does not exist in the 1.3 NDK, so the emitted C failed to compile until the include was removed; `intuition/intuition.h` provides the complete `struct Window` used by `sizeof`. | **Fixed** — the shim keeps `intuition/intuition.h` and drops the nonexistent header. |
| RV-5 | 🟡 Minor | The `Library` handle is a contract handle mapped to `struct Library *`, not an opaque `void *`; the first shim signatures conflicted with the emitted prototypes. | **Fixed** — `cl_open_library`/`cl_close_library`/`cl_library_version`/`cl_library_revision` use `struct Library *`, matching `compiler/src/abi.rs`. |
| RV-6 | 🟡 Minor | The `deviceSetBuffer` FFI must take `raw u8` + `u32`; an array parameter crosses as one `APTR` and the length would never reach the shim. | **Fixed** as planned by PF-022; the wrapper passes `raw u8(data.ptr)`/`u32(len(data))` and the shim takes `APTR` + `ULONG`. |

## Closeout verification

| Gate | Result |
|---|---|
| `cargo test --lib --test amiga_exec_spec_test --test amiga_exec_impl_test` | 400 lib + 11 impl + 14 spec green |
| `make sdk-a500-exec` | `exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1`; 620x22 golden matches |
| `make check` | pending |
| `make verify` | pending |
| `make sdk-registry` | pending |
| GitHub issue #8 | pending |
