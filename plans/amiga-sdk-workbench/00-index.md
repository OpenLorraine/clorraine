# Amiga SDK Bundle 8: Workbench and Services Implementation Plan

> **Feature**: Workbench AppIcon/AppWindow/AppMenuItem integration, the icon library, expansion boards, the card resource, a keymap call, and an A1200 Workbench demo on Kickstart 2.0+.
> **Status**: Executing — 0/26 tasks; plan preflighted
> **Created**: 2026-11-05 21:55
> **Implements**: —
> **CodeOps Artifact Schema**: 1

## Overview

Bundle 8 adds the advanced-tier Workbench and service surface that the
Kickstart 2.0+ profiles can actually run from ROM: the Workbench application
objects (`AppIcon`, `AppWindow`, `AppMenuItem`, and `AppMessage`), the icon
library's `DiskObject` round-trip, expansion-board enumeration, the A1200/A600
`card.resource`, and a raw-key keymap call. A Workbench demo starts the
ROM-resident Workbench task, adds the application objects, captures the live
Workbench screen, receives an `AppMessage`, and reports the result on an
A1200/KS3.1 and an A600/KS2.05.

The capabilities that need Workbench-disk libraries or devices (commodities,
locale, ARexx, clipboard, serial/parallel/printer, and `nonvolatile.library`)
are deferred with the empirical ROM evidence in
[02-current-state](02-current-state.md) and [05-findings](05-findings.md). The
advanced baseline in this repository has licensed Kickstart ROMs but no
Workbench media, and the missing libraries and devices are not ROM-resident.
`SDK-WBK-008` is split into the delivered expansion/keymap/card surface and the
deferred `SDK-WBK-010` nonvolatile row.

## Minimum-Sufficient Baseline

**Original goal:** add the Workbench and system-service surface on the
advanced (2.0+/ECS/1M) tier and prove it on a real 2.0 and 3.1 machine.

**Smallest viable design:** keep the existing `MsgPort`, `Window`, and
`Library` handles; add three small modules (`std.amiga.workbench`,
`std.amiga.icon`, `std.amiga.expansion`); add a public-port constructor to
`std.amiga.exec` so the Workbench message path can be exercised
deterministically; start the ROM Workbench task with a test-only fixture that
calls the same private `StartWorkbench` vector `C:LoadWB` uses; add a test-only
AppMessage fixture that posts three real `struct AppMessage`s to the demo's public
port; and gate the result with the native `CSCR` capture already used by the
GUI gate.

**Excluded machinery:** Workbench-disk libraries and devices (commodities,
locale, rexxsyslib, clipboard, serial, parallel, printer), nonvolatile.library,
public `FindPort`/ARexx protocol, BOOPSI, gadtools, ASL, datatypes, and any
language/grammar/diagnostic/ABI change.

**Approved complexity:** the two test-only C fixtures are the minimum needed to
provide a Workbench environment and a deterministic AppMessage without host
input injection. They are not part of the public SDK.

## Document Index

| #   | Document                                                   | Description                                 |
| --- | ---------------------------------------------------------- | ------------------------------------------- |
| AR  | [Ambiguity Register](00-ambiguity-register.md)             | Zero-Ambiguity Gate decisions (audit trail) |
| 00  | [Index](00-index.md)                                        | This document — overview and navigation     |
| 01  | [Requirements](01-requirements.md)                         | Feature requirements and scope              |
| 02  | [Current State](02-current-state.md)                       | ROM/media analysis and existing surface     |
| 03-1 | [Workbench and AppMessage](03-01-workbench-appicon.md)    | AppIcon/AppWindow/AppMenuItem/messages      |
| 03-2 | [Icon Library](03-02-icon-library.md)                     | `DiskObject` read/write/default/type        |
| 03-3 | [Expansion, Card, Keymap](03-03-expansion-card-keymap.md) | Boards, card resource, raw-key call         |
| 03-4 | [Demo and Gates](03-04-demo-and-gates.md)                 | A1200 Workbench and A600 services gates     |
| 05  | [Findings](05-findings.md)                                 | Discovery and closeout findings             |
| 07  | [Testing Strategy](07-testing-strategy.md)                 | Spec cases, impl cases, and verification    |
| 99  | [Execution Plan](99-execution-plan.md)                     | Phases, sessions, and task checklist        |
| PF  | [Preflight Report](00-preflight-report.md)                 | Independent audit and resolutions           |

## Quick Reference

### Usage Examples

