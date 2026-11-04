# Ambiguity Register: Amiga SDK Bundle 3 — Audio and Input

> **Document**: 00-ambiguity-register.md
> **Parent**: [Index](00-index.md)
> **Status**: ✅ GATE PASSED
> **Mode**: auto-design (eligible technical decisions delegated and recorded)

## Scope baseline

Bundle 3 (`3-audio-input`) from `docs/amiga-sdk/registry.tsv`: audio.device
open/close, four-channel sample playback, period/volume/loop/stop, gameport
joystick, timer.device delays, the paddle game with sound, and the two
advanced `could` rows (EClock 2.0+, ReadJoyPort 3.1+). No language, grammar,
diagnostic, or ABI change.

## Register

| # | Category | Decision / question | Resolution | Owner | Status |
|---|---|---|---|---|---|
| AR-1 | Scope | Which capabilities belong to Bundle 3? | The six core rows (`SDK-AUD-001`–`005`, `007`). The two advanced `could` rows (`EClock`, `ReadJoyPort`) are deferred to the advanced bundles because they require 2.0/3.1 libraries outside the A500 baseline. | User (registry), AI — delegated | ✅ Resolved |
| AR-2 | Module split | One module or several? | Three modules: `std.amiga.audio` (device, samples, channels), `std.amiga.input` (joystick), `std.amiga.timer` (timer.device). Each is a distinct resource and keeps the surface small. | AI — delegated (auto-design) | ✅ Resolved |
| AR-3 | Audio API | One channel or all four? | `openAudio(channels: u8)` requests a mask (1..4), and the handle exposes the allocated mask plus per-channel play/stop/control. The game opens four channels and plays on channel 0. | AI — delegated (auto-design) | ✅ Resolved |
| AR-4 | Sample storage | Caller buffer or owned chip copy? | `newSample` copies the caller's bytes into a chip-memory `Sample`; the audio DMA requires chip memory and word-aligned, even-length data. | AI — delegated (auto-design) | ✅ Resolved |
| AR-5 | Playback | Synchronous or async? | `audioPlay` uses `SendIO` with `ADIOF_PERVOL` and returns immediately; a busy channel returns `false`. The sample must outlive playback. | AI — delegated (auto-design) | ✅ Resolved |
| AR-6 | Control | How are period/volume/loop/stop exposed? | The `Sample` carries period, volume, and cycles (0 = loop forever, 1 = once). `audioControl` sends `ADCMD_PERVOL`; `audioStop` sends `ADCMD_FINISH`. | AI — delegated (auto-design) | ✅ Resolved |
| AR-7 | Joystick | gameport.device, lowlevel, or raw registers? | Raw OCS `JOY0DAT`/`JOY1DAT` plus the CIAAPRA fire bits, because the core target is 1.3 and gameport/lowlevel are 2.0/3.1. `readJoystick(port)` returns a direction/fire bitmask. | AI — delegated (auto-design) | ✅ Resolved |
| AR-8 | Timer | dos.library Delay or timer.device? | `std.amiga.timer` opens timer.device `UNIT_VBLANK` and uses `TR_ADDREQUEST`; the paddle loop uses it instead of the dos delay. | AI — delegated (auto-design) | ✅ Resolved |
| AR-9 | Game | Scripted demo or interactive? | A Pong-style paddle game with a deterministic auto-play fallback when the joystick is idle; the joystick is read every frame and overrides the AI. The acceptance run uses the idle path. | AI — delegated (auto-design) | ✅ Resolved |
| AR-10 | Acceptance | How is the game proven on real 1.3? | The game opens an Intuition screen, opens four audio channels, plays a beep on each paddle hit, uses the timer for the frame loop, holds a deterministic frame, and prints `paddle: ok hits <N> audio <mask> input 0`. The A500 smoke compares the held screen and the line. | User (programme acceptance), AI — delegated | ✅ Resolved |
| AR-11 | Compatibility | Does the 3.1 profile change? | The modules are available on both hosted profiles; no existing API changes. The 1.3 whitelist adds all three modules. The advanced `could` rows stay deferred. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-12 | Errors | What happens on device/allocation failure? | Constructors return `T?`; invalid channels, sample lengths, or joystick ports are no-ops or `null`; the game prints a distinct failure line. | AI — delegated (auto-design) | ✅ Resolved |
| AR-13 | Documentation | Which docs move with the code? | `spec/appendix-amiga.md`, `spec/appendix-stdlib.md`, `AGENTS.md`, registry/coverage, issue #4, programme/roadmap, plan. | User (Bundle 0 rule), AI — delegated | ✅ Resolved |
| AR-14 | Language | Any language/grammar/diagnostic/ABI change? | None. | User (programme rule) | ✅ Resolved |
| AR-15 | Testing | Where do the tests live? | `compiler/tests/amiga_audio_spec_test.rs`, `amiga_audio_impl_test.rs`, and `docker/scripts/a500-paddle-smoke.sh`. | AI — delegated (auto-design) | ✅ Resolved |

## Open items

None. Every row is resolved; the two `could` rows are explicitly deferred.
