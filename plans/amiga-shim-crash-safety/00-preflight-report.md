## Preflight Report: amiga-shim-crash-safety

> **Status**: ✅ PREFLIGHT PASSED — all 9 findings resolved (0 critical, 4 major, 4 minor, 1 observation)
> **Iteration**: 2 (re-scan after fixes)
> **Artifact**: Implementation plan at `plans/amiga-shim-crash-safety/` (10 documents, committed at `809f353d`; scanned at HEAD `4de48b81`, where the plan and every cited shim, gate, and stdlib file are unchanged). Blobs before → after: `00-ambiguity-register` `ac415e52`→`615766cd`, `00-index` `f4b5f396`→`a11b830a`, `01-requirements` `6cac0b01`→`03abfbc3`, `02-current-state` `f7b18ee5`→`6a7c5127`, `03-01` `50dd4753`→`175b0011`, `03-02` `50856091`→`08cd38ee`, `03-03` `277c386a`→`37f4d6e7`, `03-04` `b34af64b`→`f4598ee8`, `07` `221d4cf4`→`5d9740c3`, `99` `3f0004aa`→`e20cfa70`
> **Codebase Grounded**: 11 source/script files and 6 test suites examined, 3 autodocs and the NDK 1.3 headers read, ~60 references verified
> **Independence**: uniform-effort (the plan was written by the same model in an earlier session; one blind challenger reviewed the four majors)
> **Last Updated**: 2026-11-27 08:42

Audit target: the whole plan folder. Context documents (not audited): `plans/00-roadmap.md`,
`plans/independent-assessment/` (findings WS5, verifier verdicts 9.2.4),
`plans/amiga-fault-exit-cleanup/`. Modification set: this plan folder and the roadmap rows of
PLAN-49 and its four DEF rows. Scope mode: strict (no optional additions reported).

Mode: auto-design (policy version 1, root invocation `ad-pf-r5-20261127`). Every resolution below
is an eligible technical decision (testing strategy, failure design, algorithm) inside the
confirmed scope. No reserved-authority, spec, or complexity-escalation decision arose. The
`--auto-commit` flag authorized applying the fixes and committing them.

### Codebase Context Summary

**Tech Stack:** std-only Rust workspace (`clc`, `clorraine`, `clorraine-lsp`); emitted C99;
hosted-amiga builds with `m68k-amigaos-gcc` and libnix (`nix20`/`nix13`), `vamos`, and FS-UAE.
**Architecture:** the base shim `shim_hosted_amiga.c` is prepended to every hosted-amiga TU, and
the 2.0 and optional suffixes are appended on `hosted-amiga` only, so a `static` helper in the
base shim is visible to the suffixes. The exit registry (`:100-275`) pops the newest record
before calling its release function; `cl_own_ports_hold` (`:3409-3420`) is the walk precedent.
**Key Files Examined:** `shim_hosted_amiga.c` (`cl_lib_open` :346-373, bases :1620-1623, screens
:1638-1685, `cl_show_alert` :2962-2988, layers and text :6556-6929, exit hook :6950-7019),
`shim_hosted_amiga_20.c` :532-547, `optional.c` regions :960-1113, `optional_net.c` AmigaGuide
:150-319, `optional_services.c` clipboard :696-826; `a1200-addons-smoke.sh`,
`a500-fault-cleanup-smoke.sh`, `a600-addons-smoke.sh`, `workbench-media.sh`, `Makefile`;
`compiler/tests/amiga_{fault_cleanup,intuition,workbench,text,optional}_impl_test.rs`,
`amiga_fault_cleanup_spec_test.rs`; NDK 3.2 `clipboard.doc`, `layers.doc`, `graphics.doc`,
`ReleaseNotes/clipboard-RelNotes`; `ndk13-include` `clib/graphics_protos.h`,
`clib/layers_protos.h`, `graphics/text.h`; `spec/appendix-stdlib.md` §14.9/14.18/14.22 and the
`drawText` and layers rows; `stdlib/std.amiga.{text,layers,regions,amigaguide,clipboard,video}.clo`.

