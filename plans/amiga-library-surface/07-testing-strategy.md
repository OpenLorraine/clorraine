# Testing Strategy: Amiga Library Surface

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Cases

| # | Scenario | Expected | Source |
|---|---|---|---|
| ST-1 | `clc --profile host` on a program importing `std.amiga.exec` | `E11200`, exit 1, no translation unit | R5 |
| ST-2 | `clc --profile hosted-amiga` on the same program | valid C containing `cl_create_msg_port`, `cl_create_message`, `cl_put_msg`, `cl_wait_signals`, `cl_get_msg`, `cl_reply_msg` | R1-R4 |
| ST-3 | `clorraine run --profile hosted-amiga examples/amiga/exec-self-test.clo` | the five-line fixed transcript, exit 0 | R6 |
| ST-4 | Re-run ST-3 twice | identical output on both runs; ports and messages cleaned up | R6 |
| ST-5 | `clorraine run --profile hosted-amiga examples/amiga/hamurabi.clo` | the hand-checked ten-year transcript, exit 0 | R7 |
| ST-7 | `spec/appendix-stdlib.md` section against `stdlib.rs` exports | every exported name appears in the section | R8 |
| ST-8 | `make check` and `make verify` | green; the hosted-amiga legs run both programs when the toolchain is present | R8 |
| ST-9 | Run the drop-routing program: unsent `NewMessage` drop, explicit `freeMessage`, fetched `Message` drop, explicit `releaseMsg` | unsent messages are freed and never queued; fetched messages are replied by the drop; both explicit releases are clean | R1, R6 |

## Test homes

| File | Cases |
|---|---|
| `compiler/tests/amiga_exec_spec_test.rs` | ST-1-ST-5, ST-7, ST-9 |
| `driver`/`stdlib` unit tests | ST-2, ST-7 |
| `examples/amiga/*.expected.txt` | ST-3, ST-5 oracles |
| `spec/check.sh` + `make verify` | ST-8 |

## Fixtures

| Fixture | Purpose |
|---|---|
| `examples/amiga/exec-self-test.clo` | Wrapper coverage |
| `examples/amiga/message-drop.clo` | Drop-routing coverage (ST-9) |
| `examples/amiga/exec-self-test.expected.txt` | Fixed transcript |
| `examples/amiga/message-drop.expected.txt` | Drop-routing transcript |
| `examples/amiga/hamurabi.clo` | External port coverage |
| `examples/amiga/hamurabi.expected.txt` | Hand-checked transcript |

The hosted-amiga cases skip with a note when `m68k-amigaos-gcc` or `vamos` is absent, matching
the existing hosted-amiga suites. The host emission and rejection cases always run.
