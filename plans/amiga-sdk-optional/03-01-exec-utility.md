# Exec and Utility: Bundle 9

> **Document**: 03-01-exec-utility.md
> **Parent**: [Index](00-index.md)

## Overview

This component covers `SDK-EXE-011` signal semaphores, `SDK-EXE-010` hooks, and `SDK-EXE-007`
utility tags/date/memory pools. `std.amiga.semaphore` and `std.amiga.utility` are 2.0+;
`std.amiga.pool` is 3.0+ because `CreatePool` is V39. All are `hosted-amiga` only and live in the
optional suffix. `std.amiga.exec` stays 1.3-safe. [AR-8–AR-13, AR-39]

## Implementation Details

### `std.amiga.semaphore`

| Export | Signature | Ownership / behavior |
|---|---|---|
| `Semaphore` | `handle freedBy cl_free_semaphore` | Owns one `struct SignalSemaphore` |
| `createSemaphore` | `(): Semaphore?` | Allocates and `InitSemaphore`; null on OOM |
| `freeSemaphore` | `(take s: Semaphore): void` | Frees the struct; caller must release before drop |
| `obtainSemaphore` | `(borrow s: Semaphore): void` | Blocking `ObtainSemaphore` |
| `releaseSemaphore` | `(borrow s: Semaphore): void` | `ReleaseSemaphore` |
| `attemptSemaphore` | `(borrow s: Semaphore): bool` | Non-blocking `AttemptSemaphore` |

The destructor is null-safe. The caller owns the obtain/release protocol; the gate proves
obtain → attempt(false while held) → release → attempt(true).

### `std.amiga.utility` — tags, date, hooks

| Export | Signature | Ownership / behavior |
|---|---|---|
| `TagList` | `handle freedBy cl_free_tag_list` | Owns `TagItem[capacity+1]` with `TAG_DONE` |
| `newTagList` | `(capacity: i32): TagList?` | Active capacity 1..63 |
| `tagSet` | `(borrow list, tag: u32, data: u32): bool` | Updates/appends; rejects `TAG_DONE`; false when full |
| `tagGet` | `(borrow list, tag: u32): u32` | Value; use `tagPresent` to distinguish absent |
| `tagPresent` | `(borrow list, tag: u32): bool` | True when the tag exists |
| `tagCount` | `(borrow list): i32` | Active tags excluding `TAG_DONE` |
| `freeTagList` | `(take list): void` | Frees the array and record |
| `DateStamp` | `handle freedBy cl_free_date_stamp` | Owns one 12-byte snapshot |
| `currentDateStamp` | `(): DateStamp?` | One `DateStamp()` snapshot |
| `dateStampDays` | `(borrow stamp): i32` | `ds_Days` |
| `dateStampMinute` | `(borrow stamp): i32` | `ds_Minute` |
| `dateStampTick` | `(borrow stamp): i32` | `ds_Tick` |
| `freeDateStamp` | `(take stamp): void` | Frees the record |
| `Hook` | `handle freedBy cl_free_hook` | Owns a `struct Hook` record |
| `newHook` | `(entry: func(): void, data: i32): Hook?` | Capture-less callback; `h_Data` stores the `i32` payload |
| `callHook` | `(borrow hook): i32` | Returns the value set by `hookSetResult` |
| `hookData` | `(borrow hook): i32` | Returns the `i32` payload |
| `freeHook` | `(take hook): void` | Frees the record; no callback at drop |

The hook trampoline ignores the OS object/message and calls the Clorraine callback with no
arguments. The callback reads the `i32` payload with `hookCurrentData` and sets the result with
`hookSetResult`; `callHook` returns that value. This is a capture-less, allocation-free callback,
audited like the interrupt dispatcher. No `raw` pointer is exposed. [AR-11]

### `std.amiga.pool` — OS utility pools (3.0+)

| Export | Signature | Ownership / behavior |
|---|---|---|
| `UtilityPool` | `handle freedBy cl_free_utility_pool` | Owns a refcounted pool record |
| `createUtilityPool` | `(memFlags: u32, puddleSize: u32, threshSize: u32): UtilityPool?` | `CreatePool`; null on OOM |
| `allocUtilityBlock` | `(borrow pool, size: u32): UtilityBlock?` | `AllocPooled`; increments live count |
| `utilityBlockSize` | `(borrow block): u32` | Stored size |
| `writeUtilityBlock` | `(borrow block, borrow data: u8[]): bool` | Copies min(size, data length) |
| `readUtilityBlock` | `(borrow block, borrow buf: u8[]): i32` | Copies min(size, buffer); bytes copied |
| `UtilityBlock` | `handle freedBy cl_free_utility_block` | Frees with `FreePooled`; decrements pool count |
| `freeUtilityPool` | `(take pool): void` | Drops the caller reference; `DeletePool` when the count reaches zero |

The pool record is refcounted: a pool drop with live blocks defers `DeletePool` until the last
block is freed. A block drop after the pool handle is dropped is safe. The gate exercises both
orders. These are OS pools, not the deferred language-level `createPool`/`allocPooled` API.
[AR-12, AR-39]

## Error Handling

- Acquisition returns `null`; `tagSet` false when full; `attemptSemaphore` false when unavailable.
- `writeUtilityBlock`/`readUtilityBlock` copy at most the destination/source length; a short buffer
  is a successful partial copy.
- Accessors are null-safe; a null handle reads as zero/empty.
- The semaphore destructor does not release an obtained semaphore; the caller must release first.

## Tests

- Compile all three surfaces on `hosted-amiga`; reject host/1.3/freestanding with `E11200`.
- Cross-build `nix20`; `std.amiga.pool` is 3.0-gated through the A1200/KS3.0 gate, not a profile
  change.
- Pin `InitSemaphore`/`ObtainSemaphore`/`ReleaseSemaphore`/`AttemptSemaphore`, tag array
  `TAG_DONE`, `DateStamp`, hook trampoline/`h_Data`, `CreatePool`/`AllocPooled`/`FreePooled`/
  `DeletePool`, and the refcount order.
- Runtime: A600/KS2.05 for semaphore/hook/tags/date; A1200/KS3.0 for the pool.
- Gate lines: `exec: ok sem 1 hook 1 tag 1 date 1`; `pool: ok 1`.
