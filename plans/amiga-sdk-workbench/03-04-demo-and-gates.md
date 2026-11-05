# Design: Demo and Gates

> **Document**: 03-04-demo-and-gates.md
> **Parent**: [Index](00-index.md)
> **Covers**: R5, R6, R7, AR-3–AR-5, AR-9, AR-13, AR-14

## Workbench Demo

`examples/amiga/workbench.clo` runs on `hosted-amiga` on an A1200/KS3.1 and an
A600/KS2.05.

### Sequence

| Stage | Action | Failure code |
|---|---|---|
| 1 | declare the `portName` and `screenTitle` `String`s, open a 320x80x3 screen and a 300x64 window | 1 |
| 2 | create the public port `ClorraineWbTest` | 2 |
| 3 | add the AppIcon `Clorraine` (id 7, user data 0) | 3 |
| 4 | add the AppWindow (id 1, user data 0) on the custom window | 4 |
| 5 | add the AppMenuItem `Clorraine` (id 2, user data 0) | 5 |
| 6 | wait ten ticks for the AppIcon redraw, then save the Workbench public screen to `CNEXT:workbench.cscr` | 6 |
| 7 | write `CNEXT:wb-ready.txt` with `ready` | 7 |
| 8 | wait for three `AppMessage`s | 8 |
| 9 | write `CNEXT:wb.txt` with the report line (no console print, so the fixture stdout stays clean) | 9 |
| 10 | reply each message, remove the application objects, close the window and screen | — |

### Report Line

```text
wb: ok icon 1 win 1 menu 1 types 8 7 9 ids 7 8 9 users 42 43 44 classes 0 0 0 args 0 0 0
```

`icon`/`win`/`menu` are 1 when the object handle is non-null. The `types` are
`am_Type` values 8/7/9 (`AMTYPE_APPICON`, `AMTYPE_APPWINDOW`,
`AMTYPE_APPMENUITEM`), the `ids` are 7/8/9, the `users` are 42/43/44, the
`classes` are 0, and the `args` counts are 0. The three messages are posted by
the fixture in that order; `waitAppMessage` returns them in FIFO order.

### Ordering

The `portName` and `screenTitle` `String`s are declared before the port and
screen; the port and screen borrow their cstrings, so the names must outlive
them. The screen is declared before the window, and the window before the
AppWindow; the AppWindow is removed before `closeWindow`. The public port is
declared before the AppIcon and AppMenuItem, and the port drops after them.
Each AppMessage is replied explicitly; the generated drop would reply an
unhandled message.

## Services Demo

`examples/amiga/services.clo` runs on `hosted-amiga` on an A600/KS2.05 and an
A1200/KS3.1.

| Stage | Action |
|---|---|
| 1 | `getDefDiskObject(ICON_TOOL)`, `putDiskObject("CNEXT:svc-icon", icon)`, `getDiskObject`, `diskObjectType`, free both |
| 2 | `expansionBoardCount`, `expansionBoardManufacturer(0)`, `expansionBoardProduct(0)` |
| 3 | `cardPresent`, `cardStatus` |
| 4 | `mapRawKey(0x20, 0)` |
| 5 | print `svc: ok icon 3 exp 2 man 2011 prod 82 card 1 status 0 key 0` |

`icon 3` is the loaded `WBTOOL` type. `exp 2` is the board count and
`man 2011 prod 82` is the first Commodore board (pinned on the A600/KS2.05 and
A1200/KS3.1 models). `card 1` means the resource is present; `status 0` is the
`ReadCardStatus` value. `key 0` means the keymap call returned no text because
the bare ROM has no loaded keymap layout. The gate redirects the printed line
to `CNEXT:svc.txt`; the program does not also write the file.

## Test Fixtures

### `docker/scripts/workbench-start.c`

- Opens `workbench.library` 37.
- Calls the private `StartWorkbench` vector at LVO -42 with `(0, 0)`.
- Waits 2 s and locks `Workbench` to confirm the screen.
- Prints `startwb <ok> screen <hex>`; the startup-sequence redirects it to
  `CNEXT:wb-start.txt`.
- Exits 0 when the screen exists; exits 1 otherwise.

This is test-only. It mirrors `C:LoadWB`, which is on the unavailable
Workbench disk. The public SDK never exposes the private vector.

### `docker/scripts/workbench-post.c`

- Starts `CNEXT:wb-demo` asynchronously with
  `SystemTags("CNEXT:wb-demo", SYS_Asynch, TRUE, TAG_END)`.
- Polls `FindPort("ClorraineWbTest")` for up to 10 s.
- Creates a reply port and three `struct AppMessage`s. Each has
  `mn_Length = sizeof(struct AppMessage)`, `mn_Node.ln_Type = NT_MESSAGE`,
  `mn_ReplyPort = reply`, `am_Version = AM_VERSION`, `am_Class = 0`, and
  `am_NumArgs = 0`. The three are:
  - `am_Type = AMTYPE_APPICON`, `am_ID = 7`, `am_UserData = 42`;
  - `am_Type = AMTYPE_APPWINDOW`, `am_ID = 8`, `am_UserData = 43`;
  - `am_Type = AMTYPE_APPMENUITEM`, `am_ID = 9`, `am_UserData = 44`.
