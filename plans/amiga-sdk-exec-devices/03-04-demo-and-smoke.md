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
| 1 | run the worker stress loop: eight `workerSum` workers (55 each) and one `workerProduct` worker (24), created and joined one at a time; total `8 * 55 + 24 = 464`; a tenth `workerZero` task is left to the generated drop | 1/2 |
| 2 | register a VBlank handler, a ports handler, wait 3 ticks, confirm the VBlank counter advanced, remove both | 3 |
| 3 | open `exec.library` v33, read its version, and let the generated drop close it | 4 |
| 4 | open `trackdisk.device` unit 0, `CMD_READ` 512 bytes at offset 0 with a synchronous `deviceDoIO` over the caller buffer; fail stage 5 before the async read if the first read is short/failed | 5 |
| 5 | reserve a device-owned buffer, `CMD_READ` the same block with `SendIO`/`WaitIO`, and copy the result out; the second read must report error 0 and `io_Actual` 512 | 6/7 |
| 6 | call a `writeConsole()` helper: open a 320x80x3 screen and a 300x64 window, `openConsole(window, 0)`, `CMD_WRITE` the 13-byte line `exec console\n` with a synchronous `deviceDoIO`, then `CMD_READ` with a device-owned buffer and `SendIO`/`AbortIO`/`WaitIO`, expect `IOERR_ABORTED` (-2), close the console/window, and let the screen drop at helper return. The window is refcounted, so the console holds it until the device closes | 8/9 |
| 7 | open `input.device`, read its last error, and let the generated drop close it | 10 |
| 8 | open and close `keyboard.device` unit 0 | 11 |
| 9 | print the report line, write `DF0:exec.txt`, and check the written byte count | 14 |

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
`io_Actual` (the 13-byte `exec console\n`). The `key` field is the aborted
console read's `WaitIO` error; the dedicated console window has no pending
input, so the abort is deterministic. The `in` field is 1 after the input
device opens and closes cleanly.

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
    key: i32;
    code: i32;
}

func writeConsole(): ConsoleResult {
    let title: String = stringOf("exec console");
    let screen: Screen? = openScreen(320, 80, 3, toCString(title));
    if (screen == null) {
        return ConsoleResult { ok: false, actual: 0, key: 0, code: 8 };
    }
    setColor(screen, 0, 150, 165, 190);
    setColor(screen, 1, 20, 30, 50);
    let window: Window? = openWindow(screen, 8, 8, 300, 64,
                                     toCString(stringOf("exec console")));
    if (window == null) {
        return ConsoleResult { ok: false, actual: 0, key: 0, code: 8 };
    }
    let console: Device? = openConsole(window, 0);
    if (console == null) {
        closeWindow(window);
        return ConsoleResult { ok: false, actual: 0, key: 0, code: 8 };
    }
    let line: String = stringOf("exec console\n");
    let writeCommand: bool = deviceCommand(console, CMD_WRITE);
    if (!writeCommand) {
        closeDevice(console);
        closeWindow(window);
        return ConsoleResult { ok: false, actual: 0, key: 0, code: 8 };
    }
    let error: i32 = deviceDoIO(console, bytes(line));
    let actual: u32 = deviceActual(console);
    if (error != 0 || actual != 13) {
        closeDevice(console);
        closeWindow(window);
        return ConsoleResult { ok: false, actual: actual, key: 0, code: 9 };
    }
    let readBuffer: u8[64] = [; 0];
    let readCommand: bool = deviceCommand(console, CMD_READ);
    let readReserve: bool = deviceSetBuffer(console, 64);
    if (!readCommand || !readReserve) {
        closeDevice(console);
        closeWindow(window);
        return ConsoleResult { ok: false, actual: actual, key: 0, code: 9 };
    }
    if (!deviceSendIO(console)) {
        closeDevice(console);
        closeWindow(window);
        return ConsoleResult { ok: false, actual: actual, key: 0, code: 9 };
    }
    deviceAbortIO(console);
    let keyError: i32 =
        deviceWaitIO(console, borrow slice(readBuffer, 0, 64));
    closeDevice(console);
    closeWindow(window);
    if (keyError != -2) {
        return ConsoleResult { ok: false, actual: actual, key: keyError, code: 9 };
    }
    return ConsoleResult { ok: true, actual: actual, key: keyError, code: 0 };
}
```

The caller maps `code` to `failedKey(console.code, console.key)` and keeps the
observed `key` error for the failure diagnosis. The console read is on the
dedicated window; see RV-1 for why the keyboard device itself is only
opened/closed.

Declare the screen title before the screen so the screen drops first. The
synchronous write borrows `line` only for the `deviceDoIO` call; the
asynchronous read uses the device-owned buffer and copies into `readBuffer` at
`deviceWaitIO`, so neither pointer outlives the caller's scope. The console
device closes before the window; the window is refcounted, so the console holds
it until `closeDevice` releases the last reference. The screen drops at helper
return, so it cannot cover the Shell report line.

### Failure Output

On failure the program prints `exec: err <code>` and exits 1. A failed console
read prints `exec: err 9 key <error>` so the observed abort error is visible. A
partial console screen may be visible when stage 6 fails; the helper closes it
on the failure paths. The code values are the `Failure code` column above.

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
   - poll the raw ADF for `exec: ok task 464` (the runtime value, not the
     binary's format-string literal; see RV-2);
   - once the line is present, crop `620x22` at `(160,88)` and compare it to
     `docs/amiga-sdk/a500-exec.png`. The Exec line is 50 characters, so the
     crop is wider than the hello gate's 430-pixel crop at the emulator's
     1.5x scale.
6. Stop the emulator, read `exec.txt` from the ADF, and assert the full line;
   a missing file prints the ADF listing and fails.
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
  the sum is fixed, so the report does not depend on scheduling. The
  create/join cycles exercise the `Forbid`/`RemTask` removal path repeatedly;
  the tenth `workerZero` task is left to the generated drop. The `exec.library`
  and `input.device` handles are also left to their generated drops, so the
  A500 gate exercises the drop glue as well as the explicit close paths.
- The VBlank handler only proves that at least one tick arrived; the count is
  not reported.
- The ports handler is registered and removed but is not required to fire.
- The trackdisk reads use offset 0, so the device's offset unit does not
  matter. The first read borrows the caller's buffer for the synchronous call;
  the second uses the device-owned buffer, so no caller pointer outlives its
  scope (MS-1). The first read is checked before the async read starts.
- The aborted read is a `console.device` read on the dedicated window, which
  has no pending input, so the error is deterministically -2. The 1.3
  `keyboard.device` read completes with a queued event/error instead of staying
  pending (RV-1), so the keyboard and input devices are open/close coverage. A
  key press on the console window would change the line and fail the gate,
  which is the intended behavior for a deterministic headless run.
- The console write and read are on a dedicated screen/window, so they cannot
  disturb the Shell report line; the window is refcounted and the screen is
  closed before the line is printed.
