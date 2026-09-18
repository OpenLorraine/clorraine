# Compile-Fail Suite

One `.clo` source per diagnostic code registered in
[Ch 14](../../14-diagnostics.md). The compiler does not exist yet; these files
are the golden expectations for the first semantic stages of the Phase 2
compiler.

## Conventions

* The first line is the expectation:
  * `// expect-error CODE` - the compiler must report that error code.
  * `// expect-warning CODE` - the warning must be reported (and becomes
    fatal under `--deny-warnings`).
* Optional annotation lines follow, before the `module` declaration:
  * `// trigger: ...` - the intended trigger in one line.
  * `// cli: ...` - non-default harness flags.
  * `// profile: NAME` - the profile the test needs.
  * `// stage: N` - the lambda or `comptime` stage that activates the code.
  * `// requires: ...` - anything a single-file harness cannot provide.
* The module is `module tests.compile_fail.<CODE>;` and the filename is
  `<CODE>_<slug>.clo`.
* The program is valid Clorraine except for the single intended error
  (syntax tests are exempt). Parameter modes precede names, `.` is the only
  member operator, and assignment is never used as a value.

## Coverage

[`check.sh`](../../check.sh) enforces:

* every annotation code exists in the [registry](../../14-diagnostics.md);
* every registered compile-time code has at least one test.

Excluded from the per-code rule: `E11210`-`E11213` (runtime faults, which are
run-test cases under [`tests/run/`](../run/README.md)), `E90000` (the generic
staged-feature fallback with no unique trigger), and `E90003`, `E90004`, and
`W10005` (staged under [`tests/staged/`](../staged/README.md) until their
feature or harness exists). See `docs/findings-phase1.md` for the Phase D
report.

> **Note:** a `// expect-warning` test asserts the warning is reported in the
> default build. Under `--deny-warnings` every warning becomes fatal, so the
> same file then exits nonzero. Warning tests and their escalation are covered
> by the warning-escalation case planned for the run suite.
