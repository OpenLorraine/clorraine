# Design: verified FS-UAE runs

> **Requirements**: R7–R10, Q3, Q6
> **Evidence**: A1200/KS3.1 wrapper results in
> `spikes/07-linux-cli-fs-uae/README.md`

## Inputs and precedence

| Input | Precedence | Required |
|---|---|---|
| FS-UAE executable | `--fs-uae` > `CLORRAINE_FS_UAE` > `fs-uae` on `PATH` | Yes |
| Kickstart ROM | `--kickstart` > `CLORRAINE_KICKSTART` | Yes for `run --emulator` |
| Timeout | `CLORRAINE_EMULATOR_TIMEOUT_SECONDS` (integer 1–3600), default 90 | No |

FS-UAE must be a regular executable. The runner invokes `fs-uae --version`
once and requires the first output line to be a version token. The tested
output is exactly `3.2.35`; the minimum is 3.2.0. A failed, non-numeric, or
older version is a selection error, not a silent launch. Invalid timeout values
are a usage error.

The ROM must be an ordinary readable file of exactly 524,288 or 524,299 bytes.
The 524,299-byte KS3.1 A1200 image is the tested format; 524,288 is accepted as
the standard raw 512 KB image but is not the acceptance format. A 256 KB image
is rejected with:

```
clorraine: --kickstart: a 256 KB Kickstart 1.3 ROM cannot boot the A1200 recipe;
supply a 512 KB Kickstart 3.1 A1200 ROM
```

The file content is not hashed or identified. The docs state the model
requirement; the user supplies licensed media. The spike tested only KS3.1;
KS3.0 remains untested and is not an acceptance requirement.

This verified runner is selected only when the toolchain resolver returns a
native toolchain (`--toolchain`, `CLORRAINE_TOOLCHAIN`, or stored config). With
only a `PATH` compiler, the existing SDK-container handoff remains and is not a
verified guest run. Acceptance and the product path always select a native
toolchain.

## AROS

There is no implicit AROS fallback. If no ROM is supplied, the command fails
with the guidance above. FS-UAE's built-in AROS was observed to boot and run
hello during the spike, but it is not a supported or acceptance path and is not
offered as a launch-and-forget mode.

## Private staging

The runner creates a 0700 directory under `TMPDIR`, outside the build tree and
the current program directory, using the existing `Storage` rules. It writes:

```
drive/runner                          compiled guest wrapper
drive/program                         copied m68k executable
drive/s/startup-sequence              CNEXT:runner
run.fs-uae                            A1200 configuration
home/, base/, cache/, state/,
logs/, kickstarts/, tmp/              isolated emulator directories
```

The native compiler path from the resolver is passed into the emulator; the
legacy handoff branch has no emulator. The wrapper source is embedded in the
driver at build time and written into the private run, because the release
archive contains only the CLI binaries. It is compiled with the selected native
toolchain and the existing `nix20` flags:

```
<compiler> -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -Wall -Wextra -Werror \
  -o <drive>/runner compiler/runtime/emulator_runner.c -lamiga -lm
```

```c
#include <proto/dos.h>
#include <stdio.h>

int main(void) {
    LONG rc = SystemTagList("CNEXT:program >CNEXT:clorraine-result.txt", NULL);
    FILE *f = fopen("CNEXT:clorraine-rc.txt", "w");
    if (f) { fprintf(f, "%ld\n", (long)rc); fclose(f); }
    if (rc == 0) {
        f = fopen("CNEXT:clorraine-status.txt", "w");
        if (f) { fputs("ok\n", f); fclose(f); }
    }
    return 0;
}
```

The wrapper exists because a two-line `program` plus `echo` script is not a
valid success oracle: a guest returning 1 continued and wrote the marker, and
`FailAt` is not available on the bare directory drive. The wrapper records the
return code and writes `clorraine-status.txt` only for zero. It always returns
zero itself, so the startup script does not depend on AmigaDOS failure levels.

The drive uses `SystemTagList("CNEXT:program >CNEXT:clorraine-result.txt")`; the
program's stdout is the result file. The host reads `clorraine-rc.txt` and
`clorraine-status.txt`, trimming one trailing newline and an optional CR. The rc
must be exactly `0`; the status must be exactly `ok`. An empty result is valid:
a program may print nothing.

The emulator config sets `end_config=1`, `amiga_model=A1200`,
`hard_drive_0_label=CNEXT`, `hard_drive_0_priority=0`, and empty floppy and
CD-ROM drives. The existing environment isolation (`HOME`, `XDG_*`,
`FS_UAE_BASE_DIR`, `TMPDIR`) and literal argv path handling are retained.

