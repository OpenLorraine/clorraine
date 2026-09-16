# Chapter 14 - Diagnostics Registry

> **Status:** draft
> **Depends on:** Ch 03 (ownership), Ch 12 (contracts), Ch 16 (borrow rules)
> **Process inherited from:** Blend65 Ch 14 (same code scheme and controls)

## 1. Registry rules

* Every diagnostic has a stable code: `E1xxxx` errors, `W1xxxx` warnings.
  Codes are never reused after retirement.
* This chapter is the sole authority for message templates; other chapters
  reference codes but do not define text.
* A diagnostic carries: severity and code, message, primary span, related
  spans, optional help.
* **Poisoning:** an error poisons the affected node; dependent errors are
  suppressed so one mistake does not produce a cascade. Independent causes are
  still reported.
* Controls: `--warn-as-error`, `--suppress-warning=W...`, `--max-errors=N`.

## 2. Code blocks

| Range | Area |
|---|---|
| E10000-E10099 | Ownership, moves, drops |
| E10100-E10199 | FFI contracts and the raw boundary |
| E10200-E10299 | Borrow checker (loans, origins, liveness, conflicts) |
| E11000-E11099 | Types, expressions, statements |
| E11100-E11199 | Modules and declarations |
| E11200-E11299 | Codegen and target limits |
| W10000-W10099 | Lint-class warnings |
| W10100-W10199 | Contract and boundary advisories |

## 3. Ownership and drop diagnostics

| Code | Trigger | Message template (short) | Help |
|---|---|---|---|
| E10010 | Possibly-null owned value used without a check | `value '{name}' may be null here` | check for the failure handle first |
| E10020 | Read/borrow/drop of a moved value | `'{name}' was moved and cannot be used` | introduce a new binding or reborrow before moving |
| E10021 | Binding used before initialization | `'{name}' is not initialized on this path` | initialize on every path before use |
| E10022 | Move while borrowed | `cannot move '{name}' while it is borrowed` | end the borrow's scope first |
| E10023 | Assignment to an immutable binding | `cannot assign to immutable '{name}'` | declare it mutable |
| E10024 | Impossible drop: value already consumed by a `consumes` call | `'{name}' was consumed and cannot be dropped again` | remove the redundant release |
| E10030 | Borrow outlives its owner | `borrow of '{name}' does not live long enough` | move the use before the owner's scope ends |
| E10031 | Owner moved or exclusively borrowed while shared borrow is live | `cannot modify '{name}' while it is shared-borrowed` | end the shared borrow first |
| E10032 | Second conflicting borrow (exclusive vs any) | `'{name}' is already exclusively borrowed` | end the first borrow first |
| E10033 | Borrow escapes its scope | `borrow of '{name}' escapes its scope` | pass an owned value, or restructure |
| E10034 | Borrow parameter released or stored by the callee | `cannot release or store borrowed parameter '{name}'` | take it as `own` instead |
| E10035 | Returning a borrow of a local | `cannot return borrow of local '{name}'` | return an owned value |
| E10036 | `borrow` field in an aggregate (v1) | `borrow fields are not supported yet` | hold the owner instead |
| E10037 | `own` global | `owned globals are not supported` | release before the program returns, or use a module function |
| E10038 | Global borrow | `global borrows are not supported` | borrow from a local owner |

## 4. FFI and raw-boundary diagnostics

| Code | Trigger | Message template (short) | Help |
|---|---|---|---|
| E10100 | `own`/`borrow` value passed to a function without a suitable contract | `'{fn}' has no contract for parameter '{name}'` | add a contract or pass a raw pointer from an escape block |
| E10101 | Contract mismatch: caller mode not allowed | `'{fn}' expects {expected} for '{name}'` | adjust the argument's mode |
| E10102 | Raw operation outside an escape block | `raw operation requires an escape block` | wrap it in the audited block |
| E10103 | Missing deleter for an owned result | `no release function known for '{type}'` | extend the contract |
| W10110 | User-provided contract file used | `using user contract '{name}'` | review or vendor the contract |

## 5. Borrow-checker diagnostics

| Code | Trigger | Message template (short) | Help |
|---|---|---|---|
| E10200 | Loan conflicts with another access at the same point | `'{name}' is accessed while borrowed` | end the borrow before this access |
| E10201 | Loan required to be live but owner was released | `use of '{name}' after its release point` | extend the owner's scope |
| E10202 | Region constraint unsatisfiable (v2 non-lexical) | `borrow of '{name}' cannot satisfy the required lifetime` | restructure the borrow |

## 6. Warnings

| Code | Trigger | Message template (short) |
|---|---|---|
| W10010 | Owned value acquired and never used | `owned '{name}' is never used` |
| W10011 | Unreachable cleanup path | `cleanup code is unreachable` |
| W10012 | Borrow taken but never used | `borrow '{name}' is never used` |

## 7. Migration and stability

* New codes are added with the feature that needs them; templates live here
  from the first release.
* Golden compile-fail tests (Ch 16 §6) assert the code and the first line of
  the message; message wording may still change while the chapter is `draft`.
