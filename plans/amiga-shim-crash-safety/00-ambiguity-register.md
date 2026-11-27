## Ambiguity Register: amiga-shim-crash-safety

> **Status**: ✅ GATE PASSED — all 16 items resolved (AR-1 by the user's invocation; AR-2..AR-16 delegated by `--auto-design`, provenance below; no reserved or complexity row)
> **Last Updated**: 2026-11-27 03:20

Planning target: theme issue #44 (remediation roadmap R5) — findings ASM-WS5-002 (#59, DEF-587),
ASM-WS5-004 (#57, DEF-585), ASM-WS5-016 (#60, DEF-588), ASM-WS5-006 (#65, DEF-593).
Context artifacts: `plans/independent-assessment/findings/WS5-shims.md`, its probes under
`plans/independent-assessment/probes/WS5/`, `spec/appendix-stdlib.md`, `spec/appendix-amiga.md`,
the NDK 3.2 autodocs and NDK 1.3 headers vendored under `build/distribution/reference-20261117/`,
the hosted-amiga shims, and the completed `plans/amiga-fault-exit-cleanup/` registry.
Modification set: this plan folder and `plans/00-roadmap.md` only. No `spec/` change.

Mode: auto-design (policy version 1, root invocation `ad-f7e2786b995e`). Delegated categories:
algorithms, internal interfaces, failure and recovery design, testing strategy, implementation
sequencing. Reserved: product scope, public API changes, spec changes, complexity escalation.

| # | Category | Ambiguity / Gap | Options Presented | User Decision | Status |
|---|----------|-----------------|-------------------|---------------|--------|
| 1 | Scope | Which defects this plan closes | the four findings of issue #44 / a wider shim sweep | Fixed by the user's invocation of `make-plan amiga-shim-crash-safety`, whose scope is issue #44 and roadmap R5: the four findings, no more | ✅ Resolved |
| 2 | Technical | How `showAlert` and `savePublicScreen` get an Intuition base | open on demand through `cl_lib_open` / open at startup / a new helper | AI — delegated: open on demand through the existing `cl_lib_open` in the two entries | ✅ Resolved |
| 3 | Behavioral | What `drawText` does with a text box partly or fully outside the screen | draw nothing unless fully inside / drop whole glyphs / clip at the pixel edge | AI — delegated: clip at the pixel edge; fully outside draws nothing | ✅ Resolved |
| 4 | Technical | How the pixel-edge clip is drawn on Kickstart 1.3 | one-plane mask + `BltTemplate` / two-plane render + two `BltTemplate`s / copy-back bitmap / temporary layer | AI — delegated: two-plane render of the culled run, then `BltTemplate`s | ✅ Resolved |
| 5 | Edge cases | `drawText` limits: the box rule, coordinate range, count range, screen bounds, colour fonts, styles | exact metrics / conservative superset; per limit below | AI — delegated: conservative superset box and the limit set in the notes | ✅ Resolved |
| 6 | Behavioral | `freeRegion` on an installed region; freeing a layer with an installed region | uninstall then dispose / defer the dispose / copy the region | AI — delegated: uninstall from every layer, then dispose; a layer installs NULL before `DeleteLayer` | ✅ Resolved |
| 7 | Technical | How the region code finds the layers that hold it | per-region list + function pointer / walk the registry and read `Layer.ClipRegion` | AI — delegated: walk the registry for layer records and compare `ClipRegion` | ✅ Resolved |
| 8 | Edge cases | Uninstall or delete while a layer is between `layerBeginUpdate` and `layerEndUpdate` | end the update first / accept the layers BUGS case | AI — delegated: the layer records an open update; uninstall and delete end it first | ✅ Resolved |
| 9 | Behavioral | `closeAmigaGuide` with messages the program still holds | reply at close and orphan the handles / keep the context open / reply on receipt | AI — delegated: reply each held message, then close; orphans read 0 and their reply only frees the handle | ✅ Resolved |
| 10 | Technical | How `clipboardRead` finishes the read | read on until 0 (RKM loop) / skip read with `io_Data` NULL then one past-the-end read | AI — delegated: the autodoc's skip read, then exactly one read issued past the end | ✅ Resolved |
| 11 | Behavioral | `clipboardRead`'s result when a finishing read fails, and the retry scope | return the data read / return -1 / retry the whole read | AI — delegated: return the caller-buffer count; finishing errors are ignored; retries stay on the first read | ✅ Resolved |
| 12 | Scope | Do the new behaviours change the frozen spec? | a conforming refinement of undefined or misuse paths / a spec change needing a ruling | AI — delegated: conforming refinement, no spec change, docs outside `spec/` updated | ✅ Resolved |
| 13 | Technical | Where the target evidence runs, and what makes each probe red | new gates / extend `sdk-a500-fault-cleanup` and `sdk-a1200-addons` | AI — delegated: extend both gates; probe oracles in the notes | ✅ Resolved |
| 14 | Technical | How the extended A1200 gate knows its probes finished | file exists + 1 s / end marker + settle / tree unchanged | AI — delegated: an end-marker file written last, then the bounded settle | ✅ Resolved |
| 15 | Non-functional | Size acceptance | bench-compare only / bench-compare plus a recorded delta for API users / none | AI — delegated: bench-compare with empty `REQUIRE` plus recorded `.text` deltas for `gui` and `scroller` | ✅ Resolved |
| 16 | Feature gaps | Which user-facing docs change | doc comments and known-limitations / none | AI — delegated: the six stdlib doc comments and `docs/known-limitations.md` | ✅ Resolved |

## Coverage Map

> **Coverage**: 4/4 clusters · 12/12 categories · 30 candidates (14 material after dedupe, 0 optional, 0 complexity) · 6 residual limits · Effort: high/high/high/xhigh→unavailable (fail-open) · Independence: uniform-effort
> **Freshness**: Coverage as of 2026-11-27; artifacts: packet D2–D7, shims at `9cef1491`, spec appendices, NDK 3.2 autodocs, NDK 1.3 headers

| Cluster | Area | Examined | Not examined / limit | Candidates | Notes / limit | Confidence |
|---------|------|----------|----------------------|------------|---------------|------------|
| C1 Product and scope | stdlib docs, spec rows, the four findings | the six stdlib modules, spec §14.9/14.18/14.22/§21, shims | library autodocs | 6 → AR-5, AR-11, AR-12, AR-13, AR-16 | CV-005 colour fonts → AR-5 | medium |
| C2 Behavior and edges | registry/exit order, text, regions, guide, clipboard | shims, NDK 3.2 `layers.doc`/`clipboard.doc`/`graphics.doc` | no target runs; V34 behaviour from BUGS notes only | 11 → AR-5, AR-6, AR-8, AR-10, AR-11, AR-13; 2 out of scope | CV-009 write path, CV-010 public-screen lock → out of scope (notes) | medium |
| C3 Technical and data | V33/V34 availability, ink box, temp raster, clipboard, vamos | NDK 1.3 protos and `text.h`, NDK 3.2 autodocs, amitools `vamos` config | V34 ROM, layers/amigaguide internals | 6 → AR-4, AR-5, AR-10, AR-13 | BltTemplate, InstallClipRegion, BltMaskBitMapRastPort in the 1.3 protos | medium |
| C4 Risk and stakeholders | memory safety, size, chip, gates, goldens | gate scripts, examples, bench harness, verifier probes and verdicts | no target runs | 7 → AR-4, AR-5, AR-10, AR-13, AR-14, AR-15, AR-12 | the reproduced clipboard trace read 0 bytes (`verdicts-9.2.4.md:31`) | medium-high |

**Source note:** AR-4 ← C3 CV-003, C2 CV-004, challenger D3; AR-5 ← C2 CV-001/002/003/005, C3 CV-001/002/004, C4 CV-001/002, C1 CV-005; AR-6/7 ← challenger D4; AR-8 ← C2 CV-007; AR-9 ← challenger D5; AR-10 ← C2 CV-008, C3 CV-005, C4 CV-003, challenger D6; AR-11 ← C1 CV-004; AR-12 ← C1 CV-003/006, C4 CV-006; AR-13 ← C1 CV-001, C2 CV-011, C3 CV-006, C4 CV-004; AR-14 ← C4 CV-005; AR-15 ← C4 CV-007; AR-16 ← C1 CV-002. No Discovery `DQ-*` keys.

**Residual limits:** (1) Kickstart 1.3 `Text()` behaviour is known only from the V47 autodoc BUGS notes; (2) layers behaviour after a failed `InstallClipRegion` before V45 is undocumented; (3) AmigaGuide's rules for messages outstanding at `CloseAmigaGuide` are undocumented; (4) no cluster ran a target; (5) the RKM `cbio.c` loop is recalled, not vendored; (6) `showAlert` with Intuition available has no automated target run (spec appendix-stdlib.md:286-288: the headless harness cannot focus the alert screen).

### Out-of-scope observations (strict scope; not planned)

Reported for separate tracking; none is one of the four findings, and AR-1 excludes them:
- `fill` and `draw` (`cl_fill`/`cl_draw`, `shim_hosted_amiga.c:1697-1715`) pass raw coordinates to
  `RectFill`/`Draw` on the same unlayered screen `RastPort`. The verifier's `tfill.clo` contrast
  completed, so whether graphics clips them is unverified.
- `cl_clipboard_write` returns FALSE after a failed `CMD_WRITE` without `CMD_UPDATE`
  (`shim_hosted_amiga_optional_services.c:782-784`); whether that holds readers off is unverified.
- `cl_save_public_screen` holds a public-screen lock across libnix stdio; a Ctrl-C noticed by stdio
  there exits without `UnlockPubScreen` (`shim_hosted_amiga_20.c:540-546`).
- A layer can outlive its borrowed bitmap (C2 note).

### Resolution Notes

Every delegated row below carries: Authority: AI — delegated by --auto-design · Policy version: 1 ·
Root invocation ID: ad-f7e2786b995e. Hardening: one blind design challenger (D3–D6) and four
coverage clusters; their verdicts are cited per row.

**AR-1:** Scope baseline. The theme issue #44 lists exactly these four findings; the roadmap
entry R5 depends on R1 (`amiga-fault-exit-cleanup`, done) for the registry. Other shim defects
(ASM-WS5-019, the R9 protocol fixes) stay with their own plans.

**AR-2:** Eligibility: internal interface (how an entry obtains a library base); no API or
behaviour change on the correct path. Objective: no Intuition call through a NULL base.
Decision: `cl_show_alert` calls `cl_lib_open(&IntuitionBase, "intuition.library", 33)` and returns
(no-op) on NULL; `cl_save_public_screen` does the same and returns FALSE on NULL or when
`lib_Version < 36` (`LockPubScreen` is V36). Evidence: the shim's own `IntuitionBase`
(`shim_hosted_amiga.c:1621`) suppresses libnix's auto-open; only `cl_open_screen_mode` (`:1643`)
and `cl_open_workbench_window` (`:1949`) open it; a scan of every exported entry found no other
handle-less Intuition user, and every handle-less graphics entry already opens `GfxBase` (C1, C3
confirmed); `cl_screen_save_native` makes no graphics call (`:3043-3144`); `IntuitionBase` is
already one of the 20 `cl_lib_open` bases (`:346`), so no table slot is added. Verified before
planning: under `vamos -O intuition.library=mode:off` both probes crash calling through A6=0
(rc 120) on the current tree. Rejected: opening at startup (links Intuition into every program,
against the section-GC size rule); a new helper (one call each is enough). Strongest
counterargument: a no-op `showAlert` hides a failed alert; but the failure is an absent
Intuition, and the spec defines no error channel for `void showAlert`. Confidence: High.
Reopen triggers: another handle-less Intuition entry appears; the table reaches its limit.

**AR-3:** Eligibility: failure and recovery design inside the confirmed goal — issue #44's goal
text is "clip screen text". Objective: no write outside the screen bitmap; the visible part of a
partly visible string looks as it would on a larger screen. Decision: pixel-edge clip; a box
fully inside keeps today's `SetFont`/`SetAPen`/`Move`/`Text` unchanged; fully outside draws
nothing. Evidence: `cl_text_draw` (`:6695-6713`) draws on `&screen->RastPort`, which has no layer;
`windowText` and `layerText` clip through layers, so pixel clipping matches the sibling APIs; the
scroller is the finding's motivating case. Rejected: draw-nothing-unless-inside (partly visible
status text vanishes); whole-glyph truncation (edge glyphs pop instead of sliding, wrong for a
scroller). Challenger: Agree. Strongest counterargument: more code than truncation (AR-4 bounds
it). Confidence: High. Reopen trigger: the user prefers truncation.

**AR-4:** Eligibility: algorithm. Decision: for a partly visible box, cull to the run of
characters whose superset boxes (AR-5) meet the visible span; `Text()` that run into a cleared
two-plane chip raster through a temporary `RastPort` that copies the screen `RastPort`'s font,
`DrawMode`, `AlgoStyle`, and `TxSpacing`, with APen 3 and BPen 2. Plane 1 then marks every pixel
`Text()` touched and plane 0 every pixel it drew in the APen. Unless the mode has `COMPLEMENT`,
`BltTemplate` plane 1 in JAM1 with the screen's BPen, then plane 0 in JAM1 with the screen's APen,
both over the visible sub-rectangle only; with `COMPLEMENT`, `BltTemplate` plane 0 in
`COMPLEMENT`. `WaitBlit()` before freeing the raster; allocate it `MEMF_CHIP | MEMF_CLEAR` and
free it per call. Allocation failure draws nothing. Evidence: `BltTemplate`, `AllocRaster`, and
`BltMaskBitMapRastPort` are in the 1.3 protos (`ndk13 clib/graphics_protos.h:58,137,161`); BltTemplate
draws "in the current color and drawing mode" (`graphics.doc:1973`), so one plane cannot carry
the JAM2 background (C3 CV-003); `WaitBlit` must precede freeing blitter memory (`graphics.doc`
WaitBlit); the shim already does `WaitBlit` before frees (`:4769,4944,4976`). Rejected: one-plane
mask (wrong JAM2 background, C2 CV-004); copy-back full-depth bitmap (reverts Intuition rendering
between the blits unless `LockLayerInfo` is held — challenger); a temporary layer (clears its
area on 1.3, `spec/appendix-stdlib.md:1330` observation); a fake `ClipRect` (graphics internals).
Strongest counterargument: two blits per clipped call cost time in a scroller; only clipped
calls pay it, and fully inside calls are unchanged. Confidence: Medium-high. Reopen trigger: the
A500 capture comparison (ST-10) shows a visible difference from the reference.

**AR-5:** Eligibility: algorithm, failure design. Decision:
- **Box:** a conservative superset of every pixel `Text()` can touch: per character, the union of
  the readings of `tf_CharKern` (when non-NULL), `tf_CharSpace` (when non-NULL) or `tf_XSize`,
  `tf_CharLoc` width, and `rp->TxSpacing`; codes outside `tf_LoChar..tf_HiChar` use the default
  glyph (index `HiChar - LoChar + 1`); an `FPF_REVPATH` font walks right to left; algorithmic
  bold adds `tf_BoldSmear`, italic adds the font height. Vertically: `y - tf_Baseline` to
  `y - tf_Baseline + tf_YSize - 1`. Exactness inside the raster comes from `Text()` itself.
- **Bounds:** the visible rectangle is `0..min(screen->Width, BytesPerRow*8)-1` by
  `0..min(screen->Height, Rows)-1` of the `RastPort`'s bitmap.
- **Ranges:** all arithmetic in `LONG`; `x` or `y` outside -32768..32767, or a count above 32767,
  draws nothing.
- **Raster size:** the culled run's box is at most the visible width plus two glyph boxes; a run
  wider than 1008 px on graphics below V36, or wider than the visible width plus 512, draws
  nothing.
- **Colour fonts** (`FSF_COLORFONT`): drawn only when fully inside; a partly visible colour-font
  string draws nothing.
Evidence: TextExtent is V36+ and off by one for proportional fonts before V47; V34 `Text()` clips
trailing ink (`graphics.doc:6356,6403`); `Move`/`Text`/`BltTemplate` take 16-bit values
(`graphics.doc` synopses); the V34 1008-pixel `Text()` limit; `ColorTextFont` exists in the 1.3
headers (`text.h:149`) and needs `ctf_Depth` planes. Rejected: exact ROM metrics (unverifiable on
1.3, C3 CV-002); a whole-string raster (unbounded chip and 16-bit wrap, C4 CV-001). Strongest
counterargument: colour-font strings vanish at the edge; colour fonts are rare and the limit is
documented (AR-16). Confidence: Medium-high. Reopen trigger: a superset case misses an ink pixel.

**AR-6:** Eligibility: failure and recovery design on a documented misuse path (spec §14.9 tells
the program to clear the clip first; the runtime now does it). Decision: `freeRegion` installs
NULL in every layer whose `ClipRegion` is this region, then disposes it; `freeLayer` (and its
exit release) installs NULL before `DeleteLayer` when `ClipRegion` is non-NULL. After a
`freeRegion` the layer is unclipped. Evidence: layers autodoc "MUST call InstallClipRegion(l,NULL)
before calling DeleteLayer"; `cl_free_region` (`optional.c:999-1011`) and `cl_layer_free`
(`shim_hosted_amiga.c:6789-6813`) skip it; "Installing NULL will always work" (`layers.doc`). The
exit hook releases newest first, so both creation orders run through these paths. Rejected:
deferred dispose (breaks "freeRegion disposes the region", leaves a clip the program cannot
reach, and a red test cannot tell it from today's bug); copying on install (breaks "layer borrows
region"). Challenger: Simplify (adopted as AR-7). Strongest counterargument: the program loses its
clip silently; the spec already says to clear it before freeing. Confidence: High. Reopen
trigger: a user ruling that the clip must survive.

**AR-7:** Eligibility: internal interface. Decision: an exported base helper walks the registry
for records whose release function is `cl_layer_free` and installs NULL where `Layer.ClipRegion`
equals the region; no per-region list and no function pointer. Evidence: the precedent
`cl_own_ports_hold` walks the registry by release function (`shim_hosted_amiga.c:3409-3420`);
every layer is registered (`:6758,6781`); reading `ClipRegion` reflects the real state even after
a failed install, which cannot be detected before V45 (`layers.doc` NOTES). On 1.3 no region code
calls the helper, so section GC drops it. Rejected: the intrusive list (state can disagree with
`ClipRegion` after a failed install, C2 CV-006). Strongest counterargument: couples region code to
the registry; the coupling already exists for ports. Confidence: Medium-high. Reopen trigger: a
layer kind that is not registered.

**AR-8:** Eligibility: failure design. Decision: `cl_layer` records that `layerBeginUpdate`
returned TRUE; `layerEndUpdate` clears it; the uninstall in AR-6/AR-7 and `cl_layer_free` call
`EndUpdate(layer, FALSE)` first when the flag is set. Evidence: layers BUGS: InstallClipRegion
while updating "may erraneously insert cliprects… discouraged"; `cl_layer_begin_update` keeps no
state (`:6911-6929`); a fault inside an update block reaches the exit hook. The FALSE-return
pairing (ASM-WS5-019a) stays out of scope. Rejected: accepting the BUGS case. Confidence: Medium.
Reopen trigger: the region probe shows damage after an update.

**AR-9:** Eligibility: failure and recovery design within issue #44's goal ("track … guide
messages"). Decision: each message handle stores its guide; `closeAmigaGuide` walks the registry
for `cl_guide_reply` records of this guide, replies each (`ReplyAmigaGuideMsg`), clears its message
and guide pointers (orphan), then calls `CloseAmigaGuide`; an orphan's getters return 0 and its
reply or drop frees only the handle; the orphan stays registered. Evidence: the getters and reply
already treat a NULL message as 0 / no reply (`optional_net.c:283-319`); messages are newer than
their guide, so the exit order already replies first; spec §14.22 "replyGuideMessage replies
exactly once" holds (the close performs the one reply). Rejected: keeping the context open until
the last reply (the guide window stays after close; weak red test); replying on receipt (reply
would reply nothing). Challenger: Agree on behaviour, mechanism B adopted. Confidence: Medium-high.
Reopen trigger: AmigaGuide rejects a reply at close.

**AR-10:** Eligibility: algorithm following the device protocol. Decision: after the caller's
`CMD_READ` succeeds, save `io_Actual`; if it differs from `io_Length`, `io_Offset` is already past
the end, so issue one `CMD_READ` with `io_Data` NULL and `io_Length` 1; otherwise issue one skip
`CMD_READ` with `io_Data` NULL and `io_Length` 0x7FFFFFFF (which leaves the offset past the end),
then that same one-byte NULL read. `io_Offset` and `io_ClipID` are never reset between these
reads. The `CBD_CURRENTREADID` step stays (spec §14.18). Evidence: `clipboard.doc` CMD_READ: "If
io_Data is null, then io_Offset is incremented… useful to skip to the end of file by using a huge
io_Length"; "When io_Offset is beyond the end of the clip, this acts as a signal… that the
application is through reading"; "io_Offset updated to next read position, which is beyond EOF if
io_Actual != io_Length". The reproduced hang read 0 bytes (`verdicts-9.2.4.md:31`), where the RKM
`while (io_Actual)` loop issues nothing (C4 CV-003, C2 CV-008, C3 CV-005). Rejected: the RKM loop
(misses that trace); a scratch-buffer loop (more I/O, same rule). Strongest counterargument: the
NULL skip is unproven on the 2.0 device; the A1200/KS3.1 probes (ST-5, ST-6) are the proof.
Confidence: Medium. Reopen triggers: ST-5 or ST-6 still hangs after the fix (then also try a
zero `io_ClipID` first read, the autodoc's initial-read rule, and stop for a ruling if that
changes spec §14.18's "obtains the current read ID").

**AR-11:** Eligibility: failure design; the contract "returns the count, or -1" is kept.
Decision: the return value is the caller read's `io_Actual`; the finishing reads' errors are
ignored (the data was delivered); the bounded `CBERR_OBSOLETEID` retry covers only the caller
read; the caller's buffer receives at most `len(buf)` bytes. Evidence:
`optional_services.c:796-821`; `std.amiga.clipboard.clo:42`; spec §14.18. Confidence: High.

**AR-12:** Eligibility: interpretation of the frozen spec for paths it leaves undefined. Decision:
no spec change. The `drawText` row (`appendix-stdlib.md:1300`) describes drawing on the screen
`RastPort`; off-bitmap behaviour was undefined memory corruption, and fully inside calls keep the
row's exact mechanism. §14.9 makes clearing the clip the program's duty; the runtime performing it
on misuse keeps "freeRegion disposes the region". §14.18 "reads at most the caller's buffer"
holds: the finishing reads copy nothing. §14.22 "replies exactly once" holds. Evidence: the
quoted rows. Confidence: Medium-high. Reopen trigger: a reviewer or the user reads a row as
exhaustive; then stop for a spec ruling (reserved).

**AR-13:** Eligibility: testing strategy. Decision: `vamos` spec tests ST-1/ST-2 with
`-O intuition.library=mode:off`; the A1200/KS3.1 probes ST-4..ST-9 run in `sdk-a1200-addons`
after its three existing legs; the A500/KS1.3 probe ST-10 runs in `sdk-a500-fault-cleanup` with a
native capture comparison and the `chip` column. Oracles: ST-1/2/4 crash on the base (verified
for 1/2, reproduced by the verifier for 4); ST-5/6 hang on the base (reproduced trace); ST-7
compares `bitmapChecksum` with an unclipped twin (a dangling clip still clips on the base); ST-8
reads a getter after close (stale or crash on the base); ST-9 is the verifier's 200-call hang on
A1200; ST-10 compares the clipped capture with the reference drawn fully inside, shifted — it may
pass on the base, because the base also draws the visible pixels, so ST-9 carries the required
red for the text fix and ST-10 guards the new clip path. Evidence: `vamos` options parse as
`<lib>=<key>:<value>` (amitools `vamos/cfg/libs.py:44`); vamos lacks `BltTemplate`, `AllocRaster`,
and `clipboard.device` (C3); the A1200 gate already installs Workbench 3.1 `Libs`/`Devs`
(`a1200-addons-smoke.sh:79-81`); `cscr2png.py` and `compare -metric AE` already serve the GUI gate.
Rejected: new gate scripts (more machinery than two case additions); a test-only C fixture that
inspects `ClipRegion` (the checksum observes the same fact through the public API). Confidence:
Medium-high. Reopen trigger: a red phase passes where this note expects a failure.

**AR-14:** Eligibility: testing strategy. Decision: the A1200 gate's startup-sequence writes
`CNEXT:done.txt` after the last probe; the gate waits for it (bounded at 240 s) and then for the
HD tree to stay unchanged for 3 s (bounded at 30 s), the settle rule of R1's AR-24/AR-26. A
missing result file is a mismatch with its name. Evidence: redirect files exist from the start
of each command (`a1200-addons-smoke.sh:99-103,194-201`). Confidence: High.

**AR-15:** Eligibility: performance engineering. Decision: the closing phase runs
`make bench-base`, `make bench`, `make bench-compare` (empty `REQUIRE`), and records the release
`.text` of `examples/amiga/gui.clo` (hosted-amiga-13, a `drawText` user) and
`examples/amiga/scroller.clo` (a layer user) at base and head; no new ceiling. The two new
helpers (`cl_text_draw_clipped`, `cl_res_each`) get external linkage so a release-image symbol
check (ST-3) can prove programs that never call them do not link them (the R1 `cl_exit_hook`
precedent: the hunk linker drops `static` names). Evidence: no benchmark calls these APIs;
section GC is on in every profile (`toolchain.rs:71-106`).
Confidence: High.

**AR-16:** Eligibility: documentation of the decided behaviour. Decision: update the doc comments
of `drawText` (`std.amiga.text`), `showAlert` (`std.amiga.intuition`), `savePublicScreen`
(`std.amiga.workbench`), `freeRegion`/`layerSetClipRegion`/module header (`std.amiga.regions`),
`closeAmigaGuide`/getters/`replyGuideMessage` (`std.amiga.amigaguide`), and `clipboardRead`
(`std.amiga.clipboard`); update `docs/known-limitations.md` §4 (the clipboard runtime proof) and
§5 (the clipboard-read bullet), and add the colour-font clip limit. Confidence: High.