## Run loop

1. Compile `runner` into the private drive with the selected native compiler;
   a compile failure is a build error (exit 1) and no emulator starts.
2. Spawn FS-UAE with the config and path options. Redirect its stdout and
   stderr to `logs/fs-uae.out` and `logs/fs-uae.err`; never inherit them.
3. Poll `drive/clorraine-rc.txt` and `drive/clorraine-status.txt` every 250 ms
   until the timeout.
4. Success requires **both** the rc file to be exactly `0` and the status file
   to be exactly `ok` after the trim. An existing but empty/partial rc or status
   is re-read for up to one second until it stabilizes; if it does not, it is a
   failure. A nonzero rc is a guest failure even if a stale/partial status
   exists. A missing rc/status at the timeout is a failure.
5. On a complete result:
   - read the rc, status, and result files into memory; carriage returns are
     stripped from the result before it is printed or captured;
   - kill the child with SIGKILL and poll its reaping for up to five seconds;
     the standard library has no SIGTERM binding and the crate forbids unsafe
     code, so there is no graceful signal stage;
   - if termination cannot be confirmed, retain the private directory and
     report its path; otherwise remove the directory.
6. On a read error, malformed marker, nonzero rc, or timeout:
   - set the cleanup guard to retain the directory;
   - kill/wait as above;
   - report the reason and the retained path;
   - never remove a directory whose result has not been read.
   An emulator that exits before a complete marker is re-read once and then
   reported as missing success, not as a full timeout. A deadline with no
   marker at all is the timeout case.
7. `--capture` writes a **successful** guest result to the normal `out.txt`
   before cleanup; a capture-write failure retains the directory and returns 1.
   A failed guest keeps its output in the retained directory and prints it to
   stdout.

The result is never read after cleanup. The status/rc files are never treated as
a partial success: a status without a zero rc is a failure.

## Output and exit codes

| State | stdout | stderr | Exit |
|---|---|---|---|
| rc `0` + status `ok` | guest result | — | 0 |
| nonzero rc | any captured result | `clorraine: guest program exited with code N` | 1 |
| missing/malformed status or rc | any captured result | `clorraine: guest program did not report success` | 1 |
| timeout | any captured result | `clorraine: guest run timed out after N seconds` | 1 |
| read/capture/cleanup error | any captured result | `clorraine: <error>; private run retained at PATH` | 1 |
| FS-UAE missing/too old, ROM missing/size, invalid timeout | — | `clorraine: <flag>: <reason>` | 2 |
| wrapper compile failure | — | `clorraine: guest runner build failed: ...` | 1 |

`--capture` writes the guest result to the profile's normal `out.txt` path and
prints `captured <path>` instead of the result. It works with `--emulator`; the
old `mod.rs` rejection is removed.

## Profiles

Both `hosted-amiga` and `hosted-amiga-13` are accepted. The recipe always boots
an A1200 with the user's KS3.1 ROM. The A500 profile binary is compatible with
later Kickstarts, and the spike ran hello13 plus a `hosted-amiga-13` build of
`file-copier` successfully; the four corpus runs were `hosted-amiga`. This is
not a Kickstart 1.3 proof.
The existing A500 SDK gates remain the 1.3 acceptance path.

No profile-specific rejection is added. The existing CLI test that expects the
A500 profile to accept `--emulator` stays valid.

## Failure hygiene

- The private directory is removed only after the rc/status/result reads and a
  confirmed child termination.
- A failed read, capture write, or wait retains the directory and prints its
  path.
- FS-UAE stdout/stderr go to files under `logs/`; the CLI's stdout is only the
  guest result or the capture path.
- Paths with control characters or commas in `TMPDIR` keep their existing
  rejection.
- The host never writes a Kickstart ROM into the guest drive.
- The runner owns FS-UAE as a direct child and kills/waits it; it does not put
  FS-UAE in a background `xvfb-run` wrapper. The local guest check requires
  `pgrep`, starts only when no FS-UAE or Xvfb process is alive, and asserts the
  same empty set after every invocation, including a failing guest and the rc-1
  negative. It runs the CLI under `xvfb-run` when no display is set. A failed
  check keeps its evidence directory and prints the path; a successful check
  removes it.

## Test seam

A fake `fs-uae` script can parse `--hard_drive_0=...`, write the rc/status/result
files, and exit. A fake compiler can produce a dummy `runner` and record the
compile flags. Spec tests use both seams for success, rc 1/5/9, malformed,
partial-status, timeout, read-error, capture, and noisy-emulator cases; they do
not need X, a ROM, or the real emulator. A separate local target runs the real
emulator with the licensed ROM (ST-20).
