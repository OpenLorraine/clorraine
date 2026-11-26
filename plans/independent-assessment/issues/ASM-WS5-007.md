<!-- title: [bug] Stop cardPresent and cardStatus removing card.resource -->
<!-- labels: assessment,kind:finding,severity:major,priority:P1,domain:shims,tier:advanced,bug,amiga-sdk -->
<!-- parent: R9 -->

## Summary
After one call, `card.resource` is gone until reboot: the second call reports no PCMCIA, and every later `OpenResource("card.resource")` in the system fails (new users of the PCMCIA slot: carddisk, network and CF drivers being started). No crash was observed; existing users keep their pointer.

| Field | Value |
|---|---|
| Finding | ASM-WS5-007 · DEF-599 |
| Severity / Priority / Rank | Major / P1 / 18 of 58 |
| Domain / Lens | shims / A1200 shipped (A600/A1200 PCMCIA) |
| Confidence / Verdict | verified / confirmed (Major, verified) (independent verification) |
| Effort | S |
| Baseline | `e24a8752b08827c3a5d7c77b6e2827dac708b629` |
| Theme | {THEME} |

## Evidence
Reproduction — probe `probes/WS5/540-card-resource.clo`, runner `54x-fs-uae.sh` (cases `540 540c`). Expected (`540-card-resource.expected`: exec `RemResource` "removes an existing resource from the system resource list. There must be no outstanding users"; `OpenResource` has no close; `cardPresent` "True when card.resource can be opened"): `card true true`, walker `card 1` before and after. Observed on A1200/KS3.1: `card true false`, walker `after … card 0`; the contrast without the calls keeps `card 1`. The walker's resource check fired on the seeded program.

Probe `plans/independent-assessment/probes/WS5/540-card-resource.clo` (baseline `e24a8752`):

```
module Probe.Main;

import { println } from std.io;
import { cardPresent } from std.amiga.expansion;

func main(): i32 {
    let a: bool = cardPresent();
    let b: bool = cardPresent();
    println($"card {a} {b}");
    return 0;
}
```

## Root cause
the code treats `RemResource` as the release that pairs with `OpenResource`.

Location: `compiler/runtime/shim_hosted_amiga_20.c:393-401` and `:406-416` (`RemResource(card)` after `OpenResource`), at e24a8752

## Recommended fix
delete both `RemResource` calls (resources are not closed).

## Acceptance criteria
- The behavior the specification requires: `card true true`, walker `card 1` before and after (source: `540-card-resource.expected`: exec `RemResource` "removes an existing resource from the system resource list. There must be no outstanding users"; `OpenResource` has no close; `cardPresent` "True when card.resource can be opened")
- A regression test that fails without the fix and passes with it

## Verification
- AGENTS.md gates: `make check`; the `make verify` FS-UAE legs for the touched modules (`sdk-a500-*`, `sdk-a600-*`, `sdk-a1200-*`)
- Review dispatch per AGENTS.md for this change surface: correctness reviewer, amiga-platform-reviewer

## Constraints
- Do not change `spec/`; if the fix seems to need a spec change, stop and ask for a ruling.
- Do not put ASM/DEF/plan identifiers or `plans/` paths in source, tests, or doc comments; restate the rationale in plain language (AGENTS.md).
- Plan and execute through CodeOps (`make-plan` or a task plan, then `exec-plan`).

## Related
`examples/amiga/services.clo:50` calls it once, which is why the A600 gate passes. Report: `plans/independent-assessment/10-assessment-report.md`; finding text: `plans/independent-assessment/findings/WS5-shims.md`.
