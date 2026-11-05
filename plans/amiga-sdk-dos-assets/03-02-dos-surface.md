# Design: DOS File, Lock, Path, and Volume Surface

> **Document**: 03-02-dos-surface.md
> **Parent**: [Index](00-index.md)
> **Covers**: R1–R5; AR-7–AR-10

## Module surface

`std.amiga.dos` stays a hosted-Amiga module and imports `File` from `std.io`.

```text
export const SEEK_SET: i32 = 0;
export const SEEK_CUR: i32 = 1;
export const SEEK_END: i32 = 2;

export func seekFile(borrow f: File, offset: i32, whence: i32): bool
export func filePosition(borrow f: File): i32
export func fileLength(borrow f: File): i32

export handle Lock freedBy cl_dos_unlock;
export func lockPath(path: cstring): Lock?
export func unlockPath(take lock: Lock): void

export handle PathInfo freedBy cl_dos_pathinfo_free;
export func pathInfo(path: cstring): PathInfo?
export func pathInfoIsDir(const info: PathInfo): bool
export func pathInfoSize(const info: PathInfo): i32
export func pathInfoName(const info: PathInfo, buf: u8[]): i32

export handle VolumeInfo freedBy cl_dos_volumeinfo_free;
export func volumeInfo(path: cstring): VolumeInfo?
export func volumeName(const info: VolumeInfo, buf: u8[]): i32
export func volumeFreeBytes(const info: VolumeInfo): i32
export func volumeUsedBytes(const info: VolumeInfo): i32

export func makeDir(path: cstring): bool
export func deleteFile(path: cstring): bool
export func renameFile(old_path: cstring, new_path: cstring): bool
export func setCurrentDir(path: cstring): bool
export func currentDirName(buf: u8[]): i32
export func parentDirName(buf: u8[]): i32
```

`Lock` is already the contract name that maps to `BPTR`
(`compiler/src/abi.rs:16`), so the generated prototype matches `dos.library`.

**Assigns.** `SDK-DOS-003` covers path, assign, and volume information. AmigaDOS
resolves an assign (for example `SYS:`) inside `Lock`/`Open`; the module gets
that behavior through every path call. Enumerating the assign list needs
`LockDosList` (2.0+), which is outside this 1.3 row.

## File seek

The `File` handle is a libc `FILE *` behind `BPTR`, so seek uses the libc
primitive on the same handle the existing read/write wrappers use:

| Call | Implementation |
|---|---|
| `seekFile` | map 0/1/2 to `SEEK_SET`/`CUR`/`END`, `fseek`, then report `ftell >= 0` |
| `filePosition` | `ftell`, or -1 on failure |
| `fileLength` | save `ftell`, seek to end, read `ftell`, restore the saved position |

A failed `fileLength` returns -1 and makes a best effort to restore the
position. The wrappers do not close the file; the caller's `File` drop does.

## Locks and path information

`lockPath` calls `Lock(path, SHARED_LOCK)` and returns the `BPTR` as the owned
`Lock`; `unlockPath` calls `UnLock`. A zero lock is `null`.

`pathInfo` allocates one handle that owns a `FileInfoBlock`:

```text
lock = Lock(path, SHARED_LOCK)
if lock == 0: return null
if !Examine(lock, &fib):
    UnLock(lock)
    return null
UnLock(lock)
store fib in the handle
```

`pathInfoIsDir` reads `fib_DirEntryType > 0`; `pathInfoSize` reads `fib_Size`;
`pathInfoName` copies `min(name_len, buf.len)` bytes of `fib_FileName` into the
caller buffer (no NUL is written; the copied count is returned). The handle
keeps no lock, so a later rename or delete cannot leave a dangling DOS lock.

## Directory and file mutation

| Call | Implementation |
|---|---|
| `makeDir` | `CreateDir`; unlock the returned directory lock and report success |
| `deleteFile` | `DeleteFile` (a file or an empty directory) |
| `renameFile` | `Rename(old, new)` |
| `setCurrentDir` | `Lock(path, SHARED_LOCK)`, `CurrentDir`, unlock the old lock |
| `currentDirName` | `Lock("", SHARED_LOCK)`, `Examine`, copy `fib_FileName`, unlock |
| `parentDirName` | current lock, `ParentDir`, `Examine`, copy, unlock both |

`CurrentDir` returns the previous directory lock, which may be zero for the boot
volume root; zero is a valid old lock and is not unlocked.

## Volume information

`volumeInfo` allocates one handle that owns an `InfoData` and a copied name.
The lock stays held until every read from it is finished:

```text
lock = Lock(path, SHARED_LOCK)
if lock == 0: return null
if !Info(lock, &id):
    UnLock(lock)
    return null
copy id
if id.id_VolumeNode != 0:
    node = BADDR(id.id_VolumeNode)
    if node != null and node->dn_Name != 0:
        read BCPL name and copy min(length, name_cap)
if the copied name is empty:
    if Examine(lock, &fib):
        copy min(fib name length, name_cap)
UnLock(lock)
```

The BCPL name is read through `BADDR`; its first byte is the length and the
characters follow. The DOS list is never retained. `volumeName` copies the
stored bytes into the caller buffer; `volumeFreeBytes` returns
`(id_NumBlocks - id_NumBlocksUsed) * id_BytesPerBlock`; `volumeUsedBytes`
returns `id_NumBlocksUsed * id_BytesPerBlock`. The name copy is bounded by both
the source length and the caller buffer length.

## Errors

- A failed lock, examine, info, create, delete, or rename returns `null`/`false`.
- Every name copy returns `min(source_len, buf.len)`; a zero-length buffer
  returns 0 and writes nothing.
- Every handle deleter is null-safe and releases exactly one DOS allocation or
  lock.
- The module never writes to a path it did not receive from the caller.
