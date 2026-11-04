# Amiga Library Surface: PLAN-13 Acceptance Slice

> **Status**: Done (16/16)
> **Created**: 2026-11-03
> **Implements**: PLAN-13 bounded slice (AR-8, AR-13)
> **CodeOps Artifact Schema**: 1

## Overview

This plan delivers the v1.0 `std.amiga.exec` wrapper and proves it with two console
programs on the hosted-amiga profile:

1. **Hamurabi** — an external MIT-licensed C99 console game, ported to Clorraine.
2. **`exec-self-test`** — a first-party MIT console program that exercises the message-port
   and signal surface directly.

The slice is deliberately bounded. It does not add language features, a general Amiga library
bridge, or a new profile. It fixes implementation, porting, and FFI gaps the two programs expose.
A gap that needs a language-surface change stops for a user ruling.

## Scope decisions

| Decision | Outcome |
|---|---|
| Wrapper surface | `MsgPort`, `Message`, create/delete port, create/free message, payload access, `putMsg`, `signals`, `waitSignals`, `getMsg`, `replyMsg` |
| Profiles | hosted-amiga only; the host and freestanding profiles reject the module (`E11200`) |
| Program 1 | Hamurabi, MIT, `michaelcmartin/bumbershoot/full/hamurabi.c` |
| Program 2 | `exec-self-test`, first-party MIT, written for this slice |
| Language | unchanged; no new syntax, ABI rule, or diagnostic |
| Verification | hosted-amiga build/run against hand-checked expected output; `make check`; `make verify`; appendices re-reviewed |

## Documents

| Document | Contents |
|---|---|
| [01-requirements.md](01-requirements.md) | R1-R8 and acceptance criteria |
| [02-current-state.md](02-current-state.md) | Existing modules, shim surface, and gaps |
| [03-design.md](03-design.md) | Wrapper API, shim functions, ownership, and program designs |
| [05-findings.md](05-findings.md) | Red phases, reviews, and accepted residuals |
| [07-testing-strategy.md](07-testing-strategy.md) | ST-1-ST-8 |
| [99-execution-plan.md](99-execution-plan.md) | 16 tasks across three phases |

## Minimum-sufficient baseline

Add one stdlib module and the shim calls it needs; port one external program and write one
small first-party program. No dependency, no new harness, and no compiler-language change.
