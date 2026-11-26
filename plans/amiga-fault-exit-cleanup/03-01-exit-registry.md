# Exit registry and hook: Amiga fault-exit cleanup

> **Document**: 03-01-exit-registry.md
> **Parent**: [Index](00-index.md)

## Overview

One shim-level mechanism releases every OS resource on every hosted-amiga exit: a registry of
owned-handle records plus fixed phases for interrupt servers, worker tasks, chip state, and
library bases, all run by one `atexit` hook (AR-1, AR-2, AR-8). Everything lives in
`compiler/runtime/shim_hosted_amiga.c`, the shim both hosted profiles prepend, so the suffix files
call it without `#ifdef`s (AR-8 evidence; `compiler/src/profile.rs:18-28,131-149`).

## Architecture

### Current Architecture
See [02 §What Exists](02-current-state.md#what-exists): `cl_trap`/`cl_panic` call `exit(20)`;
there is no exit hook and no record of acquired resources.

### Proposed Changes

```
acquire site ──► cl_res_add / cl_res_track / cl_lib_open ──► (first use) atexit(cl_exit_hook)
release site ──► cl_res_remove / cl_res_untrack
exit(rc) from main return, cl_trap, cl_panic, break, or libnix ***Break
        └─► cl_exit_hook: phase 1 servers → 2 workers → 3 registry (reverse) → 4 chip state → 5 library bases
```

`cl_trap` and `cl_panic` keep their text, order, and `exit(20)`. They add only the worker check
([03-03](03-03-worker-faults.md)): one load and compare of a static. They gain no re-entry branch
and reference no hook or registry code, so section GC keeps the hook out of programs that never
register (§Re-entry) (AR-2, AR-9 rule 10, AR-18).

## Implementation Details

### New Types/Interfaces

```c
/* One release record. Embedded in a shim wrapper struct, or allocated as a
 * side record for a raw OS pointer. */
typedef struct cl_res {
    struct cl_res *prev;
    struct cl_res *next;
    void (*release)(void *handle);   /* typed thunk, never a cast deleter */
    void *handle;
    UBYTE side;                      /* 1: AllocMem'd side record */
    UBYTE linked;                    /* 0 after pop or removal */
} cl_res;

static cl_res *cl_res_head;          /* oldest */
static cl_res *cl_res_tail;          /* newest */
static UBYTE cl_exiting;             /* set for the whole hook run */
```

The record layout is an implementation choice inside the approved design (AR-8, AR-9); the
fields above are the minimum the rules need. No record is ever allocated with `malloc` or
`AllocVec` (1.3-clean shared shim; AR-9 rule 5 keeps libc resources out).

### New Functions/Methods

| Function | Contract | AR |
|---|---|---|
| `int cl_res_add(cl_res *node, void (*release)(void *), void *handle)` | Links an embedded node at the tail and installs the hook on first use. Returns 0 only when the hook cannot be installed; the caller then releases what it acquired and returns its existing failure. | AR-8, AR-9 rules 2, 4 |
| `void cl_res_remove(cl_res *node)` | O(1) unlink; a no-op when `node->linked == 0` (already popped or never linked). | AR-9 rules 1, 3 |
| `int cl_res_track(void (*release)(void *), void *handle)` | Allocates a side record (`AllocMem`, `MEMF_PUBLIC \| MEMF_CLEAR`), links it, installs the hook on first use. Returns 0 on allocation or install failure (nothing linked). | AR-8, AR-9 rule 4 |
| `void cl_res_untrack(void (*release)(void *), void *handle)` | Finds the newest side record with this thunk and handle (search from the tail), unlinks and frees it; a no-op when absent. The thunk identifies the kind, so equal pointer values of two kinds never collide. | AR-9 rule 3, AR-20 |
| `void *cl_lib_open(void *slot, const char *name, ULONG version)` | `slot` is `&Base`. Reads the base with `memcpy` (strict aliasing is on at `-O2`; all struct pointers share one representation). A non-null base is returned unchanged and not recorded. Otherwise opens the library, installs the hook, records the base in the library table, writes it back with `memcpy`, and returns it. On install or table failure it closes the library and returns NULL. | AR-9 rules 4, 9 |
| `static void cl_exit_hook(void)` | The `atexit` function; §Exit phases. | AR-2, AR-8 |

Hook installation happens once, on the first `cl_res_add`, `cl_res_track`, `cl_lib_open`,
`cl_add_handler`, `cl_add_port_handler`, or `cl_create_task` (AR-8, AR-18). A program that never
reaches one links none of the registry in a release build (`-ffunction-sections`,
`--gc-sections`; `compiler/src/clorraine/toolchain.rs:70-75`).

### Exit phases

`cl_exit_hook` sets `cl_exiting = 1` and runs, in order (AR-8):

1. **Interrupt servers.** For each of the two static slots with a live handler: `RemIntServer`,
   clear the handler and token (the existing `cl_remove_handler` effect). Servers run
   asynchronously against program memory, so they go first.
2. **Workers.** Every worker record still alive is removed without waiting
   ([03-03 §Exit removal](03-03-worker-faults.md#exit-removal)) (AR-9 rule 6).
3. **Registry.** Repeat: pop the tail record (unlink, `linked = 0`), copy `release`, `handle`,
   and `side` to locals, `FreeMem` a side record, then call `release(handle)`. Never hold a saved
   `next` pointer: a release may unlink other records (AR-9 rule 1). Reverse acquisition order
   releases dependents first (console and GadTools before their window; sprites before their
   screen; a Display after its flips) (AR-8 challenger evidence).
4. **Chip state.** CLXCON is written 0 if `enableCollisions` ran in this program (AR-14).
5. **Library bases.** Close the recorded bases in reverse open order, popping each entry before
   `CloseLibrary` (AR-9 rule 9).

The hook prints nothing on its own, except a worker fault discovered in phase 2 (AR-10). On a
clean exit it prints nothing (AR-9 rule 10).

### Re-entry

A fault or panic while the hook runs (a release function that traps E1, a fault in a release
path) takes the unchanged `cl_trap`/`cl_panic` path: it prints its own line and calls
`exit(20)`. libnix `exit` and `_exit` share one entry. That entry resets the stack and re-runs
the current exit priority, and `__exitatexit` walks its list without unlinking, so the nested
`exit` calls `cl_exit_hook` again. The re-run drains the remaining records. The hook is
idempotent because every phase pops or marks before it acts, so nothing runs twice (AR-9 rule 1;
libnix `startup/ncrt0.S:58-69,86-108` and `nix/stdlib/atexit.c:28-33`). `cl_trap` and `cl_panic`
therefore never call or name `cl_exit_hook`; a direct call would link the whole hook into every
program that has a fault site (AR-18, ST-9). `cl_exiting` is set for the whole hook run. While it
is set, no shim wait includes the break bit ([03-04 §During the exit](03-04-ctrl-c.md#during-the-exit)).

### Library bases

- Every lazy open site in every shim file calls `cl_lib_open` instead of `OpenLibrary` directly
  (the site table is in [03-02 §Library bases](03-02-registration-sites.md#library-bases)).
- The library table is a static array sized to the number of distinct shim-owned base globals;
  each base is recorded at most once because `cl_lib_open` records only on a null-to-open
  transition.
- A base libnix (or vbcc's startup) opened is non-null at the call and is never recorded, so
  `UtilityBase` stays libnix's under GCC and is recorded only where the shim opens it (vbcc)
  (AR-9 rule 9).
- The user `Library` handle (`openLibrary`) is an owned handle in the registry, separate from the
  table ([03-02](03-02-registration-sites.md)).

### Chip state

- The sprite attachment that set `ViewPort.Modes |= SPRITES` (`:4849`) restores the flag to its
  value before the first attachment when the last sprite attached to that screen is released, so a
  second attached sprite keeps working (AR-14; release-time rule, part of the Sprite kind in
  [03-02](03-02-registration-sites.md)).
- `cl_sprite_enable_collisions` (`:4993`) sets a static "collisions used" flag that phase 4
  reads (AR-14).
- The DMACON raster-on write (`:4586-4587`) and user-level `std.amiga.custom` writes are not
  restored (AR-14); documented in [03-05 §Docs](03-05-gates-and-docs.md#docs).

### Messages in flight

- `cl_put_msg` removes the message's record (hand-off) and adds the message to an in-flight set
  (a shim-internal list keyed by message pointer) (AR-9 rule 3, AR-15).
- The message leaves the in-flight set when the program gets it back: `getMsg` returns it (as a
  reply on its reply port, or delivered to a program-owned port). From then on it is an owned
  `Message` with its own record. A program-created message is recognised by leaving the
  in-flight set, and its record uses its own thunk (AR-20).
- Replying a program-created message is a hand-off. `cl_reply_msg`, whether called by the
  program or by that record's exit thunk, removes the record and puts the message back in the
  in-flight set before `ReplyMsg`. It is now queued on the program's own reply port, so the
  port release below frees it (AR-21). A message from a foreign sender is replied as today and
  never enters the set.
- At exit, the `MsgPort` release first unlinks and frees every in-flight message still queued on
  that port (the program owns both ends; nobody else can reply) (AR-21). It then deletes the port
  unless an in-flight message still held by a foreign port names it as `mn_ReplyPort`. In that
  case the port and those messages are left allocated (AR-15).
- ARexx packets in `cl_rexx_lifetimes` whose reply has not arrived follow the same rule: they and
  their reply port stay allocated; all other ARexx records release normally (AR-15).

### Contract for later plans

The register/unregister contract for R5 and R9 (AR-16) is: acquire, then `cl_res_add` or
`cl_res_track` after success, with a typed `static void cl_release_<kind>(void *)` thunk. Undo
the acquisition and return the existing failure when registration fails. Unregister at the real
free or hand-off point. Never assume an order other than reverse acquisition. Phases 1, 2, 4, and
5 are closed to new kinds. This paragraph becomes the comment block above the registry in the
shim (03-05 §Docs).

### Integration Points

- Every acquire and release site of the 52 registered `std.amiga` handle kinds; the 53rd,
  `Task`, uses the worker phase ([03-02](03-02-registration-sites.md)).
- `cl_trap`/`cl_panic` ([03-03](03-03-worker-faults.md), §Re-entry) and the break path
  ([03-04](03-04-ctrl-c.md)).

## Code Examples

### Example 1: a raw-pointer kind (DOS `Lock`)

```c
static void cl_release_lock(void *handle)
{
    UnLock((BPTR)handle);
}

BPTR cl_dos_lock(char *path)
{
    BPTR lock;

    if (path == NULL) {
        return 0;
    }
    lock = Lock(path, SHARED_LOCK);
    if (lock != 0 && !cl_res_track(cl_release_lock, (void *)lock)) {
        UnLock(lock);
        return 0;
    }
    return lock;
}

void cl_dos_unlock(BPTR lock)
{
    if (lock != 0) {
        cl_res_untrack(cl_release_lock, (void *)lock);
        UnLock(lock);
    }
}
```

The `BPTR`↔`void *` conversion is the integer-to-pointer form the shim already uses for handles;
the executor confirms it builds warning-free on GCC and vbcc (AR-9 rule 2).

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Side-record allocation fails at an acquire | Release the just-acquired resource; return the API's existing failure (null, or E1 where it already traps) | AR-9 rule 4 |
| `atexit` install fails | Same as above, for the first acquisition | AR-9 rule 4 |
| Library table full | Close the library; return NULL | AR-9 rule 4 |
| A release traps during the walk | Print its line and `exit(20)`; libnix re-runs the idempotent hook, which drains the rest | AR-9 rule 1 |
| A release keeps its object (`p96CloseScreen`, `DeleteLayer` failure) | Its record is already popped; the object is leaked as today; walk continues | AR-9 rule 1 |
| Port with an in-flight message held by a foreign port | Port and those messages stay allocated | AR-15 |

## Testing Requirements

- Spec: ST-1..ST-9 (including ST-3b), ST-10..ST-13, ST-17 in [07](07-testing-strategy.md).
- Impl: registry list operations, pop-before-release, idempotent hook, `cl_lib_open` recording
  rules, in-flight set ([07 §Implementation Tests](07-testing-strategy.md#implementation-tests-edge-cases-internals)).
