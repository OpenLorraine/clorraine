# Requirements: Amiga shim crash-safety

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

A program built only from safe Clorraine code must not be able to crash, hang, or corrupt a real
Amiga through these stdlib entries: `showAlert`, `savePublicScreen`, `drawText`, `clipboardRead`,
`freeRegion`/`freeLayer` with an installed clip region, and `closeAmigaGuide` with held messages.

Source: theme issue #44 (remediation roadmap R5). Findings: ASM-WS5-002 (#59, DEF-587),
ASM-WS5-004 (#57, DEF-585), ASM-WS5-016 (#60, DEF-588), ASM-WS5-006 (#65, DEF-593). No RD
upstream; this document is the owning requirements document. Scope per AR-1.

## Functional Requirements

### Must Have

- [ ] **FR-1** `showAlert` and `savePublicScreen` work as the first Intuition call in a program:
      the base is opened on demand; when Intuition cannot be opened, `showAlert` does nothing and
      `savePublicScreen` returns false (AR-2). Spec: [03-01 §Intuition](03-01-intuition-and-clipboard.md#intuition-on-demand).
- [ ] **FR-2** `drawText` never writes outside the screen's bitmap. Text that fits is drawn exactly
      as today; text that is partly outside is clipped at the pixel edge; text that is fully
      outside, out of range, or a partly visible colour-font string draws nothing (AR-3..AR-5).
      Spec: [03-02](03-02-text-clipping.md).
- [ ] **FR-3** `clipboardRead` finishes the device read before it returns, so a later write on any
      handle of that unit is not held off. It still returns the caller-buffer count or -1 and
      writes at most `len(buf)` bytes (AR-10, AR-11). Spec: [03-01 §Clipboard](03-01-intuition-and-clipboard.md#clipboard-read).
- [ ] **FR-4** Freeing a region uninstalls it from every layer that holds it before disposing it;
      freeing a layer (by the program or at exit) ends an open update, then uninstalls its clip
      region; the program's own update calls keep today's behaviour (AR-6..AR-8). Spec:
      [03-03 §Regions](03-03-borrowed-objects.md#regions).
- [ ] **FR-5** Closing an AmigaGuide context replies every message the program still holds; a held
      message then reads 0 from its getters and its reply or drop only frees the handle (AR-9).
      Spec: [03-03 §AmigaGuide](03-03-borrowed-objects.md#amigaguide-messages).
- [ ] **FR-6** Target evidence: a `vamos` leg, the A1200/KS3.1 `sdk-a1200-addons` gate, one
      clipboard probe in the A600/KS2.05 `sdk-a600-addons` gate, and the A500/KS1.3
      `sdk-a500-fault-cleanup` gate carry probes that fail before and pass after the fix, except
      where AR-13 records a justified guard (AR-13, AR-14). Spec: [03-04](03-04-gates-and-docs.md).

### Should Have

- [ ] **FR-7** The stdlib doc comments and `docs/known-limitations.md` describe the new behaviour
      (AR-16).

### Won't Have (Out of Scope)

- `fill`/`draw` on a screen, `clipboardWrite`'s failed-write path, the public-screen lock across
  a Ctrl-C, and a layer that outlives its bitmap (register §Out-of-scope observations; AR-1).
- ASM-WS5-019 (16-bit text length, `layerBeginUpdate` FALSE pairing, the other grouped items) and
  ASM-WS5-023 (raw clips discarded), owned by R9.
- Any `spec/` change (AR-12); any stdlib signature change.

## Technical Requirements

### Performance
- Text that fits keeps its current drawing calls; it pays only the metric walk that classifies
  it (one pass over the characters, no allocation, no extra blit). Only a clipped call pays the
  temporary raster and two blits (AR-4). Programs that call none of the changed entries do not grow; the release `.text`
  delta of a `drawText` user and a layer user is recorded (AR-15).

### Compatibility
- Kickstart 1.3 (`hosted-amiga-13`) for `showAlert`, `drawText`, and layers; Kickstart 2.0+ for
  `savePublicScreen`, regions, AmigaGuide, and the clipboard. Every graphics and layers call used
  exists in the 1.3 protos (AR-4). The clipboard read is proven on the 2.04 and 3.1 devices
  (AR-13).
- Emitted C stays valid C99 and builds under `-Werror` with both CRTs and the vbcc leg.

### Security
- The safe stdlib surface is the memory-safety boundary: no change may let a safe program write
  outside memory it owns or reach freed OS memory (FR-1..FR-5).

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
| -------- | ------------------ | ------ | --------- | ------ |
| Scope | four findings / wider sweep | four findings | issue #44 | AR-1 |
| Off-screen text | reject / truncate / clip | pixel clip | issue goal "clip screen text" | AR-3 |
| Region freed while installed | uninstall / defer | uninstall | keeps "freeRegion disposes" | AR-6 |
| Guide closed with held messages | reply at close / defer close | reply at close | keeps "replies exactly once" | AR-9 |
| Spec | change / refine | refine | undefined paths only | AR-12 |
| Gates | new / extend | extend two | smallest | AR-13 |

> **Traceability:** every scope decision references the Ambiguity Register entry that resolved it.

## Acceptance Criteria

1. [ ] ST-1..ST-9 fail before and pass after their fix, except the guards AR-13 names (ST-5 on
       the A600, ST-7's layer-free half); ST-10 passes after the fix (AR-13).
2. [ ] The existing legs of `sdk-a1200-addons`, `sdk-a600-addons`, and `sdk-a500-fault-cleanup`,
       and every other gate in `make verify`, still pass (ST-11).
3. [ ] `make verify` passes; `make bench-compare` passes with an empty `REQUIRE`; deltas recorded.
4. [ ] Doc comments and known-limitations updated (FR-7).
