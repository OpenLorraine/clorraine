# Spec, Corpus, and Video: Clorraine Amiga Graphics

> **Document**: 03-04-spec-corpus-video.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns everything outside the modules and the shim: the frozen
specification edits (AR-3, AR-27), the graphics corpus program and its golden
(AR-10), the CI legs that run it (AR-20, AR-21), and the story-video chapter
(AR-28).

## Architecture

### Current Architecture

- `spec/appendix-amiga.md` §1 lists `intuition`/`graphics` as "later".
- `spec/appendix-stdlib.md` §6–§8 document the Amiga modules; there is no
  graphics section.
- `compiler/tests/corpus_spec_test.rs` special-cases `interrupt-counter` as the
  one Amiga-only corpus program.
- `docker/scripts/corpus-amiga.sh` builds the portable programs and
  `interrupt-counter` build-only; `docker/scripts/corpus-fs-uae.sh` runs only
  `interrupt-counter`.

### Proposed Changes

Add the corpus program and its golden, extend the two scripts and the corpus
test, document the surface, and add a video chapter. (AR-3, AR-21, AR-27, AR-28)

## Implementation Details

### Specification Edits

- `spec/appendix-amiga.md` §1: change the `intuition` / `graphics` row from
  "later" to v1 with the wrapped surface, and add a short subsection describing
  the custom screen and the drawing calls. (AR-27)
- `spec/appendix-stdlib.md`: add a section that documents
  `std.amiga.intuition` (`Screen`, `openScreen`) and `std.amiga.graphics`
  (`setColor`, `fill`, `draw`, `clear`), matching the existing module-section
  style. (AR-27)
- Both edits must keep `spec/check.sh` green (titles, `Status` lines, links, and
  diagnostic codes). (AR-3)

### Corpus Program

`spec/tests/corpus/graphics/program.clo` draws and reports:

```clorraine
// Opens a custom screen, draws colored bars and a diagonal, and reports.
module tests.corpus.Graphics;

import { Screen, openScreen } from std.amiga.intuition;
import { setColor, fill, draw, clear } from std.amiga.graphics;
import { String, stringOf, toCString } from std.text;
import { println } from std.io;
import { delayTicks } from std.amiga.dos;

func main(): i32 {
    let title: String = stringOf("Clorraine graphics");
    let screen: Screen? = openScreen(320, 256, 4, toCString(title));
    if (screen == null) {
        println("graphics: failed");
        return 1;
    }

    setColor(screen, 0, 0, 0, 0);
    setColor(screen, 1, 255, 32, 32);
    setColor(screen, 2, 32, 255, 32);
    setColor(screen, 3, 32, 32, 255);
    clear(screen, 0);

    let i: i32 = 0;
    while (i < 8) {
        fill(screen, i * 40, 0, i * 40 + 39, 255, u8(i % 3 + 1));
        i = i + 1;
    }
    draw(screen, 0, 0, 319, 255, 3);

    println("graphics: ok");
    delayTicks(1500);
    return 0;
}
```

- `spec/tests/corpus/graphics/expected-output.txt`: `graphics: ok` with a final
  newline. (AR-10)
- `spec/tests/corpus/graphics/README.md`: module, entry, profiles, expected
  output, clean-compile contract, construct coverage, warning audit, and the
  hand-check record, matching the existing corpus READMEs. (AR-10, AR-27)

### CI Legs

- `compiler/tests/corpus_spec_test.rs`: add a test that compiles
  `graphics/program.clo` under `--profile host` and asserts the first stderr line
  is `error E11200:` naming `host`, exactly like the existing
  `interrupt-counter` test. (AR-20)
- `docker/scripts/corpus-amiga.sh`: after the `interrupt-counter` section, build
  the graphics binary with `clc --profile hosted-amiga` and `m68k-amigaos-gcc`.
  (AR-21)
- `docker/scripts/corpus-fs-uae.sh`: run the graphics binary with
  `RUN_CAPTURE=1`, wait for the `graphics: ok` sentinel, and diff against
  `spec/tests/corpus/graphics/expected-output.txt`; keep the interrupt-counter
  leg. Factor the shared sentinel-and-diff logic so both programs use it. (AR-21)

### Video Chapter

- The graphics chapter is **emulator-only** (no terminal tape): the visual is
  the screen, not a command line. (PF-009)
- `demo/story.sh`: record the graphics program under FS-UAE with the existing
  `demo/record-emulator.sh` path, add a caption card ("Drawing on a real Amiga"),
  and place the chapter after the existing emulator chapter. (AR-28)

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|---|---|---|
| Corpus program fails to open a screen | Prints `graphics: failed` and exits 1; the golden diff fails | AR-16 |
| FS-UAE produces no sentinel | The leg times out and fails, like the interrupt-counter leg | AR-21 |
| `spec/check.sh` breaks after an appendix edit | The edit is corrected; the check must stay green | AR-3 |

> **Traceability:** Every error-handling strategy and design choice references
> the Ambiguity Register entry that resolved it; see
> `00-ambiguity-register.md`.

## Testing Requirements

- `graphics_spec_test.rs`: the host rejection and the hosted-amiga compile.
  ([07](07-testing-strategy.md) ST-1–ST-5)
- `make verify` runs the FS-UAE graphics leg and diffs the golden. (AR-19, AR-21)
- `demo/story.sh` produces the graphics chapter. (AR-28)
