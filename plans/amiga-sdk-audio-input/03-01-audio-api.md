# Design: Audio API

> **Document**: 03-01-audio-api.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R4, R9; AR-2–AR-6

## Module surface

`std.amiga.audio` is available on `hosted-amiga` and `hosted-amiga-13`; the
host and freestanding profiles reject it with `E11200`. It wraps
`audio.device` through `IOAudio` blocks and a private reply port.

```text
export handle Audio freedBy cl_audio_free;
export handle Sample freedBy cl_sample_free;
```

## Audio handle

`openAudio(channels: u8)` requests a channel mask:

| `channels` | Requested combinations |
|---|---|
| 1 | `{0x01}`, `{0x02}`, `{0x04}`, `{0x08}` |
| 2 | `{0x03}`, `{0x0C}` |
| 3 | `{0x07}`, `{0x0E}` |
| 4 | `{0x0F}` |

The device returns the allocated mask in `io_Unit` and the allocation key in
`ioa_AllocKey`. The handle stores one `IOAudio` block per allocated channel,
each with the key and its channel bit.

| API | Returns | Rule |
|---|---|---|
| `openAudio(channels: u8)` | `Audio?` | 1..4; `null` on device or allocation failure |
| `freeAudio(take audio)` | `void` | stops pending writes, sends `ADCMD_FREE`, closes the device, deletes the port |
| `audioChannels(borrow audio)` | `u8` | allocated channel mask (1..15) |
| `audioPlay(borrow audio, channel: u8, borrow sample: Sample)` | `bool` | `CMD_WRITE` with `ADIOF_PERVOL`; `false` if the channel is unallocated or busy |
| `audioStop(borrow audio, channel: u8)` | `void` | `ADCMD_FINISH` aborts the playing write |
| `audioControl(borrow audio, channel: u8, period: u16, volume: u8)` | `bool` | `ADCMD_PERVOL`; `false` if not playing |

`audioPlay` is asynchronous: the sample must stay alive until the channel
finishes or is stopped. The device DMA reads the sample from chip memory.

## Sample handle

`newSample(const data: u8[], period: u16, volume: u8, cycles: u16)` copies the
signed 8-bit samples into a chip-memory buffer. The length is rounded up to an
even byte count; the period is the Amiga audio period (`3579545 / rate`).
`cycles = 0` loops forever, `1` plays once.

| API | Returns | Rule |
|---|---|---|
| `newSample(data, period, volume, cycles)` | `Sample?` | zero length or allocation failure returns `null` |
| `freeSample(take sample)` | `void` | frees the chip buffer; the sample must not be playing |
| `sampleLength`, `samplePeriod`, `sampleVolume`, `sampleCycles` | values | read-only accessors |

## Device lifecycle

`openAudio`:

1. `CreatePort(NULL, 0)` for the reply port.
2. Fill the open `IOAudio` with the reply port, `ln_Pri = 0`,
   `ioa_AllocKey = 0`, `ioa_Data = masks`, `ioa_Length = mask count`.
3. `OpenDevice("audio.device", 0, &open->ioa_Request, 0)`; the device allocates
   the channels and fills `io_Unit`/`ioa_AllocKey`.
4. Allocate one `IOAudio` per allocated channel with the key and unit bit.

`freeAudio` sends `ADCMD_FREE` with the allocated mask and key, then
`CloseDevice`, `DeletePort`, and frees the blocks.

## Errors

- A missing audio.device or failed allocation returns `null`.
- A bad channel or a zero-length sample is a no-op or `false`;
  `freeSample` must not be called while the sample is playing.
- The game prints `paddle: audio failed` when audio cannot open; the smoke
  rejects that line.
