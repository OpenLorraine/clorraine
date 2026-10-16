# Module/Global Identity and the Const Folder: Residual Major Closeout II

> **Document**: 03-08-module-global-identity-and-folds.md
> **Parent**: [Index](00-index.md)

## Overview

Three rows close the batch-K phase-6 residue: the func-global resolution's aliased-import and
qualified value-head gaps, the const folder's ABI-unfaithful extent folds, and the checker's
simple-name typing of a bare func-global call. The identity threading and the checker typing are
conformance to M10/MOD-13 (the scope's binding governs); the const fold defers to the C type
size (AR-13). The self-qualified and plain spellings are green controls (fixed by the batch-K
K6C-1 correction).

## Design

### 1. The aliased import and the qualified value head (`DEF-356`)

`import { g as bg } from def356holder; bg(5)` emits `cl_def356holder_bg(...)` — the *alias* as
the symbol — and the strict build fails (the recorded probe, `alias_fn.clo`; the k18 evidence
recorded the same "rejected" state post-correction). Separately, a cross-module qualified value
head `q.holder.g(5)` (a struct global's field through its declaring module) is rejected
`E10219`. The plain import (`holder.g(5)`) and the self-qualified `b.holder.g(5)` with a
current-module `let g` build and run 0 (fixed by the batch-K phase-6 corrections; the k18
evidence's `fnself`/`pq4`/`amb` matrix).

Thread the binding identity through the aliased and qualified value-head paths: an aliased
import resolves the *source* name's binding (its module symbol), and a qualified value-head path
resolves the leading segments as the declaring module (or the head as the global binding, per
the existing `global_binding_path` policy) so the field call spells the holder's symbol. The
identity/dotted-path policy the batch-K K6C-1 correction settled (a loaded-module path head is
not a name mention; the qualified callee resolves directly) is the model; the controls must stay
byte-identical.

### 2. The extent-position `sizeof` fold (`DEF-357`)

`const n: usize = sizeof(S); let pad: u8[n] = [; 0];` emits `u8 pad[5]` for the host layout's 8
(`ClS` has padding) and runs 253; the `str` twin emits `u8 buf[8]` against the host's 16 and runs
248. The *value* positions are already green controls: they emit `sizeof(ClS)`/`sizeof(ClStr)`
(runs 0). The folder (`consts/mod.rs:495`) folds `sizeof` through `table.size_of` (the unpadded
declared layout) when the constant is consumed as an extent.

Defer the extent-position fold to the C type size (AR-13): the extent emits `sizeof(ClS)` (or the
declared C type spelling) exactly as the value position does, so the host padding and the view
size are honored on every profile. The scalar/struct value folds and `offsetof` stay
behavior-pinned; the green value controls stay byte-identical. Stop condition: if a C extent
cannot take the deferred spelling on every profile, the phase parks the row with the probe
evidence.

### 3. The scope-aware checker typing (`DEF-359`)

`f("x")` against an imported `func(i32)` global is accepted while a same-named `func(str)`
exists elsewhere (`k18/sig/`); the checker types the callee through the program-wide simple-name
map, and only the emission fails (`incompatible type for argument 1 of 'cl_sigcholder_f'`). Per
M10/MOD-13 the scope's binding (the caller's module or its imports) governs.

Type the bare func-global call through the scope's binding: the checker resolves the same
identity the emitter threads, so the argument conversion reports the frozen `E10200` at the
mismatched call instead of emitting invalid C. The matching call (`f(5)`) and the qualified
spelling are the controls.

## Integration Points

- `compiler/src/codegen/expr.rs` (`global_binding_path` :481, `func_binding_spelling`),
  `compiler/src/own/call.rs` (`func_global_callee_path` :454, `function_in_scope` :486),
  `compiler/src/load/` (the import binding/export check, the mention policy).
- `compiler/src/consts/mod.rs:495` (`fold_intrinsic`), the `sizeof` consumer at the extent
  position, `compiler/src/types/table.rs` (`size_of`).
- `compiler/src/sema/call.rs` (the bare-callee typing), `compiler/src/sema/mod.rs`
  (`callee_type`).
- Controls: the `k18` matrix (fnself, fnself2/3/4, amb1, pdiff, pq4, pimp), the batch-K
  phase-6 pins, the `def357` value folds.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| An ambiguous qualified head (several same-named bindings, no module spelled) | The existing unresolved policy (decline rather than pick) | — |
| An unexported imported name | The existing `E11102` (the probe's `f` *is* exported; the alias resolves the source name) | — |
| A deferred extent on a profile whose C type spelling differs | The profile's declared type spelling; a park if it cannot be expressed | AR-13 |
| A redirect that changes an established spelling's symbol | The `k18` matrix and the emission differential gate it | — |

## Testing Requirements

- Spec: the aliased import (builds; the holder's symbol; runs), the qualified value head
  (builds; runs), the controls (plain/self-qualified/amb/pq4), the extent folds (the host values
  read back; both profiles), and the scope-aware typing (one `E10200` at the mismatched call;
  the matching and qualified controls).
- Impl: the emitted alias/binding symbols (the alias spelling absent), the deferred extent
  spelling (`sizeof(ClS)` in the extent), the checker typing record; the controls byte-identical;
  a re-emission determinism pin.
