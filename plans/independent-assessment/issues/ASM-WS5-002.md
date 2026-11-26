<!-- title: [bug] Open intuition.library before savePublicScreen and showAlert use it -->
<!-- labels: assessment,kind:finding,severity:critical,priority:P0,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R5 -->

## Summary
Any program that calls `savePublicScreen` (the Workbench capture API) or `showAlert` before opening a screen or window crashes the machine. `showAlert` with no window is the natural "fatal error" path of a CLI tool.

| Field | Value |
|---|---|
| Finding | ASM-WS5-002 · DEF-587 |
| Severity / Priority / Rank | Critical / P0 / 6 of 58 |
| Domain / Lens | shims / A1200 shipped (`savePublicScreen`, reproduced); A500 (`showAlert`, reproduced in Phase 9 on A500/KS1.3); A1200 (`showAlert`, code reading) |
| Confidence / Verdict | verified / confirmed (Critical, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe `probes/WS5/542-null-intuition-base.clo`, runner `54x-fs-uae.sh` (commands as in ASM-WS5-001, cases `542 542c`). Expected (`542-null-intuition-base.expected`: `std.amiga.workbench` "Returns false for a null/unknown screen"; intuition `LockPubScreen` returns NULL for an unknown name): `calling`, `saved false`, `rc 0`, `END`. Observed on A1200/KS3.1: 542c (a screen opened first) prints exactly that; 542 (no screen yet) never reaches `END` within 180 s and loses its buffered output — the machine crashed or hung. The link map (`build/tmp/assessment/WS4/size/sieve.clo.map`) shows `IntuitionBase` in the program's own `.bss` and only `libstubs.a(dos.o)` linked, so no libnix auto-open covers it.

Probe `plans/independent-assessment/probes/WS5/542-null-intuition-base.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;
import { Screen, openScreen } from std.amiga.intuition;
import { savePublicScreen } from std.amiga.workbench;

func main(): i32 {
    println("calling");
    let ok: bool = savePublicScreen("ClNoSuchScreen", "RAM:probe542");
    println($"saved {ok}");
    return 0;
}
```

## Root cause
the shim defines its own `IntuitionBase` (which suppresses the libnix auto-open object) and opens it lazily only inside the screen/window constructors; other entry points that use Intuition assume an earlier constructor ran.

Location: `compiler/runtime/shim_hosted_amiga.c:1295` (`struct IntuitionBase *IntuitionBase = NULL;`), `:1307-1311` and `:1600-1604` (the only opens: screen and Workbench-window creation), `:2617-2641` (`cl_show_alert` → `DisplayAlert`), `shim_hosted_amiga_20.c:484-496` (`cl_save_public_screen` → `LockPubScreen`), at e24a8752

## Recommended fix
one `cl_intuition_open()` helper (and `cl_graphics_open()`) called at the top of every entry that uses the base, released at exit (see ASM-WS5-013); a test that calls each Intuition entry first in a fresh program.

## Acceptance criteria
- The behavior the specification requires: `calling`, `saved false`, `rc 0`, `END` (source: `542-null-intuition-base.expected`: `std.amiga.workbench` "Returns false for a null/unknown screen"; intuition `LockPubScreen` returns NULL for an unknown name)
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS5-013; WS5-14 pair list (Intuition opened in two places, missing in a third). Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