```clorraine
import { MsgPort, createPublicPort } from std.amiga.exec;
import { AppIcon, AppMessage, addAppIcon, removeAppIcon,
         waitAppMessage, appMessageType, appMessageId, appMessageUserData,
         replyAppMessage } from std.amiga.workbench;
import { String, stringOf, toCString } from std.text;

func announce(): i32 {
    let name: String = stringOf("Clorraine");
    let port: MsgPort? = createPublicPort(toCString(name));
    if (port == null) { return 1; }
    let icon: AppIcon? = addAppIcon(port, toCString(stringOf("Clorraine")), 7, 0);
    if (icon == null) { return 2; }
    let message: AppMessage? = waitAppMessage(port);
    if (message == null) { return 3; }
    let kind: i32 = appMessageType(message);
    replyAppMessage(message);
    removeAppIcon(icon);
    return kind;
}
```

```clorraine
import { DiskObject, ICON_TOOL, getDefDiskObject, putDiskObject,
         getDiskObject, diskObjectType, freeDiskObject } from std.amiga.icon;

func writeIcon(name: cstring): bool {
    let icon: DiskObject? = getDefDiskObject(ICON_TOOL);
    if (icon == null) { return false; }
    if (!putDiskObject(name, icon)) { freeDiskObject(icon); return false; }
    freeDiskObject(icon);
    let loaded: DiskObject? = getDiskObject(name);
    if (loaded == null) { return false; }
    let kind: i32 = diskObjectType(loaded);
    freeDiskObject(loaded);
    return kind == ICON_TOOL;
}
```

### Key Decisions

| Decision | Outcome |
| -------- | ------- |
| Scope | 4 rows done (001, 002, 008, 009); 6 deferred with ROM evidence (003–007, 010) |
| Workbench start | Test-only fixture calls the private `StartWorkbench` vector, mirroring `C:LoadWB` (AR-3) |
| AppMessage proof | Public port plus a test-only fixture posts three real `struct AppMessage`s (AR-4) |
| Live Workbench proof | Native `CSCR` capture of the Workbench screen with the AppIcon, plus an AppIcon crop golden (AR-5) |
| Icon library | `DiskObject` get/put/default/free/type round-trip on both ROMs (AR-6) |
| Expansion/card | Board enumeration and `card.resource` status on both ROMs (AR-7) |
| Keymap | Library open plus `MapRawKey`; the bare ROM has no loaded keymap, so the demo reports 0 (AR-8) |
| Gate | A1200/KS3.1 Workbench gate plus A600/KS2.05 services gate (AR-9) |

## Specialist Agents

**None** — the capability-gap check found no gap a standing specialist would
close beyond the existing phase review. The bundle adds C shim code, Workbench ownership wrappers, and one profile
`#define` in `compiler/src/codegen.rs`; there is no IR, symbol-name, or
compiler semantic change. The phase review uses the correctness reviewer, the
read-only memory-safety auditor, and the read-only emission-integrity auditor
(the codegen define changes every `hosted-amiga` translation unit).

_Detection evidence: `compiler/runtime/shim_hosted_amiga.c` owns the C
resources; `stdlib/std.amiga.workbench.clo`, `stdlib/std.amiga.icon.clo`, and
`stdlib/std.amiga.expansion.clo` are the wrappers; `compiler/src/profile.rs` gates the modules;
`compiler/src/codegen.rs` adds the `CL_HOSTED_AMIGA_20` profile define; no
`compiler/src/ir` file is touched._

## Related Files

- `stdlib/std.amiga.workbench.clo` — Workbench application objects and messages
- `stdlib/std.amiga.icon.clo` — icon library `DiskObject` surface
- `stdlib/std.amiga.expansion.clo` — expansion, card, keymap surface
- `stdlib/std.amiga.exec.clo` — public port constructor
- `compiler/runtime/shim_hosted_amiga.c` — C shim implementations
- `compiler/src/profile.rs`, `compiler/src/stdlib.rs` — profile gates and exports
- `compiler/src/codegen.rs` — the `CL_HOSTED_AMIGA_20` profile define
- `compiler/tests/amiga_workbench_spec_test.rs` — specification suite
- `compiler/tests/amiga_workbench_impl_test.rs` — implementation suite
- `examples/amiga/workbench.clo`, `examples/amiga/services.clo` — acceptance programs
- `docker/scripts/workbench-start.c`, `docker/scripts/workbench-post.c` — test-only fixtures
- `docker/scripts/a1200-workbench-smoke.sh`, `docker/scripts/a600-services-smoke.sh` — gates
- `docs/amiga-sdk/a1200-workbench.png` — Workbench screen golden
- `docs/amiga-sdk/a1200-workbench-icon.png` — live AppIcon crop golden
- `spec/appendix-amiga.md`, `spec/appendix-stdlib.md` — frozen appendices
