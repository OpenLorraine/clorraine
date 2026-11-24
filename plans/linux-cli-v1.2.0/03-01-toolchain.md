# Design: CLI and toolchain

> **Requirements**: R4–R6, Q1, Q5, Q6
> **Evidence**: upstream rc24 layout and both-crt build in
> `spikes/07-linux-cli-fs-uae/README.md`

## Public surface

| Command/flag | Behavior |
|---|---|
| `clorraine setup --toolchain PATH` | Validate the upstream root, then store its canonical path. |
| `clorraine build --toolchain PATH ...` | Build with this root for this invocation. |
| `clorraine run --toolchain PATH ...` | Build and run with this root for this invocation. |
| `CLORRAINE_TOOLCHAIN=PATH` | Environment override for `build` and `run`. |
| Config file | Stored path used when no flag or environment value is present. |
| `--sdk-root` | Removed. A usage error names `--toolchain`. |
| `CLORRAINE_SDK` | Removed. No fallback. |

`--toolchain` is accepted only by `setup`, `build`, and `run`. `emit` is an
in-process operation and rejects it. `setup` is the only command that writes
the config.

## Config

Path:

1. `$XDG_CONFIG_HOME/clorraine/config` when `XDG_CONFIG_HOME` is absolute.
2. Otherwise `$HOME/.config/clorraine/config`.

Format is one line; unknown keys are ignored silently in this release:

```
toolchain=/home/user/toolchains/amiga-gcc-16.2
```

The file is UTF-8. A path containing a control character is rejected before
writing. The value is every byte after the first `=`, with only the trailing
newline removed; spaces are significant and the writer emits no spaces around
`=`. A path containing `=` is valid because parsing splits on the first `=`
only. Paths with a trailing space therefore round-trip unchanged.

Write algorithm:

1. Create the config directory.
2. Write `config.tmp.<pid>` in the same directory with mode 0644.
3. Flush and `sync_all`.
4. Rename over `config`.
5. On any error, remove the temporary file and leave the old config untouched.

There is no merge: `setup` owns the `toolchain` key. A future key is preserved
only if the parser can round-trip it; this plan does not add one.

## Validation

`setup --toolchain PATH` runs the full validator. `build`/`run` re-check the
layout and executable, but do not repeat the two-crt compile probe.

Full validation:

| Step | Check | Failure |
|---|---|---|
| 0 | If `getconf GNU_LIBC_VERSION` is available, it reports glibc >= 2.38; if it is unavailable, continue because the CLI already loaded | exit 1, names 2.38 and Ubuntu 24.04 |
| 1 | Canonicalize PATH; it is a directory | exit 2, `--toolchain: not a directory: PATH` |
| 2 | Path is UTF-8 without control characters | exit 2, `--toolchain: path must be UTF-8 without control characters` |
| 3 | `bin/m68k-amigaos-gcc` resolves to an executable regular file inside the root | exit 2, missing or escaping compiler |
| 4 | `m68k-amigaos/libnix` and `m68k-amigaos/ndk-include` are real directories inside the root | exit 2, incomplete toolchain |
| 5 | `<compiler> --version` exits 0 within 10 seconds and contains `16.2-rc24` | exit 1, print the first line; if it contains `GLIBC`, add the 2.38 requirement; otherwise name the supported v16.2-rc24 pin |
| 6 | Compile `probe.c` with `-mcrt=nix20` and `-mcrt=nix13`, `-lamiga -lm`, each within 30 seconds | exit 1, print the compiler's first error and the exact failed command |
| 7 | Remove the probe directory | failure to remove is not fatal; report it as a warning |

The probe is:

```c
#include <proto/exec.h>
#include <proto/dos.h>
int main(void) { return 0; }
```

It proves the NDK headers, both libnix runtimes, `libamiga`, and the linker.
The rc24 root contains `m68k-amigaos/libnix` and
`m68k-amigaos/ndk-include`; the probe is the functional authority and the
layout checks fail early on a clearly incomplete tree. The probe writes only
under a private temporary directory and never needs a ROM.

The supported pin is `v16.2-rc24`. The validator requires that version string;
another `m68k-amigaos-gcc` is rejected with the pin named. This makes the
"pinned toolchain" claim testable rather than documentation-only.

Build/run validation steps 1–4 only. A broken library then fails at the real
link with the compiler's own message, which is more useful than a cached probe.

