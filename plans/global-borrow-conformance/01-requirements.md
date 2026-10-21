# Requirements — Global Borrow and Move Conformance (Batch U)

## `DEF-533` — BR-9 applies uniformly

| ID | Requirement |
|----|-------------|
| U-R1 | A non-`const` global passed to a `borrow`, `const`, or unmarked parameter reports `E10038` in a module initializer and in a function body, including a module-qualified receiver. |
| U-R2 | A view builder (`slice`, `bytes`, `toCString`, `rawSlice`, `rawStr`) with a non-`const` global source reports `E10038`; a call-result, local, or `const` source stays accepted. |
| U-R3 | A `const` global, a module-qualified function (not a receiver), and a `take` parameter stay accepted. |
| U-R4 | Existing Rust spec oracles that conflict with BR-9 are retargeted to `const`, local, or call-result sources; `spec/` is not edited. |

## `DEF-537` — func-typed initializer calls

| ID | Requirement |
|----|-------------|
| U-R5 | An initializer call through a func-typed global or func-typed field judges each written argument against that signature; a non-`const` global argument to an unmarked parameter reports `E10038`. |
| U-R6 | A `take` parameter, a `const` global, and a non-place argument stay accepted. |

## `DEF-425` — generic element move through a borrow

| ID | Requirement |
|----|-------------|
| U-R7 | A generic body that moves an owning element out of a `borrow` parameter reports `E10034` at the element read, matching the concrete `borrow` twin. |
| U-R8 | The `take` twin reports `E10026`; the duplicable and local controls stay accepted. |

## General

| ID | Requirement |
|----|-------------|
| U-G1 | No `spec/` file, grammar rule, diagnostic registry entry, or generated C is edited. |
| U-G2 | Each phase runs the focused suites, `make check`, and the final `make verify`. |
| U-G3 | New behavior is pinned by a spec test; emitted shape or count is pinned by an implementation test. |
