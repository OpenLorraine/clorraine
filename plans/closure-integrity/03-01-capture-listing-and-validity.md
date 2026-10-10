# Capture Listing and Validity: Closure Integrity

> **Document**: 03-01-capture-listing-and-validity.md
> **Parent**: [Index](00-index.md)

## Overview

Phase 1 closes the capture-list validity gaps: a nested lambda's relayed entry (DEF-92), a
module-level name in a list (DEF-93, with the new `E10406`), and the dotted capture target that
misses the write rule (DEF-94). All three changes live in the capture validation and the place
root — no lowering change is needed: the accepted spellings already lift correctly.

## Architecture

### Current Architecture

- `check_lambda` (`compiler/src/sema/expr.rs:140-174`) validates each capture entry; the entry's
  name resolves through `name_type` (`:142`) with no floor check, and the entry is pushed into the
  lambda's scope so the body read passes. The direct-read check for unlisted names (`:101-117`,
  E10405) compares a read's scope depth against `lambda_floors` (`sema/mod.rs:369-381`).
- `lower_captures` (`compiler/src/own/lower.rs:365-397`) resolves each capture through the
  function's bindings; an unresolvable entry is skipped (`:372-374`) while the sema site still
  reports captures, so the lifted function expects an environment its call site does not pass.
- `place_root_name` (`compiler/src/sema/mod.rs:603-614`) returns a root only for a one-segment
  `Qualified` (`:610`); `p.x` parses as a multi-segment `Qualified` (`parse/expr.rs:439-450`), so
  the E10402 rule (`sema/mod.rs:397-414`) and the E10401 rule (`sema/expr.rs:256-276`) never see
  the dotted root, and the write falls through to the ownership pass (`own/place.rs:112-119`),
  which reports E10024.

### Proposed Changes

1. **Capture-entry validity (DEF-92, DEF-93).** In the entry validation:
   - an entry that names a module-level name — a `let`, a `const`, an imported name, a function,
     a type, or any other module declaration — reports the new `E10406` (`module-level '{name}'
     needs no capture`, help `use the name directly`) (AR-2, AR-8);
   - an entry inside a nested lambda whose name lies below the enclosing lambda's scope floor and
     is not carried by that lambda's environment reports `E10405` (`lambda reads '{name}' without
     listing it`, help `add '{name}' to the capture list`) at the entry, because the name travels
     through the enclosing lambda (AR-4, AR-9);
   - every other entry stays accepted, including the top-level lambda's function-local entries and
     the `([n, m] () => ([m] () => m)())()` relay whose outer list carries the name.
2. **Dotted roots (DEF-94).** `place_root_name` returns the first segment for any `Qualified`
   (`sema/mod.rs:610`), so `p.x` roots at `p`; the two call sites' capture-membership tests
   (`sema/mod.rs:400`, `sema/expr.rs:262`) then apply E10402/E10401 to dotted targets. A
   module-path spelling roots at its first segment too, which no capture list can name (module
   entries are E10406), so the comparison is safe.

### Check order

The entry checks run before the entry is declared into the lambda's scope, so a rejected entry
changes no state and the body's reads are judged against the surviving list. Each rejected entry
reports exactly once — the E10406 test precedes the E10405 reachability test, since a module-level
name is below every floor.

## Implementation Details

### New Functions/Methods

The entry validation gains a small helper beside `check_lambda`:

```rust
/// Reports a capture entry the enclosing scope cannot carry.
///
/// A module-level binding needs no capture: the name is directly visible
/// inside the lambda, so listing it is meaningless. A nested lambda's entry
/// for a name below the enclosing lambda's floor travels through that
/// lambda's environment, so the enclosing list must carry it; the report
/// names the entry and the fix is to add the name to the enclosing lambda.
/// Returns `true` when the entry was reported, so the caller skips it.
fn check_capture_entry(&mut self, entry: &CaptureInfo) -> bool
```

### Integration Points

| Component | Interaction |
| --------- | ----------- |
| `sema/expr.rs` | the entry loop in `check_lambda`; the E10405 direct-read check is unchanged |
| `sema/mod.rs` | `place_root_name`; the `name_scope_depth`/scope facts the helper reads |
| `spec/14-diagnostics.md` | the authorized `E10406` row (AR-2) |
| `spec/tests/compile-fail/` | the `E10406` golden (AR-12) |
| `compiler/tests/compile_fail_spec_test.rs` | the asserted-code registration (AR-12) |
| the lowering | unchanged: an accepted relay already lifts (`own/lower.rs:365-397`) |

## Code Examples

```clorraine
let n: i32 = 2;
let m: i32 = 3;
([n, m] () => ([m] () => m)())();     // accepted: the outer list carries m

([n] () => ([m] () => m)())();        // E10405: add 'm' to the outer list

const base: i32 = 7;
() => base;                            // accepted: module names need no capture
[base] () => base;                     // E10406: module-level 'base' needs no capture

let p: P = P { x: 1 };
([p] () => { p.x = 2; })();           // E10402: cannot write captured 'p' without '[borrow p]'
([borrow p] () => { p.x = 2; })();    // accepted: writes through to the owner
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Nested relay without the enclosing list | one `E10405` at the entry, naming the entry's name | AR-4, AR-9 |
| Module-level name in a list | one `E10406`, naming the binding | AR-2, AR-8 |
| Dotted write through a copy capture | one `E10402`, naming the root | AR-6 |
| Dotted write through a `[borrow]` capture | accepted (the write reaches the owner) | AR-6 |
| Copy-capture read | accepted (unchanged) | AR-6 |

> **Traceability:** every error-handling strategy back-references its register entry. See
> [00-ambiguity-register.md](00-ambiguity-register.md).

## Testing Requirements

- Specification cases ST-1 – ST-9 ([07-testing-strategy.md](07-testing-strategy.md)).
- The `E10406` golden runs through `./spec/check.sh` (part of `make check`) and the compile-fail
  driver.
- Implementation pins: the report count and span per shape, the deep-relay (three-level) single
  report, the dotted-root resolution, and byte-identical re-emission of the accepted relay.
