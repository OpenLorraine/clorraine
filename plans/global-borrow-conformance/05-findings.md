# Findings

## Phase 1 — Uniform BR-9 (`DEF-533`)

**Status**: Done (2026-10-21).
**Ruling**: frozen BR-9 wins over the conflicting Rust spec pins.

The body and initializer now report `E10038` for every non-`const` global
borrow: a module-qualified receiver, an unmarked scalar or view argument, and a
view builder's source. The initializer callee resolution keeps the body's
scope order. The shared reporter keeps one message and one site.

The conflicting oracles and emission pins were retargeted to `const`, local, or
call-result sources. New spec cases pin the qualified body/initializer
rejection, the `slice` source rejection, and the unmarked scalar/view
rejection; the `const` qualified receiver and the local/call-result controls
stay accepted.

**Verification**: `make check` and `make verify` exit 0 at the phase head; the
focused suites pass (checker 210, module-initializer 65, eval-emission 53 + 68,
store-emission 14 + 32).

## Phase 2 — Func-typed initializer calls (`DEF-537`)

**Status**: Pending.

## Phase 3 — Generic element move (`DEF-425`)

**Status**: Pending.
