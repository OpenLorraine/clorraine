# Requirements: Amiga SDK Bundle 9 — Optional Add-ons and Advanced Closeout

> **Document**: 01-requirements.md
> **Parent**: [Index](00-index.md)

## Feature Overview

Bundle 9 closes every remaining open capability in the Amiga SDK registry. It is a capability
bundle, not a header dump: each row gets an opaque, documented Clorraine surface, a shim
implementation, a specification test, an implementation test, and a headless runtime gate where
the capability is observable. The licensed Amiga Forever Workbench media supplies the disk-based
libraries and devices that were missing in Bundles 4–8.

The public surface stays within the project’s rules: no raw OS structs, no `ffi` or `unsafe` in
application code, no language/grammar/diagnostic/ABI changes, and no claim without runtime
evidence at the row’s baseline.

## Functional Requirements

### Must Have

- **R1 — Closeout scope.** Implement the 20 open registry capabilities:
  `SDK-EXE-007/010/011`, `SDK-INT-006/007/008`, `SDK-DOS-004/006`, `SDK-TXT-003/005`,
  `SDK-AUD-006/008`, `SDK-WBK-003/004/005/006/007/010`, and `SDK-OPT-002/003`.
  RTG (`SDK-OPT-004`) stays `wont` and is tracked by issue #11. [AR-1, AR-5]
- **R2 — Exec and utility.** Signal semaphores; capture-less Hooks with a scalar data payload;
  tag lists and date-stamp handles in `std.amiga.utility`; OS memory pools in a distinct
  `std.amiga.pool` module (3.0+). [AR-11, AR-12, AR-13]
- **R3 — Intuition advanced.** A gadtools gadget builder with the native `GT_GetIMsg` event path,
  an ASL file requester, and a native Intuition BOOPSI string gadget. [AR-14, AR-15, AR-16]
- **R4 — DOS and data.** ReadArgs parsing with template-derived slot kinds and a deterministic
  `readArgsFrom` input path; picture datatype objects at 3.1. [AR-17, AR-18]
- **R5 — Graphics and input.** Region objects with layer clipping and the existing damage protocol;
  a dedicated EClock handle; lowlevel `ReadJoyPort` with type-aware decoding. [AR-19, AR-20, AR-21]
- **R6 — Workbench services.** Commodity broker lifecycle, locale, ARexx messages, clipboard,
  typed serial/parallel/printer devices, and nonvolatile byte storage. [AR-22–AR-27]
- **R7 — Network and hypertext.** An IPv4 TCP socket subset gated against FS-UAE’s emulated
  `bsdsocket.library` with a host `127.0.0.1` echo server, and an AmigaGuide async context. [AR-6,
  AR-28, AR-29]
- **R8 — Diskfont proof.** Exercise the existing `openDiskFont` on A500/KS1.3 with the Workbench
  1.3.5 HDF and a disk-only font. [AR-30]
- **R9 — Media installation.** Stage licensed ADFs read-only, copy named trees (never `S/`), normalize
  case, add `LIBS: CNEXT:Classes`, run `C:AddDataTypes`, and assign `DEVS:`, `FONTS:`, `LOCALE:`,
  `ENV:`, `ENVARC:`, `REXX:` as needed; never commit media. [AR-7, AR-36]
- **R10 — Safety fix.** The generic `openDevice` must reject audio, timer, serial,
  parallel, and clipboard because each request exceeds `IOStdReq`; printer uses a
  `union printerIO` typed handle. [AR-31]

### Should Have

- **R11 — Consistent error semantics.** Acquisition returns `T?`/`null`; accessors return neutral
  0/false; optional missing resources do not trap. [AR-32]
- **R12 — Test integrity.** Every new public export has a specification test that fails without
  the implementation; every runtime gate has a full-line or non-blank assertion; every gate clears
  its work directory first. [AR-33]
- **R13 — Registry truth.** Reassign the 20 open rows to `9-optional`, keep IDs, correct
  `SDK-EXE-007` to 3.0, `SDK-DOS-006` to 3.1, `SDK-AUD-006`/`SDK-WBK-010` to 3.1,
  `SDK-INT-008` to 2.0, `SDK-OPT-003` to 3.0, change
  `SDK-WBK-003` depends to `SDK-EXE-003`, and regenerate `coverage.md` and issue #10. [AR-3, AR-4,
  AR-38, AR-39]
