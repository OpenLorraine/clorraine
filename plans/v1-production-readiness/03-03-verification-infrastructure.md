# Verification Infrastructure: Clorraine v1.0 Production Readiness

> **Document**: 03-03-verification-infrastructure.md
> **Parent**: [Index](00-index.md)

## Overview

The production gate needs two signals that are currently opt-in or absent: a sanitizer run over
every memory-safety suite, and a push-time host check. It also gets a convenience differential
gate over the same recorded outputs the corpus already pins. The approved scope is deliberately
narrow (AR-2): no full program generator, no scheduled self-hosted `make verify`, and no release
automation.

## Architecture

### Current Architecture

- `Makefile` defines `check`, `verify`, `corpus-amiga`, `corpus-fs-uae`, `run-fault-amiga`, and
  the demo targets. `check` runs `./spec/check.sh` plus `cargo test --workspace` in the SDK
  container; `verify` runs `check`, `corpus-fs-uae`, and `run-fault-amiga`.
- `docker/scripts/corpus-amiga.sh` builds the portable corpus for hosted-amiga, runs it under
  `vamos`, and diffs the captured stdout against the recorded output. It does not run the host
  profile or compare host and m68k output.
- 21 files under `compiler/tests/` reference `RDC_ASAN`; their memory cases run under
  AddressSanitizer only when the variable is set. The accepted environment flake is a parallel
  child-run hang inside a suite, so one suite per process is not enough by itself.
- There is no `.github/workflows/` directory.

### Proposed Changes

1. **`make asan`.** Add `docker/scripts/asan-suite.sh` and a Makefile target. The script discovers
   every `compiler/tests/*.rs` file that references `RDC_ASAN`, derives the test target name, and
   runs `RDC_ASAN=1 cargo test --test <name> -- --test-threads=1` with a bounded per-suite timeout.
   One suite per process limits blast radius; `--test-threads=1` removes the intra-suite parallel
   child runs that cause the accepted LSan flake. The target runs in the SDK container.
2. **`make differential`.** Add `docker/scripts/differential-seed.sh` and a Makefile target. For
   each of `buffer-container`, `file-copier`, and `string-parser`:
   - emit the host profile, build with the host `cc`, run with a private working directory, and
     capture stdout;
   - emit the hosted-amiga profile, build with the existing `m68k-amigaos-gcc` flags, run under
     `vamos`, and capture stdout;
   - diff the two outputs and fail on any difference.
   The script reuses the compile flags and `vamos` invocation from `corpus-amiga.sh`, builds
   `clc` first, guards a missing compiler, cleans its work directory, and supports `DIFF_ONLY=1`
   for a negative control. The target runs in the SDK container and joins `make verify` after
   `check` and before `corpus-fs-uae`. It is a convenience gate over the same recorded outputs,
   not a new independent signal.
3. **Host-only CI.** Add `.github/workflows/check.yml`:
   - trigger on `push` and `pull_request`;
   - `runs-on: ubuntu-latest`, `timeout-minutes: 30`;
   - least privilege (`permissions: contents: read`), a SHA-pinned checkout, and
     `persist-credentials: false`;
   - checkout, install stable Rust with `rustup`, then run `./spec/check.sh` and
     `cargo test --workspace`;
   - no secrets, no SDK image, no emulator; m68k-only tests skip themselves.
4. **Documentation.** Update `AGENTS.md` with the new targets, the workflow, and the host-only CI
   limitation.

## Implementation Details

### `docker/scripts/asan-suite.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

files="$(grep -l 'RDC_ASAN' compiler/tests/*.rs | sort || true)"
if [ -z "${files}" ]; then
    echo "asan: no RDC_ASAN suites discovered" >&2
    exit 2
fi

count=0
status=0
while IFS= read -r file; do
    suite="$(basename "${file}" .rs)"
    count=$((count + 1))
    echo "asan: ${suite}"
    if ! timeout 900 env RDC_ASAN=1 cargo test --test "${suite}" -- --test-threads=1; then
        echo "asan: ${suite}: FAILED" >&2
        status=1
    fi
done <<< "${files}"

echo "asan: ${count} suites"
exit "${status}"
```

The script runs from the repository root inside the SDK container. The 900-second bound is a
starting point; the executor raises it only with a recorded reason. `--test-threads=1` is required:
the accepted flake is inside a suite, not only across suites.

### `docker/scripts/differential-seed.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

CLC="${CLC:-./target/debug/clc}"
PROGRAMS=(buffer-container file-copier string-parser)
OUTDIR="${OUTDIR:-build/differential}"

if [ "${DIFF_ONLY:-0}" != "1" ]; then
    cargo build --workspace --quiet
fi

if [ ! -x "${CLC}" ]; then
    echo "differential: compiler not found: ${CLC}" >&2
    exit 2
fi