## Resolution

For `build` and `run`:

1. If `--toolchain PATH` is present, use it. An invalid value is an error; do
   not try the environment or config.
2. Else if `CLORRAINE_TOOLCHAIN` is set, use it with the same rule.
3. Else read the config. A missing config means no toolchain selected.
4. Else, for a non-host profile, keep the legacy `PATH` build only when the
   Docker SDK marker `/opt/amiga/BUILD-PROVENANCE.txt` exists and
   `m68k-amigaos-gcc` is an executable on `PATH`. This is the existing SDK
   container/developer path: the compiler name is spawned from `PATH` and
   `--emulator` keeps its historical host handoff. A user host with a compiler
   on `PATH` but no marker still errors; it must run `setup`.
5. Else exit 2 with:
   `clorraine: no toolchain selected; run 'clorraine setup --toolchain PATH' or pass --toolchain PATH`.

Host builds and `emit` never resolve a toolchain. `hosted-amiga`,
`hosted-amiga-13`, and `freestanding` use an explicit toolchain or the
marker-gated legacy `PATH` fallback. The legacy branch keeps the existing
handoff and its tests unchanged; the native branch selects the verified FS-UAE
runner.

Relative paths are resolved against the caller's current directory, then
canonicalized. The returned path is absolute for the whole invocation, so a
later `cd` or a replaced symlink cannot change the compiler.

## Build integration

`build::build` receives `Option<&Path>` as it does today. With a native
toolchain, the argument vector starts with the absolute
`<root>/bin/m68k-amigaos-gcc`; without one, the existing profile program name is
used, which is `cc` for host and `m68k-amigaos-gcc` from `PATH` for the legacy
Docker branch. The flag tables in `toolchain.rs:61-64` stay unchanged:

- `hosted-amiga`: `-std=c99 -mcrt=nix20 -m68000 -fwrapv`
- `hosted-amiga-13`: `-std=c99 -mcrt=nix13 -m68000 -fwrapv`

Debug, release, and size modes keep their current flags. The spike built hello
and the corpus with the debug line and hello with the release line.

The `freestanding` profile also receives the absolute compiler. Its
`-nostdlib -lg` line and the host `cc` run leg are unchanged; a build-only
check is part of ST-11 so removing the old resolver cannot silently break that
profile. If the upstream toolchain cannot link the freestanding skeleton, stop
and revise the plan; do not drop the profile silently.

## Removed coupling

The old `NativeSdk` native-assembly selection, `installation.json`,
`.ndk-prepared.json`, and paired `sdk/vamos` venv are removed. The legacy
`PATH` branch stays for the Docker SDK and its handoff tests. A non-emulator
Amiga `run` uses `vamos` from `PATH` if the user wants it; otherwise the error
says to use `--emulator` or install amitools. The native FS-UAE acceptance path
does not need `vamos`.

## Error and test contract

| Situation | Exit | Message shape |
|---|---|---|
| `setup` succeeds | 0 | `configured toolchain <absolute path>` and the compiler version |
| Invalid flag value | 2 | `clorraine: --toolchain: <reason>` |
| Incomplete root | 2 | names the missing path |
| Compiler probe fails | 1 | `clorraine: toolchain probe failed: <first error>` |
| No toolchain selected and no `PATH` compiler | 2 | names `setup --toolchain` |
| Stored config unreadable | 2 | names the config path and the parse error |
| Host build with `--toolchain` | 2 | `--toolchain is only valid for non-host build/run/setup` |

Spec tests cover the public contract; unit tests cover config parsing, atomic
write, and resolver precedence. A fake toolchain directory with a shell
compiler lets `make check` verify the absolute path, flags, and both-crt calls
without the real 318 MB tree. The real rc24 build is the local
`make toolchain-check` leg (ST-11b), not a CI test.

The existing `compiler/tests/native_driver_spec_test.rs`,
`native_driver_impl_test.rs`, `native_driver_impl/`, `native_driver_support/`,
and `native_gate_spec_test.rs` pin the old `sdk/amiga` assembly, removed flags,
capture rejection, and process-status return. Rewrite or retire those cases for
the new contract; keep the legacy handoff tests in
`clorraine_driver_spec_test.rs` green. The supersession is recorded in AR-16;
the plan approval is the required ruling.