- **R14 — Documentation.** Additive appendix sections, known-limitations update, `AGENTS.md`,
  programme index, roadmap, and issue #10. No chapter-15 edit. [AR-34, AR-35]
- **R15 — Review.** Run the correctness, memory-safety, and emission-integrity reviews; fix every
  critical/major finding and re-run the affected gates. [AR-33, AR-40]

### Won't Have (Out of Scope)

- RTG, Picasso96, CyberGraphX, third-party graphics cards.
- A real third-party TCP/IP stack for the gate; Roadshow/Miami/AmiTCP are documented only.
- Full commodities input-event/custom-hook routing; only broker lifecycle and uniqueness.
- ClassAct/custom BOOPSI classes; only native Intuition classes.
- Full printer output; the typed printer surface is gated on open/close and documented
  `PRD_RAWWRITE` limits.
- Language-level memory pools/arenas (`E90004`); only the OS `utility.library` pool wrapper.
- Capturing lambdas/Hook contexts; only capture-less callbacks.
- Any language, grammar, diagnostic, golden, or ABI change.

## Technical Requirements

### Performance

- Gates poll bounded handshake files with 60–90 s timeouts; no unbounded busy waits.
- Shim buffers are caller-sized and bounds-checked; no unbounded copy.
- The optional suffix adds no runtime work unless a module is used.

### Compatibility

- New modules compile and run on `hosted-amiga` only; the host, `hosted-amiga-13`, and
  freestanding profiles reject them with `E11200` and emit no C. [AR-10]
- `std.amiga.text.openDiskFont` stays available on `hosted-amiga-13`; only its runtime proof is
  new.
- The 2.0+ and optional C shims are appended after the generated program, so their headers cannot
  rewrite user identifiers. [AR-9]
- Baselines: A500/KS1.3 (diskfont), A600/KS2.05 (2.0/2.1 services/intuition), A600/KS3.1
  (lowlevel/nonvolatile), A1200/KS3.0 (BOOPSI/pool), A1200/KS3.1 (datatypes/network/guide).
  [AR-33, AR-39]

### Security

- Licensed media is read-only; no ROM keys or ADFs are committed. [AR-36]
- ReadArgs validates the template/slot count and per-slot kinds; clipboard/socket/device copies
  validate buffer bounds.
- Network gate binds `127.0.0.1` and uses `--network host`; no external network access.
- No secrets, credentials, or user data are written to logs or reports.

## Scope Decisions

| Decision | Options Considered | Chosen | Rationale | AR Ref |
|---|---|---|---|---|
| Bundle contents | 3 official / 20 open | 20 open | finish the programme; user confirmed | AR-1, AR-2 |
| Tracking | leave rows / reassign | reassign | generated issue is the full closeout | AR-3 |
| Network | real stack / FS-UAE | FS-UAE | no extra license; standard API | AR-6, AR-28 |
| Media install | ADF boot / staged extraction | staged | deterministic and writable-state safe | AR-7 |
| Module split | grab bag / per library | per library | matches existing SDK style | AR-8 |
| Hooks | defer / capture-less | capture-less scalar | row is in scope; interrupt precedent | AR-11 |
| Pools | skip / OS wrapper | distinct pool module | row includes pools; V39 | AR-12, AR-39 |
| BOOPSI | custom class / native Intuition class | native `strgclass` | set/get readback proven; no stored class dispatcher | AR-16 |
| Datatypes | 3.0 varargs / 3.1 object API | 3.1 object API | V40 API is the documented object surface | AR-18, AR-39 |
| Printer | full driver / typed open-close | typed open-close | driver/prefs not reliably gateable | AR-26 |
| Appendices | none / additive | additive | documented APIs need appendix text | AR-34, AR-35 |

## Acceptance Criteria

1. [ ] All 20 in-scope rows are `done` with acceptance and evidence; RTG remains out-of-scope.
2. [ ] `make check` passes with 0 failures.
3. [ ] `make verify` passes headlessly, including the new gates.
4. [ ] `make sdk-registry` passes; Bundle 9 shows 20 in-scope done / 21 total including RTG.
5. [ ] Every new module has a spec test, an impl test, a profile rejection test, and an appendix
   export check.
6. [ ] Issue #10 is regenerated, updated, and closed; issue #11 remains the RTG backlog.
7. [ ] No media, decrypted ROM, or scratch artifact remains in the repository.
