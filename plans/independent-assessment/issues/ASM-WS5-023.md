<!-- title: [bug] Make clipboardWrite store data the clipboard device keeps -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:core,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
`clipboardWrite` returns true while a raw (non-IFF) clip is either discarded (no clip yet) or stored padded to 8 bytes (after an IFF clip exists); `clipboardRead` of raw data returns 0 or the padded clip. The clipboard device keeps only IFF clips; the known-limitations entry attributes the empty read to the test environment.

| Field | Value |
|---|---|
| Finding | ASM-WS5-023 · DEF-595 |
| Severity / Priority / Rank | Major / P1 / 14 of 58 |
| Domain / Lens | shims / A1200 shipped (`hosted-amiga`, `std.amiga.clipboard`); other 2.0+ versions not tested |
| Confidence / Verdict | verified / confirmed (Major, verified) with a corrected impact and root cause (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction on A1200/KS3.1 with the licensed Workbench 3.1 `Devs/clipboard.device` and `Assign CLIPS: RAM:` (runner `probes/WS5/verify-9.2.4/run-a1200.sh`, sources `clipiff.c`, `clipdump.c`, `clipw.clo`, `clipr.clo`, `clipc.clo` in the same folder). A C control writing with the shim's exact sequence (`io_ClipID = 0`, `CMD_WRITE`, `CMD_UPDATE`): raw 4 bytes → `write 0 update 0 id 2`, then an autodoc-conformant dump reads `actual 0 id -1`; the same bytes wrapped as `FORM…FTXT CHRS` → the dump reads 24 bytes and the shim's `clipboardRead` returns 16. With the shim, `clipw.clo` prints `second program write true`, yet the following dump reads 0 bytes; `clipc.clo` (write, write, read) reads `r2 0`.

Phase 9 notes: impact — `clipboardWrite` returns true while a raw clip is either dropped (no clip yet) or stored altered (padded to 8 bytes after an IFF clip exists); root cause — the device treats clips as IFF and has no defined behavior for non-IFF data; the known-limitations entry (`docs/known-limitations.md:62-64`) misattributes this symptom to the environment.

Probe `plans/independent-assessment/probes/WS5/verify-9.2.4/clipw.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;
import { slice } from std.mem;
import { Clipboard, openClipboard, clipboardWrite } from std.amiga.clipboard;

func main(): i32 {
    let opened: Clipboard? = openClipboard(0);
    if (opened == null) {
        println("open failed");
        return 1;
    }
    let c: Clipboard = opened;
    let a: u8[4] = [105, 106, 107, 108];
    let w: bool = clipboardWrite(c, slice(a, 0, 4));
    println($"second program write {w}");
    return 0;
}
```

## Root cause
the wrapper treats the clipboard as a raw byte store; the device keeps only IFF clips, and its success codes do not report a discarded clip.

Location: `compiler/runtime/shim_hosted_amiga_optional_services.c:658-685` (`cl_clipboard_write`: raw `CMD_WRITE` of the caller's bytes, then `CMD_UPDATE`, returns TRUE), `stdlib/std.amiga.clipboard.clo:34-40` ("Writes the whole clip and marks it current; true on success"), at e24a8752

## Recommended fix
wrap text in an IFF `FTXT`/`CHRS` clip on write and unwrap on read (or expose an IFF-level API); make the gate read the clip back from a second program; or document that raw clips are unsupported and return false.

## Acceptance criteria
- The behavior stated under Recommended fix, checked against the cited spec section or autodoc
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
ASM-WS5-016; ASM-WS8-002. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
