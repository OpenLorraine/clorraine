# Requirements — Evaluation and Initializer Residue (Batch T)

These criteria derive from the row descriptions; the frozen `spec/` remains the
oracle.

## `DEF-529` — a run-time-index view store keeps its source place

| ID | Requirement |
|----|-------------|
| T-R1 | A generic run-time-index store whose element type is a type parameter and whose source is an owned place lowers the source as a place. When the instance makes the element a view, the store widens the source in place: the C has no transfer temporary and no `source = NULL`, and the caller's owner stays usable. |
| T-R2 | The same store at an owned instance still moves the value in the emitter, so the destination owns it and the source is emptied. |
| T-R3 | A constant-index store keeps its existing clean emission. A pure-place source and a call-result source keep their existing ownership behavior. |
| T-R4 | The accepted view-instance program runs under host AddressSanitizer with leak detection: no leak, double free, or use-after-free, and the caller's source remains readable after the call. |

## `DEF-531` — a non-call effectful `len` operand is evaluated

| ID | Requirement |
|----|-------------|
| T-R5 | A free, bare, or dot `len` of a fixed-array operand that is a conditional, an array literal, or a run-time-index read evaluates the operand's effects at the expression's position, then folds the extent. A conditional runs only the selected arm; an array literal runs each element expression once; a run-time index runs its index expression once. A conditional that mixes a caller place arm with a fresh arm evaluates the selected fresh arm and leaves the place arm in place. |
| T-R6 | A pure fixed-array place still folds with no read; the C unused marker remains so the strict build has no `unused-but-set-variable` warning. A direct call operand keeps the guard materialization, so a conditional, short-circuit, or loop-condition call runs only on the reached path. |
| T-R7 | When the `len` call itself sits in a conditionally evaluated position (an outer conditional arm, a short-circuit right side, or a loop condition), a non-call effectful operand must not run on a path that skips the call. If the shape cannot be filled path-aware, it keeps the structural fold and is recorded as a narrower residual rather than running unconditionally. |
| T-R8 | The accepted programs run and return the expected value under the host strict build and AddressSanitizer. |
| T-R12 | The bare `len(x)` and dot `x.len()` spellings share the free spelling's evaluation rules; a caller-owned place used as an arm or element is not moved by the structural fold. |

## `DEF-532` — module initializers apply the own call rules

| ID | Requirement |
|----|-------------|
| T-R9 | A call inside a module initializer applies the ownership argument modes exactly as the same call in a function body: a global passed to a `borrow`/unmarked parameter reports `E10038`; a global receiver follows the receiver rule. |
| T-R10 | A module initializer call with only caller-safe sources stays accepted and runs. A module-level `const` source stays accepted. |
| T-R11 | The initializer diagnostic reports once per site, at the written call, and produces no translation unit. |
| T-R13 | The initializer walk resolves the callee by the body's scope order (current module, import binding, program map) and resolves parameter types in the declaring module. A func-typed field call is not judged against a same-named free function, including a projected or indexed receiver. |
| T-R14 | An explicit `borrow`/`const` on a generic dot callee reports `E10038`; an unmarked parameter whose declared type is an unresolved type parameter keeps the existing initializer oracle. |

## General

| ID | Requirement |
|----|-------------|
| T-G1 | No `spec/` file, grammar rule, diagnostic registry entry, or generated C is edited. Existing specification tests remain the oracle. |
| T-G2 | Every phase runs the focused suites and `make check`; the batch close-out runs `make verify`. |
| T-G3 | New specification tests pin the accepted behavior and the rejection; implementation tests pin the emitted shape where the shape is the contract. |
