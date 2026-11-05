# Design: Demo and A500 Smoke

> **Document**: 03-04-demo-and-smoke.md
> **Parent**: [Index](00-index.md)
> **Covers**: R7, R8, AR-13, AR-14

## Demo Program

`examples/amiga/exec-demo.clo` is a console program with a short Intuition
interlude for the console device. It runs a fixed sequence and ends with one
report line:

```text
exec: ok task 464 lib 34 dev 512 io 13 key -2 in 1
```

### Sequence

| Stage | Action | Failure code |
|---|---|---|
| 1 | run the worker stress loop: eight `workerSum` workers (55 each) and one `workerProduct` worker (24), created and joined one at a time; total `8 * 55 + 24 = 464` | 1/2 |
| 2 | register a VBlank handler, a ports handler, wait 3 ticks, confirm the VBlank counter advanced, remove both | 3 |
| 3 | open `exec.library` v33, read its version, close it | 4 |
| 4 | open `trackdisk.device` unit 0, `CMD_READ` 512 bytes at offset 0 with `DoIO` | 5 |
| 5 | `CMD_READ` the same block with `SendIO`/`WaitIO`; both reads must report error 0 and `io_Actual` 512 | 6/7 |
| 6 | call a `writeConsole()` helper: open a 320x80x3 screen and a 300x64 window, `openConsole(window, 0)`, `CMD_WRITE` the 13-byte line `exec console\n` with `DoIO`, close the console/window, and let the screen drop at helper return | 8/9 |
| 7 | open and close `input.device` unit 0 | 10 |
| 8 | open `keyboard.device` unit 0, `CMD_READ` into a 64-byte buffer with `SendIO`, `AbortIO`, `WaitIO`; expect `IOERR_ABORTED` (-2) | 11/12/13 |
| 9 | print the report line, write `DF0:exec.txt` with the full line | 14 |

The console helper returns before the report line, so the custom screen is
already closed when the Shell prints. The Shell output line is the first
console line after the command and the line crop uses the same origin as the
hello gate. The report line is printed in full by `println` and written to the
file.

### Report Line

```text
exec: ok task <sum> lib <version> dev <actual> io <console-length> key <error> in <1>
```

On KS1.3 the pinned values are `task 464`, `lib 34`, `dev 512`, `io 13`,
`key -2`, `in 1`. The `task` value is `8 * 55 + 24`. The `dev` field is the
first trackdisk read's `io_Actual`; the second read is checked in the program
but not repeated in the line. The `io` field is the console write's
`io_Actual` (the 13-byte `exec console\n`). The `key` field is the keyboard
`WaitIO` error. The `in` field is 1 after the input device opens and closes
cleanly.

### Worker Shape

```clorraine
volatile let ticks: u32 = 0;

func onTick(): void { ticks = ticks + 1; }
func onPort(): void { }

func workerSum(): void { reportTaskResult(55); }
func workerProduct(): void { reportTaskResult(24); }

func workerLoop(): i32 {
    let total: i32 = 0;
    let index: i32 = 0;
    while (index < 8) {
        let task: Task? = createTask(workerSum, 8192);
        if (task == null) { return -1; }
        total = total + joinTask(task);
        index = index + 1;
    }
    let product: Task? = createTask(workerProduct, 8192);
    if (product == null) { return -1; }
    return total + joinTask(product);
}
```

The workers only compute and call `reportTaskResult`; they never touch DOS,
Intuition, `std.io`, or the parent's handles. Creating and joining one at a
time keeps at most one worker alive and stresses the removal path nine times.

### Console Helper

```clorraine
struct ConsoleResult {
    ok: bool;
    actual: u32;
    code: i32;
}

func writeConsole(): ConsoleResult {
    let title: String = stringOf("exec console");
    let screen: Screen? = openScreen(320, 80, 3, toCString(title));
    if (screen == null) {
        return ConsoleResult { ok: false, actual: 0, code: 8 };
    }
    setColor(screen, 0, 150, 165, 190);
    setColor(screen, 1, 20, 30, 50);
    let window: Window? = openWindow(screen, 8, 8, 300, 64,
                                     toCString(stringOf("exec console")));
    if (window == null) {
        return ConsoleResult { ok: false, actual: 0, code: 8 };
    }
    let console: Device? = openConsole(window, 0);
    if (console == null) {
        closeWindow(window);
        return ConsoleResult { ok: false, actual: 0, code: 8 };
    }
    let line: String = stringOf("exec console\n");
    if (!deviceCommand(console, CMD_WRITE) ||
        !deviceSetBuffer(console, bytes(line))) {
        closeDevice(console);
        closeWindow(window);
        return ConsoleResult { ok: false, actual: 0, code: 8 };
    }
    let error: i32 = deviceDoIO(console);
    let actual: u32 = deviceActual(console);
    closeDevice(console);
    closeWindow(window);
    if (error != 0 || actual != 13) {
        return ConsoleResult { ok: false, actual: actual, code: 9 };
    }
    return ConsoleResult { ok: true, actual: actual, code: 0 };
}
```

