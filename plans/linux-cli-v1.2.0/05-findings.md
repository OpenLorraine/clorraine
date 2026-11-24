# Findings ledger

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-001 | 1 | MAJOR | Native emulator | Fixed: native `--emulator` is explicitly unavailable in Phase 1; the old native staging path is retired, the marked SDK container keeps its handoff, and Phase 2 owns the verified runner. |
| RV-002 | 1 | MAJOR | Real toolchain gate | Fixed: `make toolchain-check` rebuilds the driver and runs the full `setup` validator (glibc, rc24 version, both probes) in a private config before building. |
| RV-003 | 1 | MAJOR | Test seam | Fixed: `resolve_from_stored` has marker/no-selection unit coverage; the container-skip integration case is no longer the only test. |
| RV-004 | 1 | MAJOR | Setup paths | Fixed: canonical roots and compilers are checked for UTF-8/control characters; compiler lines are sanitised; error messages carry the `clorraine:` prefix. |
| RV-005 | 1 | MAJOR | Test integrity | Fixed: added glibc 2.35, probe-failure-after-good-config, atomic write, no-fallback, empty-config, and config-log coverage. |
| RV-006 | 1 | MINOR | glibc parsing | Fixed: the minor component is parsed from dotted versions such as `2.37.1`. |
| RV-007 | 1 | MINOR | Probe pipe handling | Fixed: stdout/stderr are drained concurrently while a bounded watchdog waits. |
| RV-008 | 1 | MINOR | Error prefix | Fixed: setup errors are prefixed with `clorraine:`. |
| RV-009 | 1 | MINOR | Missing-tool hint | Fixed: `cc` and `vamos` get tool-specific guidance instead of setup guidance. |
| RV-010 | 1 | MINOR | Empty config | Fixed: an empty/unknown-only config is `None`, so the marked container can fall back; an empty value is still malformed. |
| RV-011 | 1 | MINOR | Directory containment | Fixed: required subdirectories are canonicalised and checked inside the root. |
| RV-012 | 1 | MINOR | Config permissions | Fixed: the Clorraine config directory is 0700 and the file is 0644 on Unix. |
| RV-013 | 1 | MINOR | Setup flags | Fixed: explicit `--profile` and `--diagnostics=json` are rejected by `setup`. |
| RV-014 | 1 | OBS | Exit-code docs | Fixed: the module doc names invalid/missing toolchain selection. |
| RV-015 | 1 | OBS | Review scope | Fixed: the expected set and refreshed diff include `run.rs`, retired files, the checker script, and `site/guide/known-limitations.md`. |
| RV-016 | 1 | OBS | Evidence | Fixed: Phase 1 red/green output, real setup, build sizes, and gate logs are recorded in the plan. |
| RV-017 | 1 | MINOR | Help/error wording | Fixed: help and the native-emulator error now describe the build limitation without phase language. |
| RV-018 | 1 | OBS | Scratch placement | Fixed: `toolchain-check` exports `TMPDIR`/`CODEOPS_TMPDIR` to `build/tmp`, and the script uses the same root. |
| RV-019 | 1 | OBS | Regression red evidence | Accepted with note: the initial public-path red run is recorded; the review-added failure tests (probe timeout, config-rename cleanup) are green and target the corrected failure paths. |

Re-review result: no CRITICAL or MAJOR findings. RV-017–RV-019 are fixed or accepted as notes above; RV-004R/RV-007R/RV-010R are resolved in the final diff.

## Phase 2 review (2026-11-24)

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| RV-020 | 2 | MINOR | Capture failure output | Fixed: a capture-write failure prints the readable guest result before the retained diagnostic; the spec asserts stdout. |
| RV-021 | 2 | MINOR | Staging coverage | Fixed: the staging test asserts the A1200 config, literal argv options, isolated environment, 0700 mode, and that no ROM enters the guest drive. |
| RV-022 | 2 | MINOR | Wrapper build failure | Fixed: `FAKE_COMPILER_FAIL` seam and a test assert exit 1, no emulator start, and no retained private run. |
| RV-023 | 2 | MINOR | Selection coverage | Fixed: environment selection, both no-fallback directions, the 3.2.0 boundary (`3.1.99`/`3.2.0`/`3.2.35-2`), and pure timeout bounds (1/90/3600/0/3601/abc) are tested. |
| RV-024 | 2 | OBS | Site selection wording | Fixed: the Amiga emulator page says "with a native toolchain selected". |
| SR-001 | 2 | MAJOR | Gate process leak | Fixed: `pgrep` is required, the check starts from a clean process set, and it asserts no FS-UAE/Xvfb survivor after every invocation, including failures and the negative. A dirty-start run exits 2; the real gate is green. |
| SR-002 | 2 | MINOR | Guest exit codes | Fixed: help and the CLI reference state that a verified guest failure exits 1 for every nonzero guest code; host/`vamos`/freestanding keep their own code. |
| SR-003 | 2 | MINOR | Capture wording | Fixed: help and docs say capture is success-only; a failed guest prints its output and retains the run. |

Open-question rulings:

- The staging spec now asserts the effective A1200 config and ROM option; the
  real A1200/KS3.1 boot remains the platform evidence.
- `make fs-uae-check` stays a local product check, like `make toolchain-check`;
  it needs a licensed ROM and a native toolchain, so it is not added to
  `make verify`.
- Failure evidence retention is intentional and documented: a failed check
  keeps its temporary run and prints the path; a successful check removes it.
