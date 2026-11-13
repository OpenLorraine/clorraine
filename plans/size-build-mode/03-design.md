# Design: Opt-in Size Build Mode (PLAN-37)

> **Document**: 03-design.md
> **Parent**: [Index](00-index.md)

## 1. Build-Mode Model

`compiler/src/clorraine/toolchain.rs` gains one enum:

```rust
pub enum BuildMode {
    Debug,
    Release,
    Size,
}
```

`BuildMode` is the single source of truth for "which optimization flags does
this build use". The CLI stores it in `Options.mode` instead of the current
`release: bool`, so the invalid "both true" state cannot exist. `BuildMode`
derives `Clone, Copy, Debug, PartialEq, Eq`.

`Tool` gains a `size: &'static [&'static str]` slice. Its doc comment states
that the slice is release processing with `-Os` in place of `-O2`.

`command(tool, mode, c_path, bin_path)` selects:

| Mode | Flags |
|---|---|
| `Debug` | `-O0 -g`, then `-Wall -Wextra -Werror` |
| `Release` | `tool.release` |
| `Size` | `tool.size` |

Warnings remain debug-only. The argument order stays
`<compiler> <base> <mode> [-warnings] -o <bin> <c> <libs>`.

## 2. Per-Profile Size Vectors

| Profile | Size vector |
|---|---|
| host Linux | `cc -std=c99 -Os -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c` |
| host non-Linux | `cc -std=c99 -Os -o prog out.c` |
| `hosted-amiga` | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -Os -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o prog out.c -lamiga -lm` |
| `hosted-amiga-13` | the same line with `-mcrt=nix13` |
| freestanding m68k | `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -Os -o prog out.c -lg` |
| freestanding host run, native syscalls | `cc -std=c99 -ffreestanding -fno-builtin -nostdlib -Os -o prog out.c` |
| freestanding host run, fallback | `cc -std=c99 -ffreestanding -fno-builtin -Os -o prog out.c` |

The host predicate remains `host_is_linux()` and selects the GNU-linker line
only on Linux, exactly as it does for release.

## 3. CLI Surface

`compiler/src/clorraine/cli.rs`:

- `Options.mode: BuildMode`, initialized to `BuildMode::Debug`.
- `--release` selects `BuildMode::Release`.
- `--size` selects `BuildMode::Size`.
- A small helper applies a requested mode:
  - the same mode repeated is accepted (idempotent);
  - a different non-debug mode is rejected with
    `--release and --size are mutually exclusive`.
- `validate` rejects `--size` with `emit` using the message
  `--size is not valid with 'emit'`. The existing `--release` message is
  unchanged.
- The help text adds:

```
      --size                          Optimize for size (-Os); same release processing; cannot combine with --release
```

The `--release` help line is unchanged, including its symbol-bearing-profile
wording.

`build.rs` passes `BuildMode` to `compile`; `mod.rs` passes `Options.mode` to
`build` and to the freestanding host-run `compile`. No other module needs to
know what a mode means.

## 4. Safety Argument

| Concern | Why it is safe |
|---|---|
| Default behaviour | Debug and release vectors are byte-identical to the base; the new slice is only read for `Size` |
| Invalid state | The enum cannot represent release and size at once; the parser rejects the conflicting flags before any build starts |
| Emitted C | `clc` never sees the mode, so the translation unit is unchanged |
| Profile coverage | Every profile that has a release vector also has a size vector, including both host fallback branches |
| Freestanding run | Both the m68k build and the host-run compile receive the same mode, so a size run does not mix `-Os` and `-O2` |
| Scored gate | `tools/perf/bench.py` still uses its own `-O2` constants; the scored line is untouched |
| Frozen spec | `spec/17-codegen.md` still defines the required default matrix; `--size` is an additional opt-in line, not a replacement |

## 5. Rejected Alternatives

| Alternative | Reason |
|---|---|
| Make `-Os` the default release line | The current `moves` result (+31.3% net instructions) and the no-regression rule for scored benchmarks keep `-O2` as the default |
| `--release --size` means "release plus size" | Ambiguous; two storage modes in one build. A conflict error is explicit |
| `--size` requires `--release` | The flag is a build mode. Making the user write two flags for one decision is noise and invites accidental `-O2` builds when `--release` is forgotten |
| Reuse `Options.release: bool` and add `size: bool` | Allows an invalid both-true state and scatters the decision across call sites |
| Forward arbitrary C flags (`--cflag=-Os`) | A much larger security and support surface; the product decision is one measured mode |
| Add `-Oz`, LTO, or per-function flags | No measured evidence in this issue and a larger gate |
| Change the benchmark harness to measure `-Os` | The harness is the scored `-O2` oracle; size-mode evidence is recorded separately |
| Change `spec/17-codegen.md` | The spec is frozen; the default matrix is unchanged and a driver convenience does not need a language-spec row |