The caller maps `code` directly to `failedAt(console.code)` and uses
`actual` only on success.

Declare the screen title before the screen so the screen drops first. Declare
the line before the `doIO` call and keep it alive through the write; the
`bytes(line)` view is valid until `line` drops at helper return. The console
device closes before the window; the window closes before the screen. The
screen drops at helper return, so it cannot cover the Shell report line.

### Failure Output

On failure the program prints `exec: err <code>` and exits 1. A partial console
screen may be visible when stage 6 fails; the helper closes it on the failure
paths. The code values are the `Failure code` column above.

## A500 Smoke

`docker/scripts/a500-exec-smoke.sh` mirrors the hello gate:

1. Check the binary, `docker/.env`, and the host tools (`xdotool`, `import`,
   `convert`, `compare`, `identify`, `sha1sum`).
2. Decrypt the licensed KS1.3 ROM and verify the official SHA-1.
3. Build a clean ADF with the demo as `exec`, a root `startup-sequence`
   containing `DF0:exec`, the `S/startup-sequence` copy, and the `boot1x`
   boot block.
4. Boot FS-UAE as an A500 with the real ROM and the writable ADF, using the
   same window settings as `docker/scripts/a500-smoke.sh`.
5. Poll from 10 s, every 2 s, up to 60 iterations:
   - re-find the `FS-UAE` window;
   - capture the host screen;
   - poll the raw ADF for the ASCII report line;
   - once the line is present, crop `430x22` at `(160,88)` and compare it to
     `docs/amiga-sdk/a500-exec.png`.
6. Stop the emulator, read `exec.txt` from the ADF, and assert the full line.
7. Assert the line crop is not blank (`identify -format '%k'` is at least 2)
   before the byte-exact comparison, so a blank golden cannot pass.
8. On failure, keep the screenshot and print the ADF listing.

The crop origin is the hello gate's origin. Both demos are single console
commands with one output line; the FS-UAE window settings are the same. The
gate runs under the caller's private Xvfb display (the same arrangement as the
other non-GUI A500 smokes); the script uses `DISPLAY`/`XAUTHORITY` and does not
start an X server itself.

## Make Targets

```make
sdk-a500-exec:
	$(DOCKER_MAKE) exec CMD="cargo build --workspace --quiet && ./target/debug/clorraine build --release --profile hosted-amiga-13 examples/amiga/exec-demo.clo"
	docker/scripts/a500-exec-smoke.sh build/clorraine/exec-demo/hosted-amiga-13/exec-demo
```

The target is added to `.PHONY` and to `verify` after `sdk-a500-gui`.

## Evidence

| Artifact | Path |
|---|---|
| Report line | `DF0:exec.txt` in the ADF; asserted as the pinned string |
| Line crop | `docs/amiga-sdk/a500-exec.png` |
| Last failure screen | `build/amiga-sdk/exec-smoke/screen.png` (gitignored) |
| Demo source | `examples/amiga/exec-demo.clo` |
| Smoke source | `docker/scripts/a500-exec-smoke.sh` |

## Determinism Notes

- The worker stress loop creates and joins one task at a time at priority -1;
  the sum is fixed, so the report does not depend on scheduling. The nine
  create/join cycles exercise the `Forbid`/`RemTask` removal path repeatedly.
- The VBlank handler only proves that at least one tick arrived; the count is
  not reported.
- The ports handler is registered and removed but is not required to fire.
- The trackdisk reads use offset 0, so the device's offset unit does not
  matter.
- The keyboard read is aborted on an idle keyboard; the error is -2. A key
  press would produce a different line and fail the gate, which is the
  intended behavior for a deterministic headless run.
- The console write is on a dedicated screen/window, so it cannot disturb the
  Shell report line; the screen is closed before the line is printed.