**Verified:** the line references in 02 (within one site, PF-009); `IntuitionBase` is opened only
by the two constructors; `vamos -O <lib>=mode:off` parses as the plan says (amitools
`vamos/cfg/libs.py`); every graphics and layers call the plan uses is in the 1.3 protos;
`cl_screen_save_native` makes no graphics call; the A1200 gate assigns `CLIPS:` and installs the
WB 3.1 `Devs`; the optional suffixes are in the same TU as the base shim. The changes between
`809f353d` and `4de48b81` touch only the freestanding shim, the `check`/`dist` recipes, and
known-limitations §17.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 0 | — |
| 2 | Implicit Assumptions | 2 (PF-002, PF-006) | 🟠 |
| 3 | Logical Contradictions | 3 (PF-003, PF-004, PF-008) | 🟠 |
| 4 | Completeness Gaps | 1 (PF-003) | 🟠 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 1 (PF-006) | 🟡 |
| 7 | Testability | 3 (PF-001, PF-005, PF-006) | 🟠 |
| 8 | Security Blind Spots (memory safety) | 1 (PF-004) | 🟠 |
| 9 | Edge Cases | 2 (PF-001, PF-004) | 🟠 |
| 10 | Scope Creep Indicators | 1 (PF-003) | 🟠 |
| 11 | Ordering & Sequencing | 0 | — |
| 12 | Consistency | 2 (PF-007, PF-009) | 🟡 |
| 13 | Codebase Alignment | 3 (PF-004, PF-007, PF-009) | 🟠 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | — |
| MAJOR | 4 | all resolved |
| MINOR | 4 | all resolved |
| OBSERVATION | 1 | resolved |

---

### PF-001: ST-5 never observes the end of the read after two of its three end-of-read paths 🟠 MAJOR

**Dimension:** Testability / Edge Cases
**Location:** `07` ST-5; `03-01` §Clipboard read, Error Handling; AR-10
**Codebase Evidence:** `optional_services.c:796-821` (one caller read today). `clipboard.doc`
CMD_READ: a read when "io_Offset is beyond the end of the clip" is the end signal, and RESULTS:
"io_Offset updated to next read position, which is beyond EOF if io_Actual != io_Length". The
verifier's hang came from a read that returned 0 bytes (`verdicts-9.2.4.md:31`).
**The Problem:** ST-5 writes after its first read only. Its second read (a buffer larger than the
clip) is followed by `closeClipboard`, which may release a hold-off by itself. So a finishing
sequence that left the read open on that path would pass. No probe reads into a buffer exactly
the clip size. In that case the skip read starts at the end and returns 0, and the planned
one-byte read follows it. A device that treated that read as the start of a new read would hold
writes off again.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Keep the finishing sequence. ST-5 writes after each of three reads: part of the clip (8 of 24), exactly the clip (24 of 24), and a larger buffer (24 into 64). Add a reopen trigger for an exact-fit hang. | Every path gets an oracle; follows the autodoc's "beyond EOF" rule | Exact-fit behaviour stays proven by target run only |
| B | Also stop the sequence after a skip read that returns 0 | Never reads after a zero-count read | The exact-fit skip read starts at the end, not past it, so stopping may leave the read open (challenger) |

**Recommendation:** Option A. After the skip read, `io_Actual` is always below `io_Length`, so the
autodoc puts `io_Offset` beyond EOF, and the one-byte read is the defined end signal. The
exact-fit write is the oracle that would catch a device that differs. Confidence: Medium.
Hardening: changed the pick. The reviewer first chose B; the challenger showed B can stop before
the end signal in the exact-fit case. Challenger: diverged; the reviewer adopted the
challenger's position.

**User Decision:** Authority: AI — delegated by --auto-design · Eligibility: testing strategy
and device-protocol algorithm inside FR-3 · Objective: every end-of-read path is proven not to
hold writes off · Decision: Option A · Evidence: as above · Rejected alternatives: B (may stop
before the end signal) · Strongest counterargument: a ROM that handles a read after an ended
read differently from its doc; the exact-fit write catches it · Confidence: Medium · Hardening:
challenger diverged and was adopted · Policy version: 1 · Root invocation ID: ad-pf-r5-20261127
· Reopen triggers: the write after the exact-fit read hangs (then stop after a zero-count skip
read and re-run ST-5). Applied: `07` ST-5 and red phase, `03-01`, `03-04`, AR-10/AR-13 notes,
`99` tasks 2.1.1–2.2.2.

