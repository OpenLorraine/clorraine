---
title: Diagnostics
description: "Reading Clorraine diagnostics, explaining a code, and consuming the JSON stream."
---

# Diagnostics

Every Clorraine diagnostic has a stable code. The registry records the trigger,
the section, and the help text. The compiler prints the code in both human and
JSON formats.

## Explain a code

```sh
clorraine --explain E10020
```

Output:

```text
error E10020: '{name}' was moved and cannot be used
  trigger: Read, move, borrow, or drop of a moved value
  section: Ownership, moves, initialization, nullability, borrows (E100xx)
  help: re-initialize the binding or use a view
```

An unknown code is a usage error with exit code 2.

## Human diagnostics

A normal compile failure prints a source location and help:

```text
error E10020: 's' was moved and cannot be used
  --> examples/ownership-fail.clo:12:13
  help: re-initialize the binding or use a view
```

The location is one-based for both line and column.

## JSON diagnostics

Use `--diagnostics=json` after the command:

```sh
clorraine build --diagnostics=json -p host examples/ownership-fail.clo
```

Each diagnostic is one JSON object on stderr:

```json
{
  "code": "E10020",
  "severity": "error",
  "message": "'s' was moved and cannot be used",
  "help": "re-initialize the binding or use a view",
  "file": "examples/ownership-fail.clo",
  "line": 12,
  "column": 13,
  "endLine": 12,
  "endColumn": 14
}
```

Fields that do not apply are `null`. Usage errors use the same shape with a
`null` code.

## Common codes

| Code | Meaning | Typical fix |
|---|---|---|
| `E10020` | Use of a moved value | Re-initialize the binding or use a view |
| `E10102` | Missing, ambiguous, or wrongly typed handle deleter | Declare one `freedBy` deleter with a `take` parameter |
| `E10300` | Wrong number of type arguments | Match the generic declaration |
| `E10301` | Cannot infer type arguments | Supply explicit type arguments |
| `E10400` | Capturing closure used as a function pointer | Invoke it inline or use a capture-less lambda |
| `E10404` | Capturing closure stored or returned | Keep it inline; it cannot escape |
| `E11200` | Construct not available on the selected profile | Change profile or remove the construct |
| `E11201` | Unknown profile name | Use `host`, `hosted-amiga`, `hosted-amiga-13`, or `freestanding` |
| `W10004` | Stack frame exceeds the profile threshold | Reduce the frame or accept the warning deliberately |

The complete registry is in
[`spec/14-diagnostics.md`](https://github.com/OpenLorraine/clorraine/blob/main/spec/14-diagnostics.md).
The machine-readable definitions live in the compiler's diagnostic registry.

## Severity and exit status

- An **error** fails the build with exit code 1.
- A **warning** does not fail the build unless `--deny-warnings` is set.
- A **usage error** exits with code 2.
- `run` returns the program's exit code after a successful build, so check the
  build output separately when a run exits nonzero.

## Reporting a diagnostic problem

If a diagnostic is wrong, misleading, or missing a source location, open an
issue with:

- The code and the exact message.
- The smallest source that triggers it.
- The command and profile.
- What you expected instead.

A diagnostic is part of the language surface. A confusing message is a bug worth
fixing.
