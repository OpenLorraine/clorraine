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

**Status**: Done (2026-10-21).
**Commit**: `167ebe6`.

A func-typed global or field callee now resolves its own `TyKind::Func`
signature. The field call is resolved before the free-function parameter
lookup, so `h.cb(s)` is judged by `cb`'s signature. Each written argument is
judged against the declared mode; a non-`const` global to an unmarked
parameter reports `E10038`, and a `const` global stays accepted.

**Verification**: `should_apply_the_borrow_rule_to_a_func_typed_initializer_callee`
(func global, func field, const control); `make check` exits 0.

## Phase 3 — Generic element move (`DEF-425`)

**Status**: Done (2026-10-21).
**Commit**: `f6d51e5`.

An opaque member move out of a borrow parameter records the per-instance
not-lend duty; an owning substitution reports `E10034`, a duplicable one stays
a copy. A deeper projection out of a `take` parameter records the per-instance
element-move duty: an owning substitution reports `E10026`, a duplicable one
copies, and a whole value or top-level field remains a legal take move. The
self-slot assignment skips the source move, so the identical self-copy stays
accepted; the distinct-index self-slot emission pin is removed because that
generic shape is now rejected.

**Verification**: the runtime-index spec case now rejects the member and whole
generic moves with `E10034`, rejects the take twin with `E10026`, and accepts
the duplicable take and identical self-copy controls; the dedupe and span pins
remain green. `make check` and `make verify` exit 0 at the batch head.