---

### PF-002: The NULL-data reads are proven only on the 3.1 device 🟠 MAJOR

**Dimension:** Implicit Assumptions
**Location:** `01` §Compatibility (2.0+ for the clipboard); `03-04` legs; AR-10's strongest
counterargument ("the NULL skip is unproven on the 2.0 device")
**Codebase Evidence:** `sdk-a600-addons` boots A600/KS2.05 with the Workbench 2.04 `Devs`
(`a600-addons-smoke.sh:80-81`) and `CLIPS:` (`workbench-media.sh:109`), but its clipboard leg
only writes (`examples/amiga/optional-services.clo:169`). `docs/known-limitations.md:62-64`:
"the 2.05 clipboard device returned no data on read in the probe environment".
`ReleaseNotes/clipboard-RelNotes:22-24`: before 45.1 the device "always assumed that the clip
written is in IFF format".
**The Problem:** hosted-amiga supports Kickstart 2.0+. A 2.0 device that ignored a NULL
`io_Data` would copy the skipped bytes to address 0 and overwrite `AbsExecBase`, from a safe call.
The plan runs no read on a 2.0 device.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Run the ST-5 probe in `sdk-a600-addons` too: one startup line after the existing legs, one expected line | Direct proof on the 2.04 device with an existing gate | One more probe line and wait in that gate |
| B | Record 2.0 as unproven in known-limitations | No gate change | Leaves a possible machine-wide corruption unproven |
| C | Replace NULL reads with scratch-buffer reads | No NULL dependency | Still needs 2.0 proof of the end-of-read behaviour; one `DoIO` per 256 bytes skipped |

**Recommendation:** Option A. Confidence: High. Challenger: converged. The likely cause of the
"no data on 2.05" note is the raw write in `optional-services.clo`; an IFF clip reads back on a
pre-45.1 device.

**User Decision:** Authority: AI — delegated by --auto-design · Eligibility: testing strategy for
the stated 2.0+ compatibility · Objective: prove the NULL-data reads on the 2.0 device ·
Decision: Option A; the A600 red is recorded but not required (the A1200 carries it) · Evidence:
as above · Rejected alternatives: B (leaves the risk), C (more I/O, still needs 2.0 proof) ·
Strongest counterargument: if the 2.04 device reads an IFF clip differently, the expected line
cannot be shared · Confidence: High · Hardening: challenger converged · Policy version: 1 ·
Root invocation ID: ad-pf-r5-20261127 · Reopen triggers: the 2.04 device reads an IFF clip
differently from 3.1 (stop for a ruling per AR-10). Applied: `00-index`, `01` FR-6 and
§Compatibility, `02`, `03-01`, `03-04` §A600 leg, `07`, AR-13/AR-16 notes, `99` tasks
2.1.1–2.2.2, 5.1.2, 5.1.3.

---

### PF-003: A runtime-ended update makes the program's `layerEndUpdate` unlock an unlocked layer 🟠 MAJOR