- `PutMsg` each to the public port, waits for the three replies, and prints
  `posted`; the startup-sequence redirects it to `CNEXT:wb-post.txt`.
- On a missing port, prints `error no port` and exits 1.
- Links with `-lamiga -lm` for `FindPort`, `CreatePort`, and `PutMsg`.

## Gates

### `make sdk-a1200-workbench`

1. Build `workbench.clo` and `services.clo` with `clorraine build --profile
   hosted-amiga`; build the two C fixtures with `m68k-amigaos-gcc -mcrt=nix20
   -m68000 -fwrapv -O0 -Wall -Wextra -Werror ... -lamiga -lm`.
2. Decrypt `amiga-os-310-a1200.rom` locally and verify SHA-1
   `e21545723fe8374e91342617604f1b3d703094f1`.
3. Clear and rebuild a directory HD with `s/startup-sequence`:
   ```text
   CNEXT:workbench-start >CNEXT:wb-start.txt
   CNEXT:workbench-post >CNEXT:wb-post.txt
   CNEXT:services >CNEXT:svc.txt
   ```
   and `workbench-start`, `workbench-post`, `wb-demo`, `services`.
4. Boot FS-UAE as A1200 with KS3.1, 2 MB chip, no fast memory, the directory
   HD, a 640x256 borderless window, scale 1, crop 1, and a private Xvfb
   display.
5. Poll `CNEXT:wb-start.txt` for `startwb 1`, `CNEXT:wb-ready.txt` for
   `ready`, `CNEXT:wb.txt` for `wb: ok`, `CNEXT:wb-post.txt` for `posted`, and
   `CNEXT:svc.txt` for `svc: ok`, up to 60 s.
6. Stop the emulator; assert the full `wb:` and `svc:` lines and `posted`.
7. Convert `workbench.cscr` with `cscr2png.py ... --scale 2`; compare the full
   image to `docs/amiga-sdk/a1200-workbench.png` byte-exact; assert the image
   is non-blank; crop the AppIcon region `(800,280,160,120)` and compare it to
   `docs/amiga-sdk/a1200-workbench-icon.png`.
8. On failure, keep the screen and the report files.

### `make sdk-a600-services`

1. Build the two `.clo` demos with `clorraine build --profile hosted-amiga` and
   the two C fixtures with `m68k-amigaos-gcc ... -lamiga -lm`.
2. Decrypt `amiga-os-205-a600.rom` and verify SHA-1
   `02843c4253bbd29aba535b0aa3bd9a85034ecde4`.
3. Clear and rebuild a directory HD with `s/startup-sequence`:
   ```text
   CNEXT:workbench-start >CNEXT:wb-start.txt
   CNEXT:workbench-post >CNEXT:wb-post.txt
   CNEXT:services >CNEXT:svc.txt
   ```
   and the four programs.
4. Boot FS-UAE as A600 with KS2.05, 1 MB chip, no fast/slow memory, the HD, and
   a private Xvfb display.
5. Poll `wb-start.txt`, `wb-ready.txt`, `wb.txt`, `wb-post.txt`, and `svc.txt`
   as above.
6. Stop and assert `startwb 1`, `ready`, the full `wb:` line, `posted`, and the
   full `svc:` line. No capture comparison is made on 2.05.

Both gates use the directory hard drive so report files land on the host
directly and no writable-floppy sync is involved. Neither gate needs host
input or opens a window on the user's desktop. Both clear the HD before every
run, so a stale report, capture, or icon file cannot satisfy a poll.

## Determinism Notes

- The Workbench screen layout is pinned by the `WBAPPICONA_PropagatePosition`
  tag and `do_CurrentX/Y = (400, 140)`. The native capture is committed from an
  accepted run and compared byte-exact; the AppIcon crop is compared
  separately so a missing icon cannot hide behind a recaptured golden.
- The fixture posts exactly three messages after the public port exists; the
  demo waits for exactly three. There is no scheduling-dependent count.
- The expansion-board count/fields and the card status are hardware constants
  on the A600/A1200 models; the `svc` line pins them.
- The keymap result is 0 on the bare ROM; a Workbench installation with
  `DEVS:keymaps` would change it, so the gate records the limitation.

## Evidence

| Artifact | Path |
|---|---|
| Workbench start | `CNEXT:wb-start.txt`; asserted `startwb 1` |
| Workbench ready | `CNEXT:wb-ready.txt`; asserted `ready` |
| Workbench report | `CNEXT:wb.txt`; asserted |
| Fixture report | `CNEXT:wb-post.txt`; asserted `posted` |
| Workbench native capture | `CNEXT:workbench.cscr`; converted and compared |
| Workbench golden | `docs/amiga-sdk/a1200-workbench.png` |
| AppIcon crop | `docs/amiga-sdk/a1200-workbench-icon.png` |
| Services report | `CNEXT:svc.txt`; asserted |
| Failure screen | `build/amiga-sdk/a1200-workbench/screen.png` (gitignored) |
| Fixtures | `docker/scripts/workbench-start.c`, `workbench-post.c` |
| Gates | `docker/scripts/a1200-workbench-smoke.sh`, `a600-services-smoke.sh` |
