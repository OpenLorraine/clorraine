# Run Suite

Phase 2 run tests for the always-on safety checks. The compiler does not exist
yet; these files are the golden expectations for the runtime stage.

## How a run test works

1. **Compile** the file. It must compile with no diagnostics.
2. **Execute** the built program.
3. **Expect a fault.** The program must print the first line of its expected
   fault to the diagnostic stream, then abort. It must not run the cleanup
   ladder or return normally.

## Conventions

* The first line is the expectation: `// expect-fault CODE` — the program must
  fault with that code at run time.
* Optional annotation lines follow, before the `module` declaration:
  * `// trigger: ...` — the intended fault in one line.
* The module is `module tests.run.<CODE>;` and the filename is
  `<CODE>_<slug>.clor`.
* The program is valid Clorraine; it compiles cleanly and faults only when run.

## Cases

| File | Code | Fault |
|---|---|---|
| `E11210_division_by_zero.clor` | E11210 | integer division by zero |
| `E11210_min_neg_one.clor` | E11210 | integer division by `-1` at the minimum signed value |
| `E11211_index_out_of_range.clor` | E11211 | index out of range |
| `E11212_float_to_int_range.clor` | E11212 | float-to-integer conversion out of range |
| `E11212_nan_float_to_int.clor` | E11212 | float-to-integer conversion of NaN |
| `E11213_assertion_failed.clor` | E11213 | assertion failed |

These four codes are excluded from the compile-fail per-code coverage rule in
[`check.sh`](../../check.sh) because they are runtime faults, not compile errors.