**Dimension:** Logical Contradictions / Completeness Gaps / Scope Creep Indicators
**Location:** `03-03` §Regions (set/clear clip, `cl_layer_unclip`); AR-8; `01` FR-4
**Codebase Evidence:** `cl_layer_end_update` calls `EndUpdate(layer, TRUE)` unconditionally
(`shim_hosted_amiga.c:6921-6929`). layers.doc EndUpdate: "The layer is then unlocked";
BeginUpdate: "The layer is locked"; BeginUpdate BUGS: a FALSE result still needs
`EndUpdate(l, FALSE)`. The spec row: "`EndUpdate(... TRUE)` completes the redraw".
**The Problem:** the plan ends an open update in `layerSetClipRegion`, `layerClearClipRegion`,
and `freeRegion`'s walk. A safe program that calls one of those inside an update block and then
`layerEndUpdate` runs `EndUpdate` twice and unlocks a layer it no longer holds. That is a crash
path the fix would add. Ending the update inside set/clear clip also changes what a valid
program's update block does, which none of the four findings asks for.

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Only `freeLayer` (program free and exit release) ends an open update, with `EndUpdate(FALSE)`, before unclipping and `DeleteLayer`. `freeRegion`'s walk installs NULL without ending it. Set/clear clip and `layerEndUpdate` unchanged. Flag set on every `layerBeginUpdate`. | No double unlock; valid programs unchanged; covers a FALSE begin at free time | The walk can install NULL during an update, which the autodoc discourages (low-memory cliprect glitch) |
| B | Keep ending updates everywhere; `layerEndUpdate` calls `EndUpdate` only while the flag is set | Never calls the discouraged sequence | Changes valid programs and the spec row's `layerEndUpdate` behaviour; more code |

**Recommendation:** Option A. Confidence: Medium-high. Challenger: converged and added the
refinement of setting the flag on every call.

**User Decision:** Authority: AI — delegated by --auto-design · Eligibility: failure design on
misuse paths · Objective: no lock imbalance; valid programs unchanged · Decision: Option A ·
Evidence: as above · Rejected alternatives: B (changes valid programs and `layerEndUpdate`) ·
Strongest counterargument: the exit-time walk may install NULL during an update; its documented
effect is cosmetic and limited to low memory · Confidence: Medium-high · Hardening: challenger
converged with a refinement · Policy version: 1 · Root invocation ID: ad-pf-r5-20261127 · Reopen
triggers: the region probe shows damage after an update. Applied: `00-index`, `01` FR-4, `02`,
`03-03`, `07` impl rows, AR-8 note and row, `99` tasks 3.2.2, 3.2.3.

---

### PF-004: The superset box misses positive kerning and the pen advance 🟠 MAJOR

