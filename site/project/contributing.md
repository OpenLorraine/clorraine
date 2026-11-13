---
title: Contributing
description: "How to build Clorraine, run the gates, report a useful bug, and help with platform support."
---

# Contributing

Clorraine is developed in the open. Contributions, bug reports, and independent
testing are welcome.

## Report a bug

A useful report includes:

- The exact command and profile.
- `clorraine --version`.
- The smallest complete `.clo` source that reproduces the problem.
- The generated C (`clorraine emit ... -o build/repro.c`) for a code-generation
  problem.
- The expected and actual output.
- The host OS and, for Amiga problems, the emulator and Kickstart version.

Open issues at
[github.com/OpenLorraine/clorraine/issues](https://github.com/OpenLorraine/clorraine/issues).

## Build and test

Build the workspace:

```sh
cargo build --workspace
```

Run the fast gate:

```sh
make check
```

Run the full acceptance gate on a machine with the SDK, X display, and licensed
media:

```sh
make verify
```

The repository's [AGENTS.md](https://github.com/OpenLorraine/clorraine/blob/main/AGENTS.md)
lists every command and its purpose.

## Test discipline

The test suite has two kinds of tests:

| Kind | Naming | Rule |
|---|---|---|
| Specification test | `<feature>_spec_test.rs` | Derives from the specification or approved requirements; it is the oracle |
| Implementation test | `<feature>_impl_test.rs` | Covers internals and pinned emission shapes |

A specification test is not changed to make an implementation pass. If the
specification is wrong, that needs an explicit ruling and a specification change,
not a quiet test edit.

## What to work on

Good places to help:

- **Platform testing.** macOS and Windows hosts, different Linux distributions,
  and real Amiga hardware.
- **Installer and packaging.** The Linux tarball exists; Windows, macOS, and
  editor distribution do not.
- **Documentation.** Correct a confusing page, add a tested example, or improve
  an error message.
- **Amiga coverage.** Pick a tracked capability, add a gate, and prove it on the
  emulator or target.
- **Compiler edge cases.** The accepted limitations list has workarounds; a
  clean fix with a regression test is valuable.

## Code style

- Keep the change focused and consistent with the surrounding module.
- Document public behaviour and non-obvious invariants.
- Add a test that fails without the fix.
- Do not reference internal planning identifiers in source or doc comments;
  restate the rationale in plain language.

## License

The repository uses the Apache License 2.0 with the Clorraine Output Exception.
By contributing, you agree that your contribution is distributed under the
repository's license. Third-party code must keep its own notice and be recorded
in `THIRD_PARTY_NOTICES.md`.

## Questions

Use [GitHub Issues](https://github.com/OpenLorraine/clorraine/issues) for
technical questions. For licensing enquiries, see the
[project brief](/project/brief#contact).