for name in "${PROGRAMS[@]}"; do
    workdir="${OUTDIR}/${name}"
    if [ "${DIFF_ONLY:-0}" = "1" ]; then
        diff -u "${workdir}/host/out.txt" "${workdir}/amiga/out.txt"
        continue
    fi

    src="spec/tests/corpus/${name}/program.clo"
    rm -rf "${workdir}"
    mkdir -p "${workdir}/host" "${workdir}/amiga"

    "${CLC}" --profile host -o "${workdir}/host/program.c" "${src}"
    cc -std=c99 -O0 -Wall -Wextra -Werror \
        -o "${workdir}/host/program" "${workdir}/host/program.c"
    host_dir="$(cd "${workdir}/host" && pwd)"
    ( cd "${host_dir}" && timeout 120 ./program | tr -d '\r' > out.txt )

    "${CLC}" --profile hosted-amiga -o "${workdir}/amiga/program.c" "${src}"
    m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror \
        -o "${workdir}/amiga/program" "${workdir}/amiga/program.c" -lamiga -lm
    amiga_dir="$(cd "${workdir}/amiga" && pwd)"
    ( cd "${amiga_dir}" && timeout 120 vamos "${amiga_dir}/program" | tr -d '\r' > out.txt )

    if ! diff -u "${workdir}/host/out.txt" "${workdir}/amiga/out.txt"; then
        echo "differential: ${name}: output differs" >&2
        exit 1
    fi
done
```

The executor keeps the exact compile flags aligned with `docker/scripts/corpus-amiga.sh` and uses
the existing `vamos` invocation. The program list is fixed at AR-10; a future portable program
joins only with a new decision. `DIFF_ONLY=1` exists for ST-9: it must not regenerate the outputs,
or the corruption would be overwritten.

### `.github/workflows/check.yml`

```yaml
name: check
on:
  push:
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: check-${{ github.ref }}
  cancel-in-progress: true

jobs:
  host:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with:
          persist-credentials: false
      - name: Install Rust
        run: rustup toolchain install stable --profile minimal
      - name: Spec check
        run: ./spec/check.sh
      - name: Workspace tests
        run: cargo test --workspace
```

The SHA is the `actions/checkout` v4.2.2 commit; the executor verifies it at first run. The
workflow is host-only; it does not build the SDK image and does not run `make verify`.
`AGENTS.md` records that the m68k-only tests skip without `m68k-amigaos-gcc`.

### Integration Points

- `make asan` and `make differential` are container targets; `make check` stays the fast gate.
- `make differential` builds `clc` first (`cargo build --workspace --quiet` in the script unless
  `DIFF_ONLY=1`), emits its own host/Amiga copies, and does not consume `corpus-amiga` artifacts.
- The exact `make verify` recipe is: `check` → `differential` → `corpus-fs-uae` (which depends on
  `corpus-amiga`) → `run-fault-amiga`. This is the verified insertion point; it avoids relying on
  a `corpus-amiga` line that `verify` does not have.
- The final production gate runs all four commands.

## Code Examples

```sh
make asan
make differential
DIFF_ONLY=1 make differential   # negative control, after corrupting a throwaway output
make verify
gh workflow run check.yml        # or push a branch and open a pull request
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| No `RDC_ASAN` files found | Fail with a clear "no suites discovered" message | AR-11 |
| One ASan suite times out | Report the suite name and keep the remaining suites running | AR-11 |
| ASan child-run flake | Serialise tests inside each suite; record the environment residual | AR-11 |
| `vamos` or `m68k-amigaos-gcc` missing | Fail the differential target with the missing tool named; never silently skip | AR-10 |
| Host and m68k output differ | Print the unified diff and exit nonzero | AR-10 |
| `clc` missing or stale | Guard `-x`; the target builds `cargo build --workspace --quiet` first | AR-10 |
| `DIFF_ONLY=1` has no prior outputs | Fail with the missing path; do not regenerate | AR-10 |
| CI runner lacks m68k tools | The m68k-only tests skip with their existing note; host tests still run | AR-9 |
| Workflow has a syntax error | The first push reports it; fix before the final gate | AR-9 |
| Workflow run is not triggered | Record local command evidence and mark acceptance 9 as locally validated | AR-9 |

> **Traceability:** Every error-handling strategy and design choice references the Ambiguity
> Register entry that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- ST-7 – ST-10 in `07-testing-strategy.md`.
- Run `make asan` and `make differential` on the clean tree; both exit zero.
- Run one negative differential probe with `DIFF_ONLY=1` after copying and corrupting a host
  output in a throwaway directory; the diff must fail. Do not modify tracked goldens.
- Run one negative `asan-suite.sh` probe with a deliberately failing suite or a missing discovery;
  it must report the failure and exit nonzero.
- Run the CI workflow commands locally on the host: `./spec/check.sh` and
  `cargo test --workspace`.
- Record the CI run URL and conclusion after the first authorized push; otherwise record the local
  evidence and the deferred CI status.
- The final `make verify` includes `make differential`.