**Dimension:** Security Blind Spots (memory safety) / Logical Contradictions / Codebase Alignment
**Location:** `03-02` §Proposed Changes step 3 and step 5; AR-5 (which lists `TxSpacing` in the
union, while 03-02 drops it from the right edge)
**Codebase Evidence:** `text.h` (`tf_CharKern`, `tf_CharSpace` per character); graphics.doc
TextExtent (`te_Extent.MaxX` "often the same as te_Width-1"), Text BUGS (V34 clips at the pen
span), SetFont NOTES (V36+ `SetFont` may modify a legacy font) and FUNCTION ("clears the effect
of any previous soft styles"); the shim calls `SetFont` before `Text` (`:6708-6711`).
**The Problem:** the glyph image is drawn at `p + k`, but the planned right edge is
`min(p, p + k) + w`, which falls `k` columns short when `k > 0`. The right edge also leaves out
`p + a`, where the JAM2 background ends. The box decides both "fully inside → today's unclipped
`Text()`" and the size of the raster `Text()` renders into. An underestimate can therefore write
past the bitplane on the bottom rows, or past the temporary raster, which is the defect class the
plan fixes. The box must be read after `SetFont`. Its bold and italic terms are dead: `SetFont`
clears soft styles, and nothing sets them again. An empty text has no defined box.

**Options:** only one is viable.
Per character, with `k`, `s`, `w`, and signed advance `d = k + s + TxSpacing` (negated for
`FPF_REVPATH`), the columns are `min(p, p + k, p + d)` to `max(p + k + w, p + d, p + XSize) - 1`.
The box is computed after `SetFont`. Drop the soft-style terms and the `AlgoStyle` copy. An empty
text draws nothing.
Considered and dropped: `TextExtent` on V36+ (the cull still needs per-character boxes, and it is
off by one before V47); a fixed margin (unproven, and it still sizes the raster from the weak
edge).

**Recommendation:** Apply the single option. Confidence: Medium-high. Challenger: converged,
adding the `SetFont` order and the dead style terms.

**User Decision:** Authority: AI — delegated by --auto-design · Eligibility: algorithm inside
FR-2 · Objective: the box contains every pixel `Text()` can touch · Decision: the corrected box ·
Evidence: as above · Rejected alternatives: TextExtent, fixed margin · Strongest
counterargument: 1.3 ROM rendering cannot be checked against source; ST-10 is the only target
check · Confidence: Medium-high · Hardening: challenger converged · Policy version: 1 · Root
invocation ID: ad-pf-r5-20261127 · Reopen triggers: ST-10 shows a missed ink pixel. Applied:
`03-02`, `07` impl row, AR-4/AR-5 notes and row, `99` task 4.2.1.

---

### PF-005: ST-7 has an unrecorded guard and a blitter race 🟡 MINOR

**Dimension:** Testability
**Location:** `07` ST-7 and red-phase expectations
**Codebase Evidence:** `cl_layer_fill` (`:6866-6875`) and `cl_video_bitmap_checksum`
(`:4980-5000`) have no `WaitBlit`; DEF-605 is open; `cl_video_bitmap_fill` (`:4730`) ends with
`WaitBlit` (`:4769`).
**The Problem:** nothing observable through the public API shows a layer deleted with a clip
installed. So ST-7's layer-free half, and the update ending in `freeLayer`, pass on the base, and
FR-6 requires such guards to be recorded. Separately, the checksum can read a bitmap while the
blitter is still filling it.
**Resolution:** record both as guards covered by impl tests. ST-7 runs `bitmapFill` on a scratch
bitmap before each comparison, which waits for the blitter.
**User Decision:** Authority: AI — delegated by --auto-design (testing strategy), Policy
version 1, root ad-pf-r5-20261127. Applied: `07` ST-7 and red phase, `01` acceptance 1, `99`
task 3.1.3.

---

### PF-006: The ST-10 gate case lacks four mechanics 🟡 MINOR

**Dimension:** Testability / Implicit Assumptions / Feasibility Concerns
**Location:** `03-04` §A500 leg; `07` ST-10
**Codebase Evidence:**
- Every case in `a500-fault-cleanup-smoke.sh:120` builds from
  `examples/robustness/fault-cleanup/`.
- The host-tool list has only `sha1sum` and `xdotool` (`:81`); `cscr2png.py` runs on host
  `python3` (`a500-gui-smoke.sh:49,302`).
- The only `chip`-checked case, `device-fault`, opens no Intuition screen (R1 03-05: "the probe
  that loads nothing resident").
- The capture palette decode is a known defect (ASM-WS5-008).
**The Problem:** the probe's source path, `python3`, a palette-independent text check, and a
`chip` fallback are missing. The I3 and I5 windows also lie in the screen's title-bar rows, an
assumption the plan does not state.
**Resolution:** 03-04 names the source path and adds `python3`. The reference must hold at least
two colours. A green `chip`/`mem` difference triggers a local contrast run, then `-` with a
recorded reason, or a defect. The title-bar assumption is stated.
**User Decision:** Authority: AI — delegated by --auto-design (testing strategy), Policy
version 1, root ad-pf-r5-20261127. Applied: `03-04`, `07` ST-10, `99` tasks 4.1.2, 4.2.3.

---

### PF-007: ST-3 names a path that does not exist and builds a stripped image 🟡 MINOR

**Dimension:** Codebase Alignment (Phantom Reference) / Consistency
**Location:** `07` ST-3
**Codebase Evidence:** `examples/hello.clo` does not exist; the program is
`examples/hello/hello.clo`. The exit-hook symbol test builds `Build::ReleaseUnstripped`
(`amiga_fault_cleanup_spec_test.rs:94-147,355-375`); the gates build with `-s`.
**The Problem:** the path is wrong. The input column also omits the `drawText` and region probes
that the expected column names, and a stripped image has no symbols for `nm` to find.
**Resolution:** ST-3 names `examples/hello/hello.clo`, `alert-first`, `text-clip`, and
`region-free`, all built release-unstripped.
**User Decision:** Authority: AI — delegated by --auto-design (testing strategy), Policy
version 1, root ad-pf-r5-20261127. Applied: `07` ST-3.

---

### PF-008: The text performance requirement contradicts the design 🟡 MINOR

**Dimension:** Logical Contradictions
**Location:** `01` §Performance ("Text that fits keeps its current cost") vs. `03-02` step 3
(every call walks the string)
**The Problem:** a call that fits now pays an O(count) metric walk.
**Resolution:** the requirement says such a call keeps its drawing calls and pays one pass over
the characters, with no allocation and no extra blit.
**User Decision:** Authority: AI — delegated by --auto-design (documentation of the decided
behaviour), Policy version 1, root ad-pf-r5-20261127. Applied: `01` §Performance.

---

### PF-009: Two small reference drifts 🔵 OBSERVATION

**Dimension:** Consistency / Codebase Alignment
**Location:** `02` (layers registered at `:6758,6781`); `07` pin-update row
**Codebase Evidence:** the only layer registration is `:6781`; `:6758` is `NewLayerInfo`. The
bodies the plan changes are also pinned in `amiga_fault_cleanup_impl_test.rs` (`cl_layer_free`,
`cl_free_region`, guide release order) and `amiga_text_impl_test.rs`.
**Resolution:** 02 cites `:6781`, and 07's pin row names the five suites.
**User Decision:** Authority: AI — delegated by --auto-design, Policy version 1, root
ad-pf-r5-20261127. Applied: `02`, `07`.

---

## Iteration 2

> **Status**: ✅ PASSED
> **Previous Iteration**: 9 findings — all resolved
> **This Iteration**: 0 new findings
> **Carried Forward**: none

- **Verified fixes:**
  - PF-001: the three-read ST-5 and the exact-fit reopen trigger appear in 07, 03-01, 03-04,
    AR-10, and 99.
  - PF-002: the A600 leg appears in 00, 01, 02, 03-01, 03-04, 07, and 99.
  - PF-003: only `freeLayer` ends an update, in 03-03, 01 FR-4, AR-8, and 99. A search found no
    remaining "end an open update in set/clear clip".
  - PF-004: the corrected box, the `SetFont` order, and the dropped style terms appear in 03-02,
    AR-5, 07, and 99. AR-4's `AlgoStyle` copy is marked superseded.
  - PF-005 to PF-009: verified in place.
- **Regression check:**
  - The register's original AR-4, AR-5, AR-8, and AR-10 notes now point to §Preflight
    refinements, so the register does not contradict itself.
  - The task count is unchanged (39).
  - The specification-first order is unchanged; every phase keeps spec tests → red → implement
    → green → impl tests.
  - The A600 probe runs after the gate's interactive ASL leg, so a red-phase hang cannot block
    the existing legs.
- **Bounded fresh scan:** all 13 dimensions within the plan folder. No new defect, and no
  optional addition was introduced.
- **Codebase re-check:** the new references are the A600 gate lines `:80-81` (media),
  `:103-107` (startup-sequence), and `:187-188` (result files), `cl_video_bitmap_fill` `:4730`,
  and `bitmapFill`/`bitmapChecksum` in `std.amiga.video.clo:189,231`. All verified.

### Adversarial checklist

- **Assumption from creation:** that a read returning 0 bytes ends the clipboard session. The
  verifier's trace shows it does not after the first read, and PF-001 keeps the doc's "beyond
  EOF" rule rather than that assumption.
- **External standard:** the device and library behaviour comes from the vendored NDK 3.2
  autodocs. 1.3 and 2.0 ROM behaviour is not available as text: the 1.3 `Text()` metrics, 2.0
  clipboard reads, and AmigaGuide close rules. The FS-UAE legs (ST-5 on A600 and A1200, ST-10 on
  A500) are the evidence, as AR residual limits 1–5 record.
- **Domain expert's objection:** the exact-fit clipboard path and the 1.3 text box. Each has a
  target oracle and a reopen trigger. This plan is on the memory-safety boundary of the safe
  stdlib, so a human Amiga reviewer is worth having in addition to the `amiga-platform-reviewer`
  phase reviews.
