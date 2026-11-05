/* Hosted-Amiga runtime shim for generated Clorraine C.
 *
 * The shim has the same surface as the host shim but carries the AmigaOS ABI
 * types (`BPTR`, `APTR`, `LONG`, `ULONG`, and the rest) that `exec/types.h`
 * and `dos/dos.h` declare. It contains no ownership logic: it only provides
 * the helpers the emitted translation unit calls and the C-side VBlank server
 * the standard-library interrupt dispatcher installs.
 *
 * The IO, memory, and text helpers use the libc the toolchain's libnix runtime
 * supplies (`nix20` for `hosted-amiga`, `nix13` for `hosted-amiga-13`), matching
 * the platform norm. The shim also calls `dos.library` (delay), `exec.library`
 * (ports, messages, interrupts), `graphics.library`, and `intuition.library`.
 * The DOS `Delay` primitive is declared here with the standard-argument
 * attribute the library requires rather than including `proto/dos.h`, whose
 * prototypes for `Open`, `Close`, `Read`, and `Write` would conflict with the
 * ABI prototypes the standard library declares and the shim defines.
 */

#include <exec/types.h>
#include <exec/interrupts.h>
#include <exec/io.h>
#include <exec/memory.h>
#include <exec/ports.h>
#include <exec/tasks.h>
#include <hardware/intbits.h>
#include <proto/exec.h>
/* `CreatePort`/`DeletePort` are amiga.lib helpers, not exec.library vectors;
 * the NDK declares them here rather than in proto/exec.h. */
#include <clib/alib_protos.h>
#include <proto/graphics.h>
#include <proto/intuition.h>
#include <intuition/intuition.h>
#include <intuition/screens.h>
#include <graphics/rastport.h>
#include <graphics/gfx.h>
#include <graphics/gfxbase.h>
#include <graphics/gfxmacros.h>
#include <graphics/layers.h>
#include <graphics/sprite.h>
#include <graphics/text.h>
#include <graphics/view.h>
#include <hardware/custom.h>
#include <devices/audio.h>
#include <devices/timer.h>
#include <proto/diskfont.h>
#include <proto/layers.h>
/* `dos/dos.h` declares the `BPTR` file-handle type the ABI mapping uses; the
 * generated code emits no AmigaOS typedef of its own, so the shim must supply
 * it. `stdbool.h` supplies the C99 `bool` the Clorraine boolean lowers to. */
#include <dos/dos.h>
#include <dos/filehandler.h>
#include <limits.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The hosted-amiga profile provides the 2.0+ Workbench, icon, and expansion
 * surface. The hosted-amiga-13 build compiles this source without the macro,
 * so the section is preprocessed out and the 1.3 NDK never sees the 2.0-only
 * headers or symbols. */
#if defined(CL_HOSTED_AMIGA_20)
#include <clib/icon_protos.h>

/* The icon library base used by the clib stubs. */
struct Library *IconBase;

/* Open icon.library 37 once and publish its base. */
static int cl_icon_open(void)
{
    if (IconBase == NULL) {
        IconBase = OpenLibrary((STRPTR)"icon.library", 37);
    }
    return IconBase != NULL;
}

/* Read <name>.info into a fresh DiskObject, or NULL. */
void *cl_get_disk_object(char *name)
{
    if (name == NULL || !cl_icon_open()) {
        return NULL;
    }
    return (void *)GetDiskObject((STRPTR)name);
}

/* Write <name>.info from a DiskObject. The caller keeps the icon alive. */
BOOL cl_put_disk_object(char *name, void *icon)
{
    if (name == NULL || icon == NULL || !cl_icon_open()) {
        return FALSE;
    }
    return PutDiskObject((STRPTR)name, (const struct DiskObject *)icon);
}

/* Free a DiskObject exactly once; null-safe. */
void cl_free_disk_object(void *icon)
{
    if (icon != NULL) {
        FreeDiskObject((struct DiskObject *)icon);
    }
}

/* A fresh default icon of the requested Workbench type, or NULL. */
void *cl_get_def_disk_object(LONG kind)
{
    if (kind < WBDISK || kind > WBAPPICON || !cl_icon_open()) {
        return NULL;
    }
    return (void *)GetDefDiskObject(kind);
}

/* The icon's Workbench type, or 0 for a null handle. */
LONG cl_disk_object_type(void *icon)
{
    const struct DiskObject *object = (const struct DiskObject *)icon;

    return object == NULL ? 0 : (LONG)object->do_Type;
}

/* Set a valid Workbench type; reject a null handle or an invalid kind. */
BOOL cl_set_disk_object_type(void *icon, LONG kind)
{
    struct DiskObject *object = (struct DiskObject *)icon;

    if (object == NULL || kind < WBDISK || kind > WBAPPICON) {
        return FALSE;
    }
    object->do_Type = (UBYTE)kind;
    return TRUE;
}

#include <clib/wb_protos.h>
#include <utility/tagitem.h>

/* The Workbench library base used by the clib stubs. */
struct Library *WorkbenchBase;

/* Open workbench.library 37 and icon.library 37 once. */
static int cl_workbench_open(void)
{
    if (WorkbenchBase == NULL) {
        WorkbenchBase = OpenLibrary((STRPTR)"workbench.library", 37);
    }
    return WorkbenchBase != NULL && cl_icon_open();
}

/* Copy a title for a Workbench record. The caller frees it with the same
 * length. */
static char *cl_workbench_copy_title(const char *title, ULONG *size)
{
    ULONG length = (ULONG)strlen(title) + 1;
    char *copy = (char *)AllocMem(length, MEMF_PUBLIC);

    if (copy == NULL) {
        return NULL;
    }
    memcpy(copy, title, length);
    *size = length;
    return copy;
}

typedef struct cl_app_icon {
    struct AppIcon *icon;
    struct DiskObject *object;
    char *title;
    ULONG title_size;
} cl_app_icon;

typedef struct cl_app_menu_item {
    struct AppMenuItem *item;
    char *title;
    ULONG title_size;
} cl_app_menu_item;

/* Register an AppIcon with a default image at a pinned position. The record
 * owns the title copy and the DiskObject until removal. */
void *cl_add_app_icon(void *port_handle, char *title, ULONG id,
                      ULONG user_data)
{
    struct MsgPort *port = (struct MsgPort *)port_handle;
    struct TagItem tags[2];
    struct DiskObject *object;
    cl_app_icon *record;
    char *title_copy;
    ULONG title_size = 0;

    if (port == NULL || title == NULL || !cl_workbench_open()) {
        return NULL;
    }
    title_copy = cl_workbench_copy_title(title, &title_size);
    if (title_copy == NULL) {
        return NULL;
    }
    object = GetDefDiskObject(WBTOOL);
    if (object == NULL) {
        FreeMem(title_copy, title_size);
        return NULL;
    }
    object->do_Type = 0;
    object->do_CurrentX = 400;
    object->do_CurrentY = 140;
    record = (cl_app_icon *)AllocMem((ULONG)sizeof *record,
                                     MEMF_PUBLIC | MEMF_CLEAR);
    if (record == NULL) {
        FreeDiskObject(object);
        FreeMem(title_copy, title_size);
        return NULL;
    }
    tags[0].ti_Tag = WBAPPICONA_PropagatePosition;
    tags[0].ti_Data = TRUE;
    tags[1].ti_Tag = TAG_DONE;
    tags[1].ti_Data = 0;
    record->icon = AddAppIconA(id, user_data, (STRPTR)title_copy, port, 0,
                               object, tags);
    if (record->icon == NULL) {
        FreeDiskObject(object);
        FreeMem(title_copy, title_size);
        FreeMem(record, (ULONG)sizeof *record);
        return NULL;
    }
    record->object = object;
    record->title = title_copy;
    record->title_size = title_size;
    return record;
}

/* Remove an AppIcon and release its DiskObject and title copy. */
void cl_remove_app_icon(void *handle)
{
    cl_app_icon *record = (cl_app_icon *)handle;

    if (record == NULL) {
        return;
    }
    if (record->icon != NULL) {
        (void)RemoveAppIcon(record->icon);
    }
    if (record->object != NULL) {
        FreeDiskObject(record->object);
    }
    if (record->title != NULL) {
        FreeMem(record->title, record->title_size);
    }
    FreeMem(record, (ULONG)sizeof *record);
}

/* Register an AppMenuItem with a copied title. */
void *cl_add_app_menu_item(void *port_handle, char *title, ULONG id,
                           ULONG user_data)
{
    struct MsgPort *port = (struct MsgPort *)port_handle;
    cl_app_menu_item *record;
    char *title_copy;
    ULONG title_size = 0;

    if (port == NULL || title == NULL || !cl_workbench_open()) {
        return NULL;
    }
    title_copy = cl_workbench_copy_title(title, &title_size);
    if (title_copy == NULL) {
        return NULL;
    }
    record = (cl_app_menu_item *)AllocMem((ULONG)sizeof *record,
                                          MEMF_PUBLIC | MEMF_CLEAR);
    if (record == NULL) {
        FreeMem(title_copy, title_size);
        return NULL;
    }
    record->item = AddAppMenuItemA(id, user_data, (STRPTR)title_copy, port,
                                   NULL);
    if (record->item == NULL) {
        FreeMem(title_copy, title_size);
        FreeMem(record, (ULONG)sizeof *record);
        return NULL;
    }
    record->title = title_copy;
    record->title_size = title_size;
    return record;
}

/* Remove an AppMenuItem and release its title copy. */
void cl_remove_app_menu_item(void *handle)
{
    cl_app_menu_item *record = (cl_app_menu_item *)handle;

    if (record == NULL) {
        return;
    }
    if (record->item != NULL) {
        (void)RemoveAppMenuItem(record->item);
    }
    if (record->title != NULL) {
        FreeMem(record->title, record->title_size);
    }
    FreeMem(record, (ULONG)sizeof *record);
}

/* Wait for the next AppMessage on a port. */
void *cl_wait_app_message(void *port_handle)
{
    struct MsgPort *port = (struct MsgPort *)port_handle;

    if (port == NULL) {
        return NULL;
    }
    (void)WaitPort(port);
    return (void *)GetMsg(port);
}

/* AppMessage field accessors; a null message reads as zero. */
LONG cl_app_message_type(void *handle)
{
    const struct AppMessage *message = (const struct AppMessage *)handle;

    return message == NULL ? 0 : (LONG)message->am_Type;
}

ULONG cl_app_message_id(void *handle)
{
    const struct AppMessage *message = (const struct AppMessage *)handle;

    return message == NULL ? 0 : message->am_ID;
}

ULONG cl_app_message_user_data(void *handle)
{
    const struct AppMessage *message = (const struct AppMessage *)handle;

    return message == NULL ? 0 : message->am_UserData;
}

LONG cl_app_message_class(void *handle)
{
    const struct AppMessage *message = (const struct AppMessage *)handle;

    return message == NULL ? 0 : (LONG)message->am_Class;
}

LONG cl_app_message_arg_count(void *handle)
{
    const struct AppMessage *message = (const struct AppMessage *)handle;

    return message == NULL ? 0 : message->am_NumArgs;
}

/* Reply an AppMessage exactly once. */
void cl_reply_app_message(void *handle)
{
    if (handle != NULL) {
        ReplyMsg((struct Message *)handle);
    }
}
#endif

/* The DOS timer primitive. The library calling convention needs the
 * standard-argument attribute, so the prototype is written directly instead
 * of including `proto/dos.h`, which would also declare the file functions with
 * signatures the generated ABI prototypes cannot match. */
extern void __stdargs Delay(LONG timeout);
extern void __stdargs DateStamp(struct DateStamp *stamp);

/* The dos.library calls Bundle 5 adds. `proto/dos.h` cannot be included: its
 * `Open`/`Close`/`Read`/`Write` prototypes conflict with the libc-backed ABI
 * functions above, so each call is declared directly with the standard-argument
 * attribute the m68k library vectors require. The names match the NDK
 * prototypes; `char *` is used instead of `STRPTR` so the shim's caller
 * arguments and the declarations agree without a pointer-sign warning. */
extern BPTR __stdargs Lock(char *name, LONG type);
extern void __stdargs UnLock(BPTR lock);
extern LONG __stdargs Examine(BPTR lock, struct FileInfoBlock *fileInfoBlock);
extern LONG __stdargs Info(BPTR lock, struct InfoData *parameterBlock);
extern BPTR __stdargs CreateDir(char *name);
extern LONG __stdargs DeleteFile(char *name);
extern LONG __stdargs Rename(char *oldName, char *newName);
extern BPTR __stdargs CurrentDir(BPTR lock);
extern BPTR __stdargs ParentDir(BPTR lock);

/* Suspends the calling task for `ticks` vertical-blank intervals. */
void cl_delay_ticks(ULONG ticks)
{
    Delay((LONG)ticks);
}

/* Returns the DOS tick counter (1/50 s on PAL) as one 32-bit value. */
ULONG cl_dos_ticks(void)
{
    struct DateStamp stamp;
    DateStamp(&stamp);
    return (ULONG)stamp.ds_Days * 24UL * 60UL * 3000UL +
           (ULONG)stamp.ds_Minute * 3000UL +
           (ULONG)stamp.ds_Tick;
}

/* --- Amiga DOS: file positioning, locks, paths, and volumes. ------------ */

/* A BCPL pointer stores an address divided by four. `InfoData.id_VolumeNode`
 * and `DeviceNode.dn_Name` are BCPL pointers, so they need this conversion
 * before they can be dereferenced as C pointers. */
#define CL_BADDR(value) ((APTR)((ULONG)(value) << 2))

/* The longest volume name a BSTR can carry. */
#define CL_DOS_NAME_CAP 256

/* One captured FileInfoBlock. The lock that filled it is not retained. */
typedef struct cl_dos_pathinfo {
    struct FileInfoBlock fib;
    ULONG name_len;
} cl_dos_pathinfo;

/* One captured InfoData plus the copied volume name and byte totals. */
typedef struct cl_dos_volumeinfo {
    struct InfoData info;
    UBYTE name[CL_DOS_NAME_CAP];
    ULONG name_len;
    LONG free_bytes;
    LONG used_bytes;
} cl_dos_volumeinfo;

/* Bounded text length for a NUL-terminated DOS field. */
static ULONG cl_dos_text_length(const char *text, ULONG maximum)
{
    ULONG length = 0;

    while (length < maximum && text[length] != '\0') {
        length++;
    }
    return length;
}

/* Copy at most `capacity` name bytes, never a NUL. */
static LONG cl_dos_copy_name(APTR buffer, ULONG capacity, const UBYTE *name,
                             ULONG length)
{
    if (buffer == NULL || capacity == 0 || name == NULL) {
        return 0;
    }
    if (length > capacity) {
        length = capacity;
    }
    memcpy(buffer, name, (size_t)length);
    return (LONG)length;
}

/* Convert a block count to bytes without wrapping the signed result. */
static LONG cl_dos_bytes(LONG blocks, LONG bytes_per_block)
{
    if (blocks <= 0 || bytes_per_block <= 0) {
        return 0;
    }
    if (blocks > LONG_MAX / bytes_per_block) {
        return LONG_MAX;
    }
    return blocks * bytes_per_block;
}

/* Move the libc stream behind `file`; the File contract is a FILE pointer. */
BOOL cl_dos_seek(BPTR file, LONG offset, LONG whence)
{
    FILE *stream = (FILE *)(intptr_t)file;
    int origin;

    if (stream == NULL) {
        return FALSE;
    }
    if (whence == 0) {
        origin = SEEK_SET;
    } else if (whence == 1) {
        origin = SEEK_CUR;
    } else if (whence == 2) {
        origin = SEEK_END;
    } else {
        return FALSE;
    }
    return fseek(stream, (long)offset, origin) == 0 ? TRUE : FALSE;
}

/* The current libc stream position, or -1. */
LONG cl_dos_tell(BPTR file)
{
    FILE *stream = (FILE *)(intptr_t)file;
    long position;

    if (stream == NULL) {
        return -1;
    }
    position = ftell(stream);
    if (position < 0) {
        return -1;
    }
    return (LONG)position;
}

/* The stream length, restoring the original position, or -1. */
LONG cl_dos_file_length(BPTR file)
{
    FILE *stream = (FILE *)(intptr_t)file;
    long position;
    long end;

    if (stream == NULL) {
        return -1;
    }
    position = ftell(stream);
    if (position < 0) {
        return -1;
    }
    if (fseek(stream, 0L, SEEK_END) != 0) {
        return -1;
    }
    end = ftell(stream);
    if (end < 0 || fseek(stream, position, SEEK_SET) != 0) {
        return -1;
    }
    return (LONG)end;
}

/* A shared DOS lock, or zero. */
BPTR cl_dos_lock(char *path)
{
    if (path == NULL) {
        return 0;
    }
    return Lock(path, SHARED_LOCK);
}

/* Release a lock; null-safe. */
void cl_dos_unlock(BPTR lock)
{
    if (lock != 0) {
        UnLock(lock);
    }
}

/* Capture one path's FileInfoBlock, releasing the lock before returning. */
void *cl_dos_pathinfo_new(char *path)
{
    BPTR lock;
    cl_dos_pathinfo *info;

    if (path == NULL) {
        return NULL;
    }
    lock = Lock(path, SHARED_LOCK);
    if (lock == 0) {
        return NULL;
    }
    info = (cl_dos_pathinfo *)AllocMem(
        (ULONG)sizeof *info, MEMF_ANY | MEMF_CLEAR);
    if (info == NULL) {
        UnLock(lock);
        return NULL;
    }
    if (!Examine(lock, &info->fib)) {
        FreeMem(info, (ULONG)sizeof *info);
        UnLock(lock);
        return NULL;
    }
    UnLock(lock);
    info->name_len = cl_dos_text_length(
        (const char *)info->fib.fib_FileName,
        (ULONG)sizeof info->fib.fib_FileName);
    return info;
}

/* Release one path-info handle; null-safe. */
void cl_dos_pathinfo_free(void *handle)
{
    if (handle != NULL) {
        FreeMem(handle, (ULONG)sizeof(cl_dos_pathinfo));
    }
}

/* True when the captured entry is a directory. */
BOOL cl_dos_pathinfo_is_dir(void *handle)
{
    const cl_dos_pathinfo *info = (const cl_dos_pathinfo *)handle;

    if (info == NULL) {
        return FALSE;
    }
    return info->fib.fib_DirEntryType > 0 ? TRUE : FALSE;
}

/* The captured entry size in bytes. */
LONG cl_dos_pathinfo_size(void *handle)
{
    const cl_dos_pathinfo *info = (const cl_dos_pathinfo *)handle;

    if (info == NULL) {
        return 0;
    }
    return info->fib.fib_Size;
}

/* Copy the captured entry name, bounded by the caller's buffer. */
LONG cl_dos_pathinfo_name(void *handle, APTR buffer, ULONG capacity)
{
    const cl_dos_pathinfo *info = (const cl_dos_pathinfo *)handle;

    if (info == NULL) {
        return 0;
    }
    return cl_dos_copy_name(buffer, capacity,
                            (const UBYTE *)info->fib.fib_FileName,
                            info->name_len);
}

/* Capture volume information and the volume name. The lock stays held until
 * both the InfoData and the Examine fallback have finished. */
void *cl_dos_volumeinfo_new(char *path)
{
    BPTR lock;
    cl_dos_volumeinfo *info;
    struct FileInfoBlock fib;

    if (path == NULL) {
        return NULL;
    }
    lock = Lock(path, SHARED_LOCK);
    if (lock == 0) {
        return NULL;
    }
    info = (cl_dos_volumeinfo *)AllocMem(
        (ULONG)sizeof *info, MEMF_ANY | MEMF_CLEAR);
    if (info == NULL) {
        UnLock(lock);
        return NULL;
    }
    if (!Info(lock, &info->info)) {
        FreeMem(info, (ULONG)sizeof *info);
        UnLock(lock);
        return NULL;
    }
    if (info->info.id_VolumeNode != 0) {
        struct DeviceNode *node = (struct DeviceNode *)CL_BADDR(
            info->info.id_VolumeNode);
        if (node != NULL && node->dn_Name != 0) {
            const UBYTE *name = (const UBYTE *)CL_BADDR(node->dn_Name);
            if (name != NULL) {
                ULONG length = name[0];
                if (length > CL_DOS_NAME_CAP) {
                    length = CL_DOS_NAME_CAP;
                }
                memcpy(info->name, name + 1, (size_t)length);
                info->name_len = length;
            }
        }
    }
    if (info->name_len == 0 && Examine(lock, &fib)) {
        ULONG length = cl_dos_text_length(
            (const char *)fib.fib_FileName,
            (ULONG)sizeof fib.fib_FileName);
        memcpy(info->name, fib.fib_FileName, (size_t)length);
        info->name_len = length;
    }
    info->free_bytes = cl_dos_bytes(
        info->info.id_NumBlocks - info->info.id_NumBlocksUsed,
        info->info.id_BytesPerBlock);
    info->used_bytes = cl_dos_bytes(info->info.id_NumBlocksUsed,
                                    info->info.id_BytesPerBlock);
    UnLock(lock);
    return info;
}

/* Release one volume-info handle; null-safe. */
void cl_dos_volumeinfo_free(void *handle)
{
    if (handle != NULL) {
        FreeMem(handle, (ULONG)sizeof(cl_dos_volumeinfo));
    }
}

/* Copy the captured volume name, bounded by the caller's buffer. */
LONG cl_dos_volume_name(void *handle, APTR buffer, ULONG capacity)
{
    const cl_dos_volumeinfo *info = (const cl_dos_volumeinfo *)handle;

    if (info == NULL) {
        return 0;
    }
    return cl_dos_copy_name(buffer, capacity, info->name, info->name_len);
}

/* The captured free byte count. */
LONG cl_dos_volume_free(void *handle)
{
    const cl_dos_volumeinfo *info = (const cl_dos_volumeinfo *)handle;

    if (info == NULL) {
        return 0;
    }
    return info->free_bytes;
}

/* The captured used byte count. */
LONG cl_dos_volume_used(void *handle)
{
    const cl_dos_volumeinfo *info = (const cl_dos_volumeinfo *)handle;

    if (info == NULL) {
        return 0;
    }
    return info->used_bytes;
}

/* Create one directory and release the lock CreateDir returns. */
BOOL cl_dos_make_dir(char *path)
{
    BPTR lock;

    if (path == NULL) {
        return FALSE;
    }
    lock = CreateDir(path);
    if (lock == 0) {
        return FALSE;
    }
    UnLock(lock);
    return TRUE;
}

/* Delete one file or empty directory. */
BOOL cl_dos_delete_file(char *path)
{
    if (path == NULL) {
        return FALSE;
    }
    return DeleteFile(path) ? TRUE : FALSE;
}

/* Rename one path. */
BOOL cl_dos_rename(char *old_path, char *new_path)
{
    if (old_path == NULL || new_path == NULL) {
        return FALSE;
    }
    return Rename(old_path, new_path) ? TRUE : FALSE;
}

/* Change the process current directory; the old lock may be zero at the boot
 * volume root and is not unlocked in that case. */
BOOL cl_dos_set_current_dir(char *path)
{
    BPTR lock;
    BPTR old_lock;

    if (path == NULL) {
        return FALSE;
    }
    lock = Lock(path, SHARED_LOCK);
    if (lock == 0) {
        return FALSE;
    }
    old_lock = CurrentDir(lock);
    if (old_lock != 0) {
        UnLock(old_lock);
    }
    return TRUE;
}

/* Copy the current directory name through a temporary lock. */
LONG cl_dos_current_dir_name(APTR buffer, ULONG capacity)
{
    BPTR lock;
    struct FileInfoBlock fib;
    LONG copied = 0;

    if (buffer == NULL || capacity == 0) {
        return 0;
    }
    lock = Lock("", SHARED_LOCK);
    if (lock == 0) {
        return 0;
    }
    if (Examine(lock, &fib)) {
        ULONG length = cl_dos_text_length(
            (const char *)fib.fib_FileName,
            (ULONG)sizeof fib.fib_FileName);
        copied = cl_dos_copy_name(buffer, capacity,
                                  (const UBYTE *)fib.fib_FileName, length);
    }
    UnLock(lock);
    return copied;
}

/* Copy the parent directory name through a current-directory lock. */
LONG cl_dos_parent_dir_name(APTR buffer, ULONG capacity)
{
    BPTR current_lock;
    BPTR parent_lock;
    struct FileInfoBlock fib;
    LONG copied = 0;

    if (buffer == NULL || capacity == 0) {
        return 0;
    }
    current_lock = Lock("", SHARED_LOCK);
    if (current_lock == 0) {
        return 0;
    }
    parent_lock = ParentDir(current_lock);
    if (parent_lock != 0 && Examine(parent_lock, &fib)) {
        ULONG length = cl_dos_text_length(
            (const char *)fib.fib_FileName,
            (ULONG)sizeof fib.fib_FileName);
        copied = cl_dos_copy_name(buffer, capacity,
                                  (const UBYTE *)fib.fib_FileName, length);
    }
    if (parent_lock != 0) {
        UnLock(parent_lock);
    }
    UnLock(current_lock);
    return copied;
}

/* Zero one object through its address. A macro so a call site writes the
 * single argument `cl_zero(&value)`; `sizeof` supplies the byte count. */
#define cl_zero(pointer) memset((void *)(pointer), 0, sizeof(*(pointer)))

/* IEEE-754 NaN test on the raw bit pattern.
 *
 * The platform soft-float comparisons are not IEEE for NaN, so the exponent
 * and mantissa are inspected directly. A `double` parameter accepts a `float`
 * argument through the usual promotion. */
static inline int cl_isnan(double value)
{
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof bits);
    if ((bits & UINT64_C(0x7ff0000000000000)) != UINT64_C(0x7ff0000000000000)) {
        return 0;
    }
    return (bits & UINT64_C(0x000fffffffffffff)) != 0;
}

/* Leave the process after a runtime fault. The Amiga fault contract writes the
 * first line `error CODE: message` to stdout and then exits, so the run-fault
 * leg reads it from the program's standard output. */
static inline void cl_trap(const char *code, const char *message)
{
    printf("error %s: %s\n", code, message);
    fflush(stdout);
    exit(20);
}

/* The byte length of a NUL-terminated C string. The pointer forms of a panic
 * message keep printing their text through the length-based runtime. */
static inline unsigned long cl_cstring_len(const char *text)
{
    return (unsigned long)strlen(text);
}

/* Leave the process after a user-level panic. The message is passed as pointer
 * and length, so a view or handle needs no NUL terminator; the bytes are
 * written exactly as they are. */
static inline void cl_panic(const unsigned char *message, unsigned long len)
{
    fwrite("panic: ", 1, 7, stdout);
    fwrite(message, 1, (size_t)len, stdout);
    fwrite("\n", 1, 1, stdout);
    fflush(stdout);
    exit(20);
}

/* The C-side VBlank dispatcher surface.
 *
 * The standard-library interrupt dispatcher registers a capture-less Clorraine
 * function here. A handler is a plain function pointer, and the server
 * trampoline calls it and returns, so the handler stays allocation-free,
 * drop-free, and panic-free as the interrupt contract requires.
 *
 * There is one server slot per kind, so registering a handler replaces the
 * current one of that kind. Each registration gets a fresh non-zero token from
 * one shared counter, so a token from a replaced registration can never remove
 * the handler installed after it. `cl_remove_handler` clears the live token,
 * which makes a second removal a no-op. */
typedef void (*cl_handler_fn)(void);

static cl_handler_fn cl_vblank_handler;
static struct Interrupt cl_vblank_server;
static ULONG cl_vblank_token;

static cl_handler_fn cl_ports_handler;
static struct Interrupt cl_ports_server;
static ULONG cl_ports_token;

static ULONG cl_next_token = 1;

static void cl_vblank_code(void)
{
    if (cl_vblank_handler != NULL) {
        cl_vblank_handler();
    }
}

static void cl_ports_code(void)
{
    if (cl_ports_handler != NULL) {
        cl_ports_handler();
    }
}

/* Take the next non-zero token so a wrapped counter never names a
 * registration with the token's "no handler" sentinel. */
static ULONG cl_take_token(void)
{
    ULONG token = cl_next_token;

    cl_next_token++;
    if (cl_next_token == 0) {
        cl_next_token = 1;
    }
    return token;
}

/* Installs `handler` as the VBlank server, replacing any current one.
 *
 * A null handler is rejected so an accidental unregister cannot leave a
 * dangling callback. The server primitives return no status on this NDK, so
 * no result is checked. */
ULONG cl_add_handler(cl_handler_fn handler)
{
    if (handler == NULL) {
        return 0;
    }
    if (cl_vblank_handler != NULL) {
        RemIntServer(INTB_VERTB, &cl_vblank_server);
    }
    cl_vblank_handler = handler;
    cl_vblank_token = cl_take_token();
    cl_vblank_server.is_Node.ln_Type = NT_INTERRUPT;
    cl_vblank_server.is_Node.ln_Pri = 0;
    cl_vblank_server.is_Node.ln_Name = (char *)"clorraine.vblank";
    cl_vblank_server.is_Data = NULL;
    cl_vblank_server.is_Code = (VOID (*)())cl_vblank_code;
    AddIntServer(INTB_VERTB, &cl_vblank_server);
    return cl_vblank_token;
}

/* Installs `handler` as the ports interrupt server, replacing any current one.
 * The VBlank slot is independent. */
ULONG cl_add_port_handler(cl_handler_fn handler)
{
    if (handler == NULL) {
        return 0;
    }
    if (cl_ports_handler != NULL) {
        RemIntServer(INTB_PORTS, &cl_ports_server);
    }
    cl_ports_handler = handler;
    cl_ports_token = cl_take_token();
    cl_ports_server.is_Node.ln_Type = NT_INTERRUPT;
    cl_ports_server.is_Node.ln_Pri = 0;
    cl_ports_server.is_Node.ln_Name = (char *)"clorraine.ports";
    cl_ports_server.is_Data = NULL;
    cl_ports_server.is_Code = (VOID (*)())cl_ports_code;
    AddIntServer(INTB_PORTS, &cl_ports_server);
    return cl_ports_token;
}

/* Removes the server whose slot owns `id`. Idempotent: a second removal, a
 * zero token, or a stale token does nothing. The shared counter makes the two
 * live tokens distinct, so at most one slot matches. */
void cl_remove_handler(ULONG id)
{
    if (id == 0) {
        return;
    }
    if (id == cl_vblank_token && cl_vblank_handler != NULL) {
        RemIntServer(INTB_VERTB, &cl_vblank_server);
        cl_vblank_handler = NULL;
        cl_vblank_token = 0;
        return;
    }
    if (id == cl_ports_token && cl_ports_handler != NULL) {
        RemIntServer(INTB_PORTS, &cl_ports_server);
        cl_ports_handler = NULL;
        cl_ports_token = 0;
    }
}

/* Heap allocation with the allocator contract: a failure traps. The size is a
 * `size_t`, so a caller that computes a block size in `size_t` never has it
 * truncated back to the 32-bit `ULONG` an AmigaOS word would hold. */
void *cl_alloc(size_t size)
{
    void *block = malloc(size == 0 ? 1 : size);
    if (block == NULL) {
        cl_trap("E1", "out of memory");
    }
    return block;
}

/* Release a block from `cl_alloc`. Null-safe. */
void cl_free(void *block)
{
    free(block);
}

/* Byte copy and compare helpers the library may bind. */
void *cl_memcpy(void *dst, const void *src, ULONG n)
{
    return memcpy(dst, src, (size_t)n);
}

int cl_memcmp(const void *left, const void *right, ULONG n)
{
    return memcmp(left, right, (size_t)n);
}

/* A Buffer is a stable descriptor: callers hold the descriptor pointer while
 * its byte block may be reallocated, so borrowing the descriptor stays sound.
 * The generated `ClBuffer` typedef is an opaque pointer, so these definitions
 * spell the underlying C types directly; the prototypes the emitter writes
 * later are compatible with them. They are not `static`, because a program
 * that does not use `std.mem` still sees the definitions and must not fail
 * `-Wunused-function`. */
typedef struct cl_buffer_block {
    unsigned char *data;
    ULONG len;
    ULONG cap;
} cl_buffer_block;

/* Allocate a buffer with `size` readable bytes. An allocation failure traps,
 * matching the allocator contract. */
void *cl_buffer_new(ULONG size)
{
    cl_buffer_block *buffer = (cl_buffer_block *)cl_alloc(sizeof *buffer);
    buffer->data = (unsigned char *)cl_alloc(size);
    buffer->len = size;
    buffer->cap = size;
    return buffer;
}

/* Release a buffer's data and descriptor. Null-safe. */
void FreeBuffer(void *handle)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (buffer == NULL) {
        return;
    }
    cl_free(buffer->data);
    cl_free(buffer);
}

/* The first byte of a buffer's storage. */
void *cl_buffer_ptr(void *handle)
{
    return ((cl_buffer_block *)handle)->data;
}

/* The number of live bytes in a buffer. */
ULONG cl_buffer_len(void *handle)
{
    return ((cl_buffer_block *)handle)->len;
}

/* Grow a buffer's capacity to at least `cap` bytes. */
void cl_buffer_reserve(void *handle, ULONG cap)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (cap <= buffer->cap) {
        return;
    }
    ULONG old_cap = buffer->cap;
    unsigned char *grown = (unsigned char *)realloc(buffer->data, (size_t)cap);
    if (grown == NULL) {
        cl_trap("E1", "out of memory");
    }
    /* The grown region is fresh storage; zero it so a container element store
     * reads the canonical empty state instead of allocator garbage. */
    memset(grown + old_cap, 0, (size_t)(cap - old_cap));
    buffer->data = grown;
    buffer->cap = cap;
}

/* Append `n` bytes from `src` to a buffer, growing it as needed. */
void cl_buffer_append(void *handle, APTR src, ULONG n)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (n > ULONG_MAX - buffer->len) {
        cl_trap("E1", "buffer too large");
    }
    ULONG needed = buffer->len + n;
    if (needed > buffer->cap) {
        ULONG cap = buffer->cap == 0 ? 8 : buffer->cap;
        while (cap < needed) {
            if (cap > ULONG_MAX / 2) {
                cap = needed;
                break;
            }
            cap *= 2;
        }
        cl_buffer_reserve(handle, cap);
    }
    memcpy(buffer->data + buffer->len, src, (size_t)n);
    buffer->len = needed;
}

/* Write `n` bytes from `src` at `off`, growing the buffer as needed. The
 * offset and count are both bounded so `off + n` cannot wrap and the copy stays
 * inside the buffer. */
void cl_buffer_write(void *handle, ULONG off, APTR src, ULONG n)
{
    cl_buffer_block *buffer = (cl_buffer_block *)handle;
    if (n > ULONG_MAX - off) {
        cl_trap("E11211", "index out of range");
    }
    ULONG needed = off + n;
    if (needed > buffer->cap) {
        cl_buffer_reserve(handle, needed);
    }
    memcpy(buffer->data + off, src, (size_t)n);
    if (needed > buffer->len) {
        buffer->len = needed;
    }
}

/* A String is one heap block laid out as `[len: u32][cap: u32][bytes][NUL]`.
 * The handle points at the block start, so the payload bytes begin right after
 * the two counts and a trailing NUL keeps them usable as a C string.
 *
 * In-place growth is deferred, and growing the block would move the handle, so
 * a builder reserves the final size in `cl_string_new` and its appends fit.
 * `cl_string_append` therefore refuses to grow rather than leave the caller
 * with a stale handle. */
typedef struct cl_string_block {
    ULONG len;
    ULONG cap;
} cl_string_block;

/* The first payload byte of a string block. */
static unsigned char *cl_string_bytes(cl_string_block *string)
{
    return (unsigned char *)(string + 1);
}

/* Allocate an empty string with room for `cap` payload bytes.
 *
 * The block size is computed in `size_t` so a capacity near the top of the
 * 32-bit range cannot wrap the allocation size down to a short block; a
 * capacity whose block cannot be represented is rejected with the E1 fault. */
void *cl_string_new(ULONG cap)
{
    if ((size_t)cap > SIZE_MAX - sizeof(cl_string_block) - 1) {
        cl_trap("E1", "string too large");
    }
    cl_string_block *string =
        (cl_string_block *)cl_alloc(sizeof *string + (size_t)cap + 1);
    string->len = 0;
    string->cap = cap;
    cl_string_bytes(string)[0] = '\0';
    return string;
}

/* Append `n` bytes from `src`. The caller must have reserved the capacity. */
void cl_string_append(void *handle, APTR src, ULONG n)
{
    cl_string_block *string = (cl_string_block *)handle;
    if (n > string->cap - string->len) {
        cl_trap("E1", "string capacity exceeded");
    }
    memcpy(cl_string_bytes(string) + string->len, src, (size_t)n);
    string->len += n;
    cl_string_bytes(string)[string->len] = '\0';
}

/* The first payload byte of a string; the trailing NUL makes it a C string. */
void *cl_string_ptr(void *handle)
{
    return cl_string_bytes((cl_string_block *)handle);
}

/* The number of live payload bytes in a string. */
ULONG cl_string_len(void *handle)
{
    return ((cl_string_block *)handle)->len;
}

/* Release a string block. Null-safe. */
void FreeString(void *handle)
{
    if (handle != NULL) {
        cl_free(handle);
    }
}

/* Copy `n` bytes into a fresh string of exactly that size. */
static void *cl_string_from(const void *bytes, ULONG n)
{
    void *string = cl_string_new(n);
    cl_string_append(string, (APTR)bytes, n);
    return string;
}

/* Copy a whole string block into a fresh, independent block. The caller owns
 * the copy and releases it through the string deleter. */
void *cl_string_copy(void *handle)
{
    cl_string_block *string = (cl_string_block *)handle;
    return cl_string_from(cl_string_bytes(string), string->len);
}

/* One piece of a template. A literal piece borrows static bytes and has no
 * `owned` handle; a converted placeholder piece owns a freshly built string
 * that the builder releases once the bytes are appended. */
typedef struct cl_str_part {
    const unsigned char *ptr;
    ULONG len;
    void *owned;
} ClStrPart;

/* Wrap an owned string as a template piece. The builder takes the string over
 * and releases it, so the caller must not use the handle afterwards. */
static inline ClStrPart cl_str_part_owned(void *string)
{
    ClStrPart part;
    part.ptr = (const unsigned char *)cl_string_ptr(string);
    part.len = cl_string_len(string);
    part.owned = string;
    return part;
}

/* Concatenate template pieces into one owned string.
 *
 * The total length is the sum of the piece lengths, so the builder reserves
 * exactly that size once and every append fits. Each owned piece is released
 * after its bytes are copied, which keeps a placeholder's converted string
 * from leaking: the compiler does not drop unnamed temporaries. */
void *cl_string_concat(const ClStrPart *parts, ULONG count)
{
    ULONG index;
    ULONG total = 0;
    for (index = 0; index < count; index++) {
        total += parts[index].len;
    }
    void *string = cl_string_new(total);
    for (index = 0; index < count; index++) {
        cl_string_append(string, (APTR)parts[index].ptr, parts[index].len);
        if (parts[index].owned != NULL) {
            cl_free(parts[index].owned);
        }
    }
    return string;
}

/* The text converters build the same String layout the templates desugar to,
 * so a template and an explicit `text` call format a value identically. */
void *cl_text_i32(LONG value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%ld", (long)value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (ULONG)written);
}

void *cl_text_u32(ULONG value)
{
    char buffer[24];
    int written = snprintf(buffer, sizeof buffer, "%lu", (unsigned long)value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (ULONG)written);
}

/* Six significant digits: `%g` is the C default-precision spelling of that. */
void *cl_text_f64(double value)
{
    char buffer[32];
    int written = snprintf(buffer, sizeof buffer, "%g", value);
    if (written < 0) {
        cl_trap("E1", "text formatting failed");
    }
    return cl_string_from(buffer, (ULONG)written);
}

void *cl_text_bool(int value)
{
    static const char truth[] = "true";
    static const char falsehood[] = "false";
    if (value) {
        return cl_string_from(truth, sizeof truth - 1);
    }
    return cl_string_from(falsehood, sizeof falsehood - 1);
}

/* Build a string from a borrowed byte range; the compiler passes a `str` as
 * its pointer and length so no view struct crosses the shim boundary. */
void *cl_text_str(const void *bytes, ULONG n)
{
    return cl_string_from(bytes, n);
}

/* `0x`-prefixed uppercase hex, with no leading zeros beyond the value. */
void *cl_text_hex_u32(ULONG value)
{
    static const char digits[] = "0123456789ABCDEF";
    char buffer[10];
    ULONG count = 1;
    ULONG remaining = value;
    while (remaining >= 16) {
        remaining >>= 4;
        count++;
    }
    buffer[0] = '0';
    buffer[1] = 'x';
    ULONG index = 0;
    while (index < count) {
        ULONG shift = 4 * (count - 1 - index);
        buffer[2 + index] = digits[(value >> shift) & 0x0f];
        index++;
    }
    return cl_string_from(buffer, 2 + count);
}

/* Pad `src` on the left to `width` bytes with `fill`. */
void *cl_text_pad_left(APTR src, ULONG n, ULONG width, UBYTE fill)
{
    ULONG total = width > n ? width : n;
    void *string = cl_string_new(total);
    ULONG pad = total - n;
    while (pad > 0) {
        cl_string_append(string, &fill, 1);
        pad--;
    }
    cl_string_append(string, src, n);
    return string;
}

/* Pad `src` on the right to `width` bytes with `fill`. */
void *cl_text_pad_right(APTR src, ULONG n, ULONG width, UBYTE fill)
{
    ULONG total = width > n ? width : n;
    void *string = cl_string_new(total);
    ULONG pad = total - n;
    cl_string_append(string, src, n);
    while (pad > 0) {
        cl_string_append(string, &fill, 1);
        pad--;
    }
    return string;
}

/* A file handle is a `BPTR`, an integer word on this ABI, while the libc the
 * shim mirrors names its stream with a pointer. The pointer round-trips through
 * `intptr_t`, which is the same width, so no information is lost. The generated
 * code spells the `File` contract handle as `BPTR` and compares it with zero. */
#define CL_MODE_NEWFILE 1006

/* Open `path` for reading or writing; a failure returns zero. */
BPTR Open(char *path, LONG mode)
{
    const char *spelling = (mode == CL_MODE_NEWFILE) ? "wb" : "rb";
    FILE *file = fopen(path, spelling);
    return (BPTR)(intptr_t)file;
}

/* Close a file handle. Null-safe, because the generated drop is null-safe but
 * a caller may also close an already-empty handle. */
void Close(BPTR file)
{
    if (file != 0) {
        fclose((FILE *)(intptr_t)file);
    }
}

/* Read up to `length` bytes into `buffer` and return the count moved. */
LONG Read(BPTR file, APTR buffer, LONG length)
{
    return (LONG)fread(buffer, 1, (size_t)length, (FILE *)(intptr_t)file);
}

/* Write `length` bytes from `buffer` and return the count moved. */
LONG Write(BPTR file, APTR buffer, LONG length)
{
    return (LONG)fwrite(buffer, 1, (size_t)length, (FILE *)(intptr_t)file);
}

/* Write `n` bytes to standard output without a trailing newline. */
void cl_write(APTR buffer, ULONG n)
{
    if (n > 0) {
        fwrite(buffer, 1, (size_t)n, stdout);
    }
}

/* Write `n` bytes to standard output followed by a newline. */
void cl_write_line(APTR buffer, ULONG n)
{
    if (n > 0) {
        fwrite(buffer, 1, (size_t)n, stdout);
    }
    fputc('\n', stdout);
}

/* --- Amiga graphics: a custom screen and drawing calls. -------------------
 *
 * The `Screen` contract handle is an opaque pointer; `cl_open_screen` returns
 * the `intuition.library` screen or NULL. The drawing helpers take that handle
 * back and drive the `graphics.library` RastPort calls. The two library bases
 * are opened on the first screen open and stay open for the life of the task. */

struct GfxBase *GfxBase = NULL;
struct IntuitionBase *IntuitionBase = NULL;
struct Library *DiskfontBase = NULL;
struct Library *LayersBase = NULL;

/* The sprite module clears attached channels when their screen closes. */
static void cl_sprite_screen_closing(struct Screen *screen);

/* Open a custom screen; NULL on failure. */
void *cl_open_screen(LONG width, LONG height, UBYTE depth, char *title)
{
    if (GfxBase == NULL) {
        GfxBase = (struct GfxBase *)OpenLibrary((STRPTR) "graphics.library", 33);
    }
    if (IntuitionBase == NULL) {
        IntuitionBase =
            (struct IntuitionBase *)OpenLibrary((STRPTR) "intuition.library", 33);
    }
    if (GfxBase == NULL || IntuitionBase == NULL) {
        return NULL;
    }

    struct NewScreen screen;
    memset(&screen, 0, sizeof screen);
    screen.LeftEdge = 0;
    screen.TopEdge = 0;
    screen.Width = (WORD)width;
    screen.Height = (WORD)height;
    screen.Depth = depth;
    screen.DetailPen = 0;
    screen.BlockPen = 1;
    screen.ViewModes = 0;
    screen.Type = CUSTOMSCREEN;
    screen.Font = NULL;
    /* Intuition keeps the `DefaultTitle` pointer in the opened screen's
     * `Screen.Title` field; it does not copy the text. The caller must
     * therefore keep the title string alive for as long as the screen is open. */
    screen.DefaultTitle = (STRPTR)title;
    screen.Gadgets = NULL;
    screen.CustomBitMap = NULL;
    return (void *)OpenScreen(&screen);
}

/* Set screen color `pen` from 4-bit R/G/B components. */
void cl_set_color(void *screen_handle, UBYTE pen, UBYTE r, UBYTE g, UBYTE b)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL && pen < 32) {
        SetRGB4(&screen->ViewPort, (LONG)pen, r >> 4, g >> 4, b >> 4);
    }
}

/* Fill the rectangle (`x1`,`y1`)-(`x2`,`y2`) with color `pen`. */
void cl_fill(void *screen_handle, LONG x1, LONG y1, LONG x2, LONG y2, UBYTE pen)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        SetAPen(&screen->RastPort, (ULONG)pen);
        RectFill(&screen->RastPort, x1, y1, x2, y2);
    }
}

/* Draw the line (`x1`,`y1`)-(`x2`,`y2`) with color `pen`: Move then Draw. */
void cl_draw(void *screen_handle, LONG x1, LONG y1, LONG x2, LONG y2, UBYTE pen)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        SetAPen(&screen->RastPort, (ULONG)pen);
        Move(&screen->RastPort, x1, y1);
        Draw(&screen->RastPort, x2, y2);
    }
}

/* Fill the whole screen with color `pen`. */
void cl_clear(void *screen_handle, UBYTE pen)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        SetAPen(&screen->RastPort, (ULONG)pen);
        RectFill(&screen->RastPort, 0, 0, (LONG)screen->Width - 1,
                 (LONG)screen->Height - 1);
    }
}

/* Close a screen. Null-safe. Detaches any sprite channels first. */
void cl_close_screen(void *screen_handle)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    if (screen != NULL) {
        cl_sprite_screen_closing(screen);
        CloseScreen(screen);
    }
}

/* --- Amiga Intuition: windows and the IDCMP event loop. -------------------
 *
 * A `Window` contract handle is a private `cl_window` wrapper, not the raw
 * `struct Window *`: the wrapper owns the copied title, the gadget and menu
 * nodes, the pointer image, and the last decoded event. The window borrows the
 * caller's screen; the caller must close the window before the screen. */

#define CL_WINDOW_TEXT_MAX 64
#define CL_WINDOW_BODY_MAX 28
#define CL_ALERT_MESSAGE_MAX (CL_WINDOW_BODY_MAX + 5)
#define CL_WINDOW_POINTER_BYTES 64
#define CL_MENU_HEIGHT 11
#define CL_MENU_WIDTH 48

typedef struct cl_window_event {
    LONG kind;        /* EVENT_* */
    LONG code;        /* raw IntuiMessage.Code */
    UWORD qualifier;
    LONG mouse_x;     /* inner-window coordinate */
    LONG mouse_y;
    LONG gadget;      /* user id, 0 when none */
    LONG menu;        /* user id, 0 when none */
} cl_window_event;

typedef struct cl_gadget_node {
    struct cl_gadget_node *next;
    struct Gadget *gadget;
    struct Image *normal_image;   /* button only */
    struct Image *select_image;   /* button only */
    struct Border *box;           /* string only */
    WORD *box_xy;                 /* string only */
    struct IntuiText *label;
    char *label_text;
    struct StringInfo *string_info; /* string only */
    char *string_buffer;            /* string only */
    ULONG string_buffer_size;       /* string only */
    LONG id;
} cl_gadget_node;

typedef struct cl_menu_item_node {
    struct cl_menu_item_node *next;
    struct MenuItem *item;
    struct IntuiText *label;
    char *label_text;
    LONG id;
} cl_menu_item_node;

typedef struct cl_menu_node {
    struct cl_menu_node *next;
    struct Menu *menu;
    cl_menu_item_node *items;
    char *title;
} cl_menu_node;

typedef struct cl_window {
    struct Window *window;
    char *title;
    UWORD *pointer;
    int pointer_shown;
    cl_gadget_node *gadgets;
    cl_menu_node *menus;
    cl_window_event last_event;
    int console_refs;   /* open console.device handles on this window */
    int close_pending;  /* the Window handle closed while a console was open */
} cl_window;

static void cl_window_free_gadgets(cl_window *handle);
static void cl_window_free_menus(cl_window *handle);
static cl_gadget_node *cl_window_find_gadget(cl_window *handle, LONG id);
static cl_menu_node *cl_window_find_menu(cl_window *handle, LONG index);
static int cl_window_id_taken(cl_window *handle, LONG id);

/* Copy a NUL-terminated string with a hard byte bound. Returns NULL when the
 * source is null or longer than `max`; the allocation is `alloc_size` bytes and
 * is always freed with the same size. */
static char *cl_window_copy_text(const char *text, ULONG max, ULONG alloc_size)
{
    ULONG length = 0;
    char *copy;

    if (text == NULL) {
        return NULL;
    }
    while (text[length] != '\0') {
        length++;
        if (length > max) {
            return NULL;
        }
    }
    copy = (char *)AllocMem(alloc_size, MEMF_CLEAR);
    if (copy == NULL) {
        return NULL;
    }
    memcpy(copy, text, length);
    copy[length] = '\0';
    return copy;
}

/* Open a window on a custom screen. Returns NULL on invalid geometry, an
 * over-long title, or an Intuition failure. The title is copied. */
void *cl_open_window(void *screen_handle, LONG x, LONG y, LONG width,
                     LONG height, char *title)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    cl_window *handle;
    struct NewWindow nw;
    char *title_copy;

    if (screen == NULL || title == NULL) {
        return NULL;
    }
    if (x < 0 || y < 0 || width <= 0 || height <= 0) {
        return NULL;
    }
    if (width > (LONG)screen->Width || height > (LONG)screen->Height) {
        return NULL;
    }
    if (x > (LONG)screen->Width - width || y > (LONG)screen->Height - height) {
        return NULL;
    }
    title_copy =
        cl_window_copy_text(title, CL_WINDOW_TEXT_MAX, CL_WINDOW_TEXT_MAX + 1);
    if (title_copy == NULL) {
        return NULL;
    }
    handle = (cl_window *)AllocMem((ULONG)sizeof *handle, MEMF_CLEAR);
    if (handle == NULL) {
        FreeMem(title_copy, CL_WINDOW_TEXT_MAX + 1);
        return NULL;
    }
    handle->title = title_copy;

    memset(&nw, 0, sizeof nw);
    nw.LeftEdge = (WORD)x;
    nw.TopEdge = (WORD)y;
    nw.Width = (WORD)width;
    nw.Height = (WORD)height;
    nw.DetailPen = 0;
    nw.BlockPen = 1;
    nw.IDCMPFlags = IDCMP_CLOSEWINDOW | IDCMP_GADGETUP | IDCMP_MENUPICK
                  | IDCMP_MOUSEBUTTONS | IDCMP_RAWKEY | IDCMP_VANILLAKEY
                  | IDCMP_ACTIVEWINDOW | IDCMP_INACTIVEWINDOW
                  | IDCMP_REFRESHWINDOW;
    nw.Flags = WFLG_DRAGBAR | WFLG_DEPTHGADGET | WFLG_CLOSEGADGET
             | WFLG_ACTIVATE | WFLG_GIMMEZEROZERO;
    nw.FirstGadget = NULL;
    nw.CheckMark = NULL;
    nw.Title = (STRPTR)title_copy;
    nw.Screen = screen;
    nw.BitMap = NULL;
    nw.MinWidth = 0;
    nw.MinHeight = 0;
    nw.MaxWidth = 0;
    nw.MaxHeight = 0;
    nw.Type = CUSTOMSCREEN;

    handle->window = OpenWindow(&nw);
    if (handle->window == NULL) {
        FreeMem(title_copy, CL_WINDOW_TEXT_MAX + 1);
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    /* WFLG_ACTIVATE normally suffices, but an explicit activation makes the
     * menu strip visible on a custom screen even when another window had the
     * screen's input focus at open time. */
    WindowToFront(handle->window);
    ActivateWindow(handle->window);
    return handle;
}

/* Release the wrapper-owned allocations and the Intuition window. */
static void cl_window_destroy(cl_window *handle)
{
    if (handle == NULL) {
        return;
    }
    if (handle->window != NULL) {
        if (handle->pointer_shown) {
            ClearPointer(handle->window);
        }
        if (handle->window->MenuStrip != NULL) {
            ClearMenuStrip(handle->window);
        }
        CloseWindow(handle->window);
    }
    cl_window_free_gadgets(handle);
    cl_window_free_menus(handle);
    if (handle->pointer != NULL) {
        FreeMem(handle->pointer, CL_WINDOW_POINTER_BYTES);
    }
    if (handle->title != NULL) {
        FreeMem(handle->title, CL_WINDOW_TEXT_MAX + 1);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

/* Close a window. Null-safe. A window still referenced by an open console
 * device defers the actual close until the last console reference is
 * released. */
void cl_close_window(void *window_handle)
{
    cl_window *handle = (cl_window *)window_handle;

    if (handle == NULL) {
        return;
    }
    if (handle->console_refs > 0) {
        handle->close_pending = 1;
        return;
    }
    cl_window_destroy(handle);
}

/* Drop one console reference; the last reference performs a deferred close. */
void cl_window_release(cl_window *handle)
{
    if (handle == NULL) {
        return;
    }
    if (handle->console_refs > 0) {
        handle->console_refs--;
    }
    if (handle->console_refs == 0 && handle->close_pending) {
        cl_window_destroy(handle);
    }
}

/* Inner drawable width, or 0 on a null window. */
LONG cl_window_width(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;

    if (handle == NULL || handle->window == NULL) {
        return 0;
    }
    return (LONG)handle->window->GZZWidth;
}

/* Inner drawable height, or 0 on a null window. */
LONG cl_window_height(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;

    if (handle == NULL || handle->window == NULL) {
        return 0;
    }
    return (LONG)handle->window->GZZHeight;
}

/* Fill an inner-window rectangle. Null-safe. */
void cl_window_fill(void *window_handle, LONG x1, LONG y1, LONG x2, LONG y2,
                    UBYTE pen)
{
    cl_window *handle = (cl_window *)window_handle;

    if (handle == NULL || handle->window == NULL) {
        return;
    }
    SetAPen(handle->window->RPort, (ULONG)pen);
    RectFill(handle->window->RPort, x1, y1, x2, y2);
}

/* Draw an inner-window line. Null-safe. */
void cl_window_draw(void *window_handle, LONG x1, LONG y1, LONG x2, LONG y2,
                    UBYTE pen)
{
    cl_window *handle = (cl_window *)window_handle;

    if (handle == NULL || handle->window == NULL) {
        return;
    }
    SetAPen(handle->window->RPort, (ULONG)pen);
    Move(handle->window->RPort, x1, y1);
    Draw(handle->window->RPort, x2, y2);
}

/* Fill the whole inner window. Null-safe. */
void cl_window_clear(void *window_handle, UBYTE pen)
{
    cl_window *handle = (cl_window *)window_handle;

    if (handle == NULL || handle->window == NULL) {
        return;
    }
    SetAPen(handle->window->RPort, (ULONG)pen);
    RectFill(handle->window->RPort, 0, 0,
             (LONG)handle->window->GZZWidth - 1,
             (LONG)handle->window->GZZHeight - 1);
}

/* --- Intuition gadgets: buttons and one string field. -------------------
 *
 * Each node owns its classic `struct Gadget` and every render/label record
 * Intuition reads. The window closes before the nodes are freed, so the
 * Intuition list never points at released storage. */

static cl_gadget_node *cl_window_find_gadget(cl_window *handle, LONG id)
{
    cl_gadget_node *node;

    if (handle == NULL) {
        return NULL;
    }
    for (node = handle->gadgets; node != NULL; node = node->next) {
        if (node->id == id) {
            return node;
        }
    }
    return NULL;
}

static int cl_window_id_taken(cl_window *handle, LONG id)
{
    cl_menu_node *menu;

    if (cl_window_find_gadget(handle, id) != NULL) {
        return 1;
    }
    if (handle == NULL) {
        return 0;
    }
    for (menu = handle->menus; menu != NULL; menu = menu->next) {
        cl_menu_item_node *item;
        for (item = menu->items; item != NULL; item = item->next) {
            if (item->id == id) {
                return 1;
            }
        }
    }
    return 0;
}

static int cl_window_gadget_geometry_ok(cl_window *handle, LONG x, LONG y,
                                        LONG width, LONG height)
{
    if (handle == NULL || handle->window == NULL) {
        return 0;
    }
    if (x < 0 || y < 0 || width <= 0 || height <= 0) {
        return 0;
    }
    if (width > (LONG)handle->window->GZZWidth
        || height > (LONG)handle->window->GZZHeight) {
        return 0;
    }
    if (x > (LONG)handle->window->GZZWidth - width) {
        return 0;
    }
    if (y > (LONG)handle->window->GZZHeight - height) {
        return 0;
    }
    return 1;
}

static void cl_window_free_gadget(cl_gadget_node *node)
{
    if (node == NULL) {
        return;
    }
    if (node->string_buffer != NULL && node->string_buffer_size > 0) {
        FreeMem(node->string_buffer, node->string_buffer_size);
    }
    if (node->string_info != NULL) {
        FreeMem(node->string_info, (ULONG)sizeof *node->string_info);
    }
    if (node->box_xy != NULL) {
        FreeMem(node->box_xy, (ULONG)(10 * sizeof(WORD)));
    }
    if (node->box != NULL) {
        FreeMem(node->box, (ULONG)sizeof *node->box);
    }
    if (node->normal_image != NULL) {
        FreeMem(node->normal_image, (ULONG)sizeof *node->normal_image);
    }
    if (node->select_image != NULL) {
        FreeMem(node->select_image, (ULONG)sizeof *node->select_image);
    }
    if (node->label != NULL) {
        FreeMem(node->label, (ULONG)sizeof *node->label);
    }
    if (node->label_text != NULL) {
        FreeMem(node->label_text, CL_WINDOW_TEXT_MAX + 1);
    }
    if (node->gadget != NULL) {
        FreeMem(node->gadget, (ULONG)sizeof *node->gadget);
    }
    FreeMem(node, (ULONG)sizeof *node);
}

static void cl_window_free_gadgets(cl_window *handle)
{
    cl_gadget_node *node;

    if (handle == NULL) {
        return;
    }
    node = handle->gadgets;
    while (node != NULL) {
        cl_gadget_node *next = node->next;
        cl_window_free_gadget(node);
        node = next;
    }
    handle->gadgets = NULL;
}

static void cl_window_append_gadget(cl_window *handle, cl_gadget_node *node)
{
    cl_gadget_node **link = &handle->gadgets;

    while (*link != NULL) {
        link = &(*link)->next;
    }
    *link = node;
}

/* Append a boolean button. Returns FALSE on validation or allocation
 * failure; the caller's label is copied. */
BOOL cl_window_add_button(void *window_handle, LONG x, LONG y, LONG width,
                          LONG height, char *label, LONG id)
{
    cl_window *handle = (cl_window *)window_handle;
    cl_gadget_node *node;
    struct Gadget *gadget;
    struct Image *normal;
    struct Image *select;
    struct IntuiText *text;
    char *label_text;

    if (handle == NULL || handle->window == NULL || label == NULL || id < 1) {
        return FALSE;
    }
    if (!cl_window_gadget_geometry_ok(handle, x, y, width, height)) {
        return FALSE;
    }
    if (cl_window_id_taken(handle, id)) {
        return FALSE;
    }
    label_text =
        cl_window_copy_text(label, CL_WINDOW_TEXT_MAX, CL_WINDOW_TEXT_MAX + 1);
    if (label_text == NULL) {
        return FALSE;
    }
    node = (cl_gadget_node *)AllocMem((ULONG)sizeof *node, MEMF_CLEAR);
    if (node == NULL) {
        FreeMem(label_text, CL_WINDOW_TEXT_MAX + 1);
        return FALSE;
    }
    node->label_text = label_text;
    node->id = id;
    gadget = (struct Gadget *)AllocMem((ULONG)sizeof *gadget, MEMF_CLEAR);
    if (gadget == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->gadget = gadget;
    normal = (struct Image *)AllocMem((ULONG)sizeof *normal, MEMF_CLEAR);
    if (normal == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->normal_image = normal;
    select = (struct Image *)AllocMem((ULONG)sizeof *select, MEMF_CLEAR);
    if (select == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->select_image = select;
    text = (struct IntuiText *)AllocMem((ULONG)sizeof *text, MEMF_CLEAR);
    if (text == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->label = text;

    normal->LeftEdge = 0;
    normal->TopEdge = 0;
    normal->Width = (WORD)width;
    normal->Height = (WORD)height;
    normal->Depth = 1;
    normal->ImageData = NULL;
    normal->PlanePick = 0;
    normal->PlaneOnOff = 1;
    normal->NextImage = NULL;
    *select = *normal;
    select->PlaneOnOff = 2;

    text->FrontPen = 0;
    text->BackPen = 1;
    text->DrawMode = JAM1;
    text->LeftEdge = 0;
    text->TopEdge = 0;
    text->ITextFont = NULL;
    text->IText = (STRPTR)label_text;
    text->NextText = NULL;

    gadget->NextGadget = NULL;
    gadget->LeftEdge = (WORD)x;
    gadget->TopEdge = (WORD)y;
    gadget->Width = (WORD)width;
    gadget->Height = (WORD)height;
    gadget->Flags = GFLG_GADGIMAGE | GFLG_GADGHIMAGE;
    gadget->Activation = GACT_RELVERIFY;
    gadget->GadgetType = GTYP_BOOLGADGET;
    gadget->GadgetRender = normal;
    gadget->SelectRender = select;
    gadget->GadgetText = text;
    gadget->MutualExclude = 0;
    gadget->SpecialInfo = NULL;
    gadget->GadgetID = (UWORD)id;
    gadget->UserData = NULL;

    AddGList(handle->window, gadget, (ULONG)-1, 1, NULL);
    RefreshGList(gadget, handle->window, NULL, 1);
    cl_window_append_gadget(handle, node);
    return TRUE;
}

/* Append a string gadget. Returns FALSE on validation or allocation failure;
 * the label and initial text are copied. */
BOOL cl_window_add_string(void *window_handle, LONG x, LONG y, LONG width,
                          LONG height, char *label, char *initial,
                          LONG max_chars, LONG id)
{
    cl_window *handle = (cl_window *)window_handle;
    cl_gadget_node *node;
    struct Gadget *gadget;
    struct Border *box;
    struct IntuiText *text;
    struct StringInfo *info;
    char *label_text;
    char *string_buffer;
    WORD *box_xy;

    if (handle == NULL || handle->window == NULL || label == NULL
        || initial == NULL || id < 1) {
        return FALSE;
    }
    if (max_chars < 1 || max_chars > 64) {
        return FALSE;
    }
    if (!cl_window_gadget_geometry_ok(handle, x, y, width, height)) {
        return FALSE;
    }
    if (cl_window_id_taken(handle, id)) {
        return FALSE;
    }
    label_text =
        cl_window_copy_text(label, CL_WINDOW_TEXT_MAX, CL_WINDOW_TEXT_MAX + 1);
    if (label_text == NULL) {
        return FALSE;
    }
    string_buffer = cl_window_copy_text(initial, (ULONG)max_chars,
                                        (ULONG)max_chars + 1);
    if (string_buffer == NULL) {
        FreeMem(label_text, CL_WINDOW_TEXT_MAX + 1);
        return FALSE;
    }
    node = (cl_gadget_node *)AllocMem((ULONG)sizeof *node, MEMF_CLEAR);
    if (node == NULL) {
        FreeMem(label_text, CL_WINDOW_TEXT_MAX + 1);
        FreeMem(string_buffer, (ULONG)max_chars + 1);
        return FALSE;
    }
    node->label_text = label_text;
    node->string_buffer = string_buffer;
    node->string_buffer_size = (ULONG)max_chars + 1;
    node->id = id;
    gadget = (struct Gadget *)AllocMem((ULONG)sizeof *gadget, MEMF_CLEAR);
    if (gadget == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->gadget = gadget;
    box = (struct Border *)AllocMem((ULONG)sizeof *box, MEMF_CLEAR);
    if (box == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->box = box;
    box_xy = (WORD *)AllocMem((ULONG)(10 * sizeof(WORD)), MEMF_CLEAR);
    if (box_xy == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->box_xy = box_xy;
    text = (struct IntuiText *)AllocMem((ULONG)sizeof *text, MEMF_CLEAR);
    if (text == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->label = text;
    info = (struct StringInfo *)AllocMem((ULONG)sizeof *info, MEMF_CLEAR);
    if (info == NULL) {
        cl_window_free_gadget(node);
        return FALSE;
    }
    node->string_info = info;

    box_xy[0] = 0;
    box_xy[1] = 0;
    box_xy[2] = (WORD)(width - 1);
    box_xy[3] = 0;
    box_xy[4] = (WORD)(width - 1);
    box_xy[5] = (WORD)(height - 1);
    box_xy[6] = 0;
    box_xy[7] = (WORD)(height - 1);
    box_xy[8] = 0;
    box_xy[9] = 0;
    box->LeftEdge = 0;
    box->TopEdge = 0;
    box->FrontPen = 1;
    box->BackPen = 0;
    box->DrawMode = JAM2;
    box->Count = 5;
    box->XY = box_xy;
    box->NextBorder = NULL;

    text->FrontPen = 1;
    text->BackPen = 0;
    text->DrawMode = JAM1;
    text->LeftEdge = 0;
    text->TopEdge = 0;
    text->ITextFont = NULL;
    text->IText = (STRPTR)label_text;
    text->NextText = NULL;

    info->Buffer = (STRPTR)string_buffer;
    info->UndoBuffer = NULL;
    info->BufferPos = 0;
    info->MaxChars = (WORD)(max_chars + 1);
    info->DispPos = 0;
    info->NumChars = (WORD)strlen(string_buffer);
    info->UndoPos = 0;
    info->DispCount = 0;
    info->CLeft = 0;
    info->CTop = 0;
    info->Extension = NULL;

    gadget->NextGadget = NULL;
    gadget->LeftEdge = (WORD)x;
    gadget->TopEdge = (WORD)y;
    gadget->Width = (WORD)width;
    gadget->Height = (WORD)height;
    gadget->Flags = 0;
    /* Kickstart only sends IDCMP_GADGETUP for a string gadget that has
     * GACT_RELVERIFY set; Return otherwise deactivates the field silently.
     * GACT_IMMEDIATE keeps the field active from mouse-down. */
    gadget->Activation = GACT_IMMEDIATE | GACT_RELVERIFY;
    gadget->GadgetType = GTYP_STRGADGET;
    gadget->GadgetRender = box;
    gadget->SelectRender = box;
    gadget->GadgetText = text;
    gadget->MutualExclude = 0;
    gadget->SpecialInfo = info;
    gadget->GadgetID = (UWORD)id;
    gadget->UserData = NULL;

    AddGList(handle->window, gadget, (ULONG)-1, 1, NULL);
    RefreshGList(gadget, handle->window, NULL, 1);
    cl_window_append_gadget(handle, node);
    return TRUE;
}

/* Copy the current string field into the caller's buffer. Never trusts the
 * Intuition-edited buffer's NUL; the length comes from StringInfo.NumChars and
 * is clamped to MaxChars - 1 and the caller's capacity. */
LONG cl_window_string(void *window_handle, LONG id, APTR buffer, ULONG capacity)
{
    cl_window *handle = (cl_window *)window_handle;
    cl_gadget_node *node;
    LONG length;
    UBYTE *out = (UBYTE *)buffer;

    if (handle == NULL || buffer == NULL || capacity == 0) {
        return 0;
    }
    node = cl_window_find_gadget(handle, id);
    if (node == NULL || node->string_info == NULL) {
        return 0;
    }
    length = (LONG)node->string_info->NumChars;
    if (length < 0) {
        length = 0;
    }
    if (length > (LONG)node->string_info->MaxChars - 1) {
        length = (LONG)node->string_info->MaxChars - 1;
    }
    if ((ULONG)length > capacity) {
        length = (LONG)capacity;
    }
    memcpy(out, node->string_info->Buffer, (size_t)length);
    if ((ULONG)length < capacity) {
        out[length] = '\0';
    }
    return length;
}

/* --- Intuition menus: one classic menu strip. ---------------------------
 *
 * The window owns the menu headers and items. Every add reattaches the strip
 * so Intuition lays out the new chain; a failed attach unlinks and frees the
 * new node before the previous strip is restored. */

static cl_menu_node *cl_window_find_menu(cl_window *handle, LONG index)
{
    cl_menu_node *menu;
    LONG current = 0;

    if (handle == NULL || index < 0) {
        return NULL;
    }
    for (menu = handle->menus; menu != NULL; menu = menu->next) {
        if (current == index) {
            return menu;
        }
        current++;
    }
    return NULL;
}

static void cl_window_free_menu_item(cl_menu_item_node *node)
{
    if (node == NULL) {
        return;
    }
    if (node->label_text != NULL) {
        FreeMem(node->label_text, CL_WINDOW_TEXT_MAX + 1);
    }
    if (node->label != NULL) {
        FreeMem(node->label, (ULONG)sizeof *node->label);
    }
    if (node->item != NULL) {
        FreeMem(node->item, (ULONG)sizeof *node->item);
    }
    FreeMem(node, (ULONG)sizeof *node);
}

static void cl_window_free_menus(cl_window *handle)
{
    cl_menu_node *menu;

    if (handle == NULL) {
        return;
    }
    menu = handle->menus;
    while (menu != NULL) {
        cl_menu_node *next_menu = menu->next;
        cl_menu_item_node *item = menu->items;
        while (item != NULL) {
            cl_menu_item_node *next_item = item->next;
            cl_window_free_menu_item(item);
            item = next_item;
        }
        if (menu->title != NULL) {
            FreeMem(menu->title, CL_WINDOW_TEXT_MAX + 1);
        }
        if (menu->menu != NULL) {
            FreeMem(menu->menu, (ULONG)sizeof *menu->menu);
        }
        FreeMem(menu, (ULONG)sizeof *menu);
        menu = next_menu;
    }
    handle->menus = NULL;
}

/* Detach the strip before the menu chain is modified. Clearing an attached
 * strip after a NextMenu/NextItem change would let Intuition walk a mutating
 * list. */
static void cl_window_menu_detach(cl_window *handle)
{
    if (handle != NULL && handle->window != NULL
        && handle->window->MenuStrip != NULL) {
        ClearMenuStrip(handle->window);
    }
}

/* Return the first menu with at least one item, or NULL. Intuition does not
 * accept an empty FirstItem chain, so empty headers are skipped. */
static struct Menu *cl_window_menu_head(cl_window *handle)
{
    cl_menu_node *node;

    if (handle == NULL) {
        return NULL;
    }
    for (node = handle->menus; node != NULL; node = node->next) {
        if (node->menu->FirstItem != NULL) {
            return node->menu;
        }
    }
    return NULL;
}

/* Rebuild the Menu.NextMenu chain from the non-empty menus. */
static void cl_window_menu_link(cl_window *handle)
{
    cl_menu_node *node;
    struct Menu *previous = NULL;

    if (handle == NULL) {
        return;
    }
    for (node = handle->menus; node != NULL; node = node->next) {
        if (node->menu->FirstItem == NULL) {
            node->menu->NextMenu = NULL;
            continue;
        }
        if (previous != NULL) {
            previous->NextMenu = node->menu;
        }
        previous = node->menu;
    }
    if (previous != NULL) {
        previous->NextMenu = NULL;
    }
}

/* Attach the current chain. Only called with a detached strip. */
static BOOL cl_window_menu_set(cl_window *handle)
{
    struct Menu *head;

    if (handle == NULL || handle->window == NULL) {
        return FALSE;
    }
    head = cl_window_menu_head(handle);
    if (head == NULL) {
        return FALSE;
    }
    return SetMenuStrip(handle->window, head);
}

/* Append a menu header. Returns its zero-based index, or -1 on failure.
 *
 * The header is not attached until it has at least one item: Intuition does
 * not accept a menu with an empty FirstItem chain. The header is linked into
 * the Menu.NextMenu chain by the first addMenuItem call. */
LONG cl_window_add_menu(void *window_handle, char *title)
{
    cl_window *handle = (cl_window *)window_handle;
    cl_menu_node *node;
    cl_menu_node *tail;
    struct Menu *menu;
    char *title_copy;
    LONG index = 0;

    if (handle == NULL || handle->window == NULL || title == NULL) {
        return -1;
    }
    title_copy =
        cl_window_copy_text(title, CL_WINDOW_TEXT_MAX, CL_WINDOW_TEXT_MAX + 1);
    if (title_copy == NULL) {
        return -1;
    }
    node = (cl_menu_node *)AllocMem((ULONG)sizeof *node, MEMF_CLEAR);
    if (node == NULL) {
        FreeMem(title_copy, CL_WINDOW_TEXT_MAX + 1);
        return -1;
    }
    menu = (struct Menu *)AllocMem((ULONG)sizeof *menu, MEMF_CLEAR);
    if (menu == NULL) {
        FreeMem(title_copy, CL_WINDOW_TEXT_MAX + 1);
        FreeMem(node, (ULONG)sizeof *node);
        return -1;
    }
    node->menu = menu;
    node->title = title_copy;
    menu->MenuName = (STRPTR)title_copy;
    menu->Flags = MENUENABLED;
    menu->LeftEdge = (WORD)(index * CL_MENU_WIDTH);
    menu->Width = CL_MENU_WIDTH;
    menu->Height = CL_MENU_HEIGHT;
    menu->FirstItem = NULL;
    menu->NextMenu = NULL;

    tail = handle->menus;
    if (tail == NULL) {
        handle->menus = node;
    } else {
        index = 1;
        while (tail->next != NULL) {
            tail = tail->next;
            index++;
        }
        tail->next = node;
    }
    return index;
}

/* Append a textual, enabled menu item. Returns FALSE on failure. */
static BOOL cl_window_add_menu_item_internal(void *window_handle,
                                            LONG menu_index, char *label,
                                            LONG id, LONG key, BOOL has_key)
{
    cl_window *handle = (cl_window *)window_handle;
    cl_menu_node *menu;
    cl_menu_item_node *node;
    cl_menu_item_node *tail;
    struct MenuItem *item;
    struct IntuiText *text;
    char *label_text;

    if (handle == NULL || handle->window == NULL || label == NULL || id < 1) {
        return FALSE;
    }
    menu = cl_window_find_menu(handle, menu_index);
    if (menu == NULL) {
        return FALSE;
    }
    if (cl_window_id_taken(handle, id)) {
        return FALSE;
    }
    label_text =
        cl_window_copy_text(label, CL_WINDOW_TEXT_MAX, CL_WINDOW_TEXT_MAX + 1);
    if (label_text == NULL) {
        return FALSE;
    }
    node = (cl_menu_item_node *)AllocMem((ULONG)sizeof *node, MEMF_CLEAR);
    if (node == NULL) {
        FreeMem(label_text, CL_WINDOW_TEXT_MAX + 1);
        return FALSE;
    }
    node->label_text = label_text;
    node->id = id;
    item = (struct MenuItem *)AllocMem((ULONG)sizeof *item, MEMF_CLEAR);
    if (item == NULL) {
        cl_window_free_menu_item(node);
        return FALSE;
    }
    node->item = item;
    text = (struct IntuiText *)AllocMem((ULONG)sizeof *text, MEMF_CLEAR);
    if (text == NULL) {
        cl_window_free_menu_item(node);
        return FALSE;
    }
    node->label = text;

    text->FrontPen = 1;
    text->BackPen = 0;
    text->DrawMode = JAM1;
    text->LeftEdge = 0;
    text->TopEdge = 0;
    text->ITextFont = NULL;
    text->IText = (STRPTR)label_text;
    text->NextText = NULL;

    item->NextItem = NULL;
    item->Width = CL_MENU_WIDTH;
    item->Height = CL_MENU_HEIGHT;
    item->Flags = ITEMTEXT | ITEMENABLED;
    if (has_key) {
        item->Flags |= COMMSEQ;
        item->Command = (BYTE)key;
    }
    item->MutualExclude = 0;
    item->ItemFill = text;
    item->SelectFill = NULL;
    item->SubItem = NULL;
    item->NextSelect = 0;

    cl_window_menu_detach(handle);
    if (menu->items == NULL) {
        menu->items = node;
        menu->menu->FirstItem = item;
    } else {
        tail = menu->items;
        while (tail->next != NULL) {
            tail = tail->next;
        }
        tail->next = node;
        tail->item->NextItem = item;
    }
    cl_window_menu_link(handle);
    if (!cl_window_menu_set(handle)) {
        if (menu->items == node) {
            menu->items = NULL;
            menu->menu->FirstItem = NULL;
        } else {
            cl_menu_item_node *previous = menu->items;
            while (previous->next != NULL && previous->next != node) {
                previous = previous->next;
            }
            if (previous->next == node) {
                previous->next = NULL;
                previous->item->NextItem = NULL;
            }
        }
        cl_window_menu_link(handle);
        (void)cl_window_menu_set(handle);
        cl_window_free_menu_item(node);
        return FALSE;
    }
    return TRUE;
}

BOOL cl_window_add_menu_item(void *window_handle, LONG menu_index, char *label,
                             LONG id)
{
    return cl_window_add_menu_item_internal(window_handle, menu_index, label,
                                            id, 0, FALSE);
}

BOOL cl_window_add_menu_item_key(void *window_handle, LONG menu_index,
                                 char *label, LONG id, UBYTE key)
{
    return cl_window_add_menu_item_internal(window_handle, menu_index, label,
                                            id, (LONG)key, TRUE);
}

/* Activate a string or custom gadget so the keyboard goes to it. */
BOOL cl_window_activate_gadget(void *window_handle, LONG id)
{
    cl_window *handle = (cl_window *)window_handle;
    cl_gadget_node *node;

    if (handle == NULL || handle->window == NULL) {
        return FALSE;
    }
    node = cl_window_find_gadget(handle, id);
    if (node == NULL || node->gadget == NULL) {
        return FALSE;
    }
    return ActivateGadget(node->gadget, handle->window, NULL);
}

/* --- Intuition requesters, alerts, and the built-in pointer. ------------- */

/* Show a modal Yes/No requester and return TRUE for Yes. The body is copied
 * for the duration of the blocking call. */
BOOL cl_window_ask_yes_no(void *window_handle, char *body)
{
    cl_window *handle = (cl_window *)window_handle;
    struct IntuiText body_text;
    struct IntuiText yes_text;
    struct IntuiText no_text;
    char *body_copy;
    BOOL result;

    if (handle == NULL || handle->window == NULL || body == NULL) {
        return FALSE;
    }
    body_copy =
        cl_window_copy_text(body, CL_WINDOW_BODY_MAX, CL_WINDOW_BODY_MAX + 1);
    if (body_copy == NULL) {
        return FALSE;
    }

    memset(&body_text, 0, sizeof body_text);
    body_text.FrontPen = 1;
    body_text.BackPen = 0;
    body_text.DrawMode = JAM2;
    body_text.IText = (STRPTR)body_copy;

    memset(&yes_text, 0, sizeof yes_text);
    yes_text.FrontPen = 1;
    yes_text.BackPen = 0;
    yes_text.DrawMode = JAM2;
    yes_text.IText = (STRPTR) "Yes";

    memset(&no_text, 0, sizeof no_text);
    no_text.FrontPen = 1;
    no_text.BackPen = 0;
    no_text.DrawMode = JAM2;
    no_text.IText = (STRPTR) "No";

    /* The 1.3 AutoRequest uses these dimensions directly; 0,0 does not
     * auto-size (the V36 note says the parameters are ignored). The IDCMP
     * flags are 0; they are not pen flags. */
    result = AutoRequest(handle->window, &body_text, &yes_text, &no_text,
                         0, 0, 240, 40);
    FreeMem(body_copy, CL_WINDOW_BODY_MAX + 1);
    return result;
}

/* Show a modal recovery alert. The call blocks until the user clicks
 * Continue; a null or over-long body is a no-op.
 *
 * DisplayAlert takes an AlertMessage: a 16-bit x, an 8-bit y, the text, a
 * terminating NUL, and a zero continuation byte (two NULs total). The fixed
 * header places the text at (20,20). */
void cl_show_alert(char *body)
{
    char *message;
    ULONG length = 0;

    if (body == NULL) {
        return;
    }
    while (body[length] != '\0') {
        length++;
        if (length > CL_WINDOW_BODY_MAX) {
            return;
        }
    }
    message = (char *)AllocMem(CL_ALERT_MESSAGE_MAX, MEMF_CLEAR);
    if (message == NULL) {
        return;
    }
    message[0] = 0;
    message[1] = 20;
    message[2] = 20;
    memcpy(message + 3, body, length);
    /* MEMF_CLEAR leaves the text NUL at 3+length and the continuation NUL at
     * 3+length+1. */
    (void)DisplayAlert(1, (STRPTR)message, 20);
    FreeMem(message, CL_ALERT_MESSAGE_MAX);
}

/* A 16x16, two-plane pointer. Color 0 is transparent, color 1 is the white
 * interior (plane 0), and color 2 is the black outline (plane 1). Intuition
 * reads 16 rows of two words: plane 0 then plane 1. */
static const UWORD cl_window_pointer_template[32] = {
    0x0000, 0x8000, 0x0000, 0xC000, 0x4000, 0xA000, 0x6000, 0x9000,
    0x7000, 0x8800, 0x7800, 0x8400, 0x7C00, 0x8200, 0x7E00, 0x8100,
    0x7F00, 0x8080, 0x7F80, 0x8040, 0x7FC0, 0x8020, 0x7FE0, 0x8010,
    0x7E00, 0x81F0, 0x6C00, 0x9200, 0x4600, 0xA900, 0x0600, 0xC900,
};

/* Install or clear the built-in pointer. The image is allocated in chip memory
 * on first show and kept until the window closes. */
void cl_window_set_pointer(void *window_handle, BOOL show)
{
    cl_window *handle = (cl_window *)window_handle;

    if (handle == NULL || handle->window == NULL) {
        return;
    }
    if (!show) {
        if (handle->pointer_shown) {
            ClearPointer(handle->window);
            handle->pointer_shown = 0;
        }
        return;
    }
    if (handle->pointer == NULL) {
        handle->pointer = (UWORD *)AllocMem(CL_WINDOW_POINTER_BYTES,
                                            MEMF_CHIP | MEMF_CLEAR);
        if (handle->pointer == NULL) {
            return;
        }
        memcpy(handle->pointer, cl_window_pointer_template,
               CL_WINDOW_POINTER_BYTES);
    }
    SetPointer(handle->window, handle->pointer, 16, 16, 0, 0);
    handle->pointer_shown = 1;
}

/* Native screen-capture format written by cl_screen_save_native:
 *
 *   0  "CSCR"
 *   4  u16 width          (big-endian)
 *   6  u16 height
 *   8  u16 depth
 *  10  u16 bytes_per_row
 *  12  palette: (1 << depth) RGB triples, 8 bits per component
 *  ..  bitplanes: depth * bytes_per_row * height bytes
 *
 * The capture stores the raw screen bitmap, so it is the native Amiga image
 * and does not depend on the host window size or emulator scaling. Keeping the
 * bitplanes instead of expanded pixels makes the file small enough to write to
 * a floppy without stalling the UI. */
BOOL cl_screen_save_native(void *screen_handle, char *path)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    struct BitMap *bitmap;
    struct ColorMap *colormap;
    UBYTE header[12];
    UBYTE rgb[3];
    UBYTE *table;
    FILE *file;
    UWORD width;
    UWORD height;
    UWORD depth;
    UWORD bytes_per_row;
    UWORD colors;
    UWORD count;
    UWORD index;
    WORD plane;
    ULONG plane_bytes;
    BOOL ok = TRUE;

    if (screen == NULL || path == NULL) {
        return FALSE;
    }
    bitmap = &screen->BitMap;
    colormap = screen->ViewPort.ColorMap;
    if (colormap == NULL || colormap->ColorTable == NULL) {
        return FALSE;
    }
    if (screen->Width <= 0 || screen->Height <= 0) {
        return FALSE;
    }
    if (bitmap->Depth < 1 || bitmap->Depth > 8
        || bitmap->BytesPerRow == 0) {
        return FALSE;
    }
    table = (UBYTE *)colormap->ColorTable;
    width = (UWORD)screen->Width;
    height = (UWORD)screen->Height;
    depth = (UWORD)bitmap->Depth;
    bytes_per_row = (UWORD)bitmap->BytesPerRow;
    colors = (UWORD)(1U << depth);
    count = colormap->Count < colors ? colormap->Count : colors;
    plane_bytes = (ULONG)bytes_per_row * (ULONG)height;

    header[0] = 'C';
    header[1] = 'S';
    header[2] = 'C';
    header[3] = 'R';
    header[4] = (UBYTE)(width >> 8);
    header[5] = (UBYTE)width;
    header[6] = (UBYTE)(height >> 8);
    header[7] = (UBYTE)height;
    header[8] = (UBYTE)(depth >> 8);
    header[9] = (UBYTE)depth;
    header[10] = (UBYTE)(bytes_per_row >> 8);
    header[11] = (UBYTE)bytes_per_row;

    file = fopen(path, "wb");
    if (file == NULL) {
        return FALSE;
    }
    if (fwrite(header, 1, (ULONG)sizeof header, file) != sizeof header) {
        ok = FALSE;
    }
    for (index = 0; ok && index < colors; index++) {
        if (index < count) {
            if (colormap->Type == 0) {
                /* Kickstart 1.3 stores each color as a big-endian 12-bit
                 * 0x0RGB word, not as three separate bytes. */
                UWORD color = (UWORD)(((UWORD)table[index * 2] << 8)
                                      | (UWORD)table[index * 2 + 1]);
                rgb[0] = (UBYTE)(((color >> 8) & 0x0F) * 17);
                rgb[1] = (UBYTE)(((color >> 4) & 0x0F) * 17);
                rgb[2] = (UBYTE)((color & 0x0F) * 17);
            } else {
                /* V36 and later use three bytes of four-bit components. */
                rgb[0] = (UBYTE)((table[index * 3] & 0x0F) * 17);
                rgb[1] = (UBYTE)((table[index * 3 + 1] & 0x0F) * 17);
                rgb[2] = (UBYTE)((table[index * 3 + 2] & 0x0F) * 17);
            }
        } else {
            rgb[0] = 0;
            rgb[1] = 0;
            rgb[2] = 0;
        }
        if (fwrite(rgb, 1, 3, file) != 3) {
            ok = FALSE;
        }
    }
    for (plane = 0; ok && plane < (WORD)depth; plane++) {
        UBYTE *data = (UBYTE *)bitmap->Planes[plane];
        if (data == NULL) {
            ok = FALSE;
        } else if (fwrite(data, 1, plane_bytes, file) != plane_bytes) {
            ok = FALSE;
        }
    }
    if (fclose(file) != 0) {
        ok = FALSE;
    }
    return ok;
}

#if defined(CL_HOSTED_AMIGA_20)
/* Register an AppWindow on the caller's window. The window must outlive the
 * AppWindow; the wrapper does not close it. */
void *cl_add_app_window(void *window_handle, void *port_handle, ULONG id,
                        ULONG user_data)
{
    const cl_window *window = (const cl_window *)window_handle;
    struct MsgPort *port = (struct MsgPort *)port_handle;

    if (window == NULL || window->window == NULL || port == NULL
        || !cl_workbench_open()) {
        return NULL;
    }
    return (void *)AddAppWindowA(id, user_data, window->window, port, NULL);
}

/* Remove an AppWindow; null-safe. */
void cl_remove_app_window(void *handle)
{
    if (handle != NULL) {
        (void)RemoveAppWindow((struct AppWindow *)handle);
    }
}

/* Capture a public screen by name to a native CSCR file. */
BOOL cl_save_public_screen(char *name, char *path)
{
    struct Screen *screen;
    BOOL ok;

    if (name == NULL || path == NULL) {
        return FALSE;
    }
    screen = LockPubScreen((STRPTR)name);
    if (screen == NULL) {
        return FALSE;
    }
    ok = cl_screen_save_native((void *)screen, path);
    UnlockPubScreen(NULL, screen);
    return ok;
}
#endif

/* EVENT_* values shared with std.amiga.intuition. */
#define CL_EVENT_NONE 0
#define CL_EVENT_CLOSE 1
#define CL_EVENT_GADGET 2
#define CL_EVENT_MENU 3
#define CL_EVENT_MOUSE 4
#define CL_EVENT_KEY 5
#define CL_EVENT_ACTIVE 6
#define CL_EVENT_INACTIVE 7

/* Resolve a gadget IAddress to the caller's id. */
static LONG cl_window_gadget_id(cl_window *handle, APTR address)
{
    cl_gadget_node *node;

    if (handle == NULL) {
        return 0;
    }
    for (node = handle->gadgets; node != NULL; node = node->next) {
        if ((APTR)node->gadget == address) {
            return node->id;
        }
    }
    return 0;
}

/* Resolve a menu number to the caller's item id. */
static LONG cl_window_menu_id(cl_window *handle, ULONG code)
{
    struct MenuItem *item;
    cl_menu_node *menu;

    if (handle == NULL || handle->window == NULL
        || handle->window->MenuStrip == NULL) {
        return 0;
    }
    if ((UWORD)code == MENUNULL) {
        return 0;
    }
    item = ItemAddress(handle->window->MenuStrip, code);
    if (item == NULL) {
        return 0;
    }
    for (menu = handle->menus; menu != NULL; menu = menu->next) {
        cl_menu_item_node *node;
        for (node = menu->items; node != NULL; node = node->next) {
            if (node->item == item) {
                return node->id;
            }
        }
    }
    return 0;
}

/* Copy one IntuiMessage into the wrapper's last_event. Returns false when the
 * class is not part of the published event set (the caller replies and skips
 * it). */
static bool cl_window_fill_event(cl_window *handle, struct IntuiMessage *msg)
{
    memset(&handle->last_event, 0, sizeof handle->last_event);
    handle->last_event.code = (LONG)msg->Code;
    handle->last_event.qualifier = msg->Qualifier;
    handle->last_event.mouse_x = (LONG)handle->window->GZZMouseX;
    handle->last_event.mouse_y = (LONG)handle->window->GZZMouseY;

    switch (msg->Class) {
    case IDCMP_CLOSEWINDOW:
        handle->last_event.kind = CL_EVENT_CLOSE;
        return true;
    case IDCMP_GADGETUP:
        handle->last_event.kind = CL_EVENT_GADGET;
        handle->last_event.gadget =
            cl_window_gadget_id(handle, msg->IAddress);
        return true;
    case IDCMP_MENUPICK:
        handle->last_event.kind = CL_EVENT_MENU;
        handle->last_event.menu =
            cl_window_menu_id(handle, (ULONG)msg->Code);
        return true;
    case IDCMP_MOUSEBUTTONS:
        handle->last_event.kind = CL_EVENT_MOUSE;
        return true;
    case IDCMP_RAWKEY:
    case IDCMP_VANILLAKEY:
        handle->last_event.kind = CL_EVENT_KEY;
        return true;
    case IDCMP_ACTIVEWINDOW:
        handle->last_event.kind = CL_EVENT_ACTIVE;
        return true;
    case IDCMP_INACTIVEWINDOW:
        handle->last_event.kind = CL_EVENT_INACTIVE;
        return true;
    default:
        return false;
    }
}

/* Wait for (or poll) one event. Refresh messages are acknowledged and skipped.
 * Every other message is copied to last_event and replied before returning. */
BOOL cl_window_next_event(void *window_handle, BOOL wait)
{
    cl_window *handle = (cl_window *)window_handle;
    struct IntuiMessage *msg;

    if (handle == NULL || handle->window == NULL
        || handle->window->UserPort == NULL) {
        return false;
    }
    for (;;) {
        if (wait) {
            (void)WaitPort(handle->window->UserPort);
        }
        msg = (struct IntuiMessage *)GetMsg(handle->window->UserPort);
        if (msg == NULL) {
            return false;
        }
        if (msg->Class == IDCMP_REFRESHWINDOW) {
            BeginRefresh(handle->window);
            EndRefresh(handle->window, TRUE);
            ReplyMsg((struct Message *)msg);
            continue;
        }
        if (!cl_window_fill_event(handle, msg)) {
            ReplyMsg((struct Message *)msg);
            continue;
        }
        ReplyMsg((struct Message *)msg);
        return true;
    }
}

/* Event field accessors. Null-safe; an invalid window reads as zero. */
LONG cl_window_event_kind(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;
    return handle == NULL ? 0 : handle->last_event.kind;
}

LONG cl_window_event_code(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;
    return handle == NULL ? 0 : handle->last_event.code;
}

UWORD cl_window_event_qualifier(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;
    return handle == NULL ? 0 : handle->last_event.qualifier;
}

LONG cl_window_event_mouse_x(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;
    return handle == NULL ? 0 : handle->last_event.mouse_x;
}

LONG cl_window_event_mouse_y(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;
    return handle == NULL ? 0 : handle->last_event.mouse_y;
}

LONG cl_window_event_gadget(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;
    return handle == NULL ? 0 : handle->last_event.gadget;
}

LONG cl_window_event_menu(void *window_handle)
{
    const cl_window *handle = (const cl_window *)window_handle;
    return handle == NULL ? 0 : handle->last_event.menu;
}

/* One message with a small integer payload the acceptance programs can read.
 * The embedded `struct Message` must stay first so exec's list nodes and the
 * reply path see an ordinary message. */
typedef struct cl_message {
    struct Message msg;
    LONG payload;
} cl_message;

/* Create a private message port, or NULL when the system is out of signals or
 * memory. */
void *cl_create_msg_port(void)
{
    /* CreatePort is the 1.3-compatible private-port constructor. Passing a
     * null name keeps the port private, matching CreateMsgPort's semantics. */
    return (void *)CreatePort(NULL, 0);
}

/* Create a public named message port, or NULL for a null/empty name or an
 * allocation failure. CreatePort does not copy the name: the caller keeps the
 * string alive until the port is deleted. */
void *cl_create_public_port(char *name)
{
    if (name == NULL || name[0] == '\0') {
        return NULL;
    }
    return (void *)CreatePort((STRPTR)name, 0);
}

/* Delete a private message port. Null-safe; the caller has already drained it. */
void cl_delete_msg_port(void *port_handle)
{
    struct MsgPort *port = (struct MsgPort *)port_handle;
    if (port != NULL) {
        DeletePort(port);
    }
}

/* Allocate a message whose reply port is `reply_handle`, with `payload` carried
 * alongside the exec message header. Returns NULL on allocation failure. */
void *cl_create_message(void *reply_handle, LONG payload)
{
    cl_message *message =
        (cl_message *)AllocMem(sizeof *message, MEMF_PUBLIC | MEMF_CLEAR);
    if (message == NULL) {
        return NULL;
    }
    message->msg.mn_Node.ln_Type = NT_MESSAGE;
    message->msg.mn_Length = sizeof *message;
    message->msg.mn_ReplyPort = (struct MsgPort *)reply_handle;
    message->payload = payload;
    return message;
}

/* Read the payload of a message created by `cl_create_message`. */
LONG cl_message_payload(void *message_handle)
{
    return ((const cl_message *)message_handle)->payload;
}

/* Queue `message_handle` on `port_handle` and wake the port owner. */
void cl_put_msg(void *port_handle, void *message_handle)
{
    PutMsg((struct MsgPort *)port_handle, (struct Message *)message_handle);
}

/* The signal mask that wakes the owner of `port_handle`. */
ULONG cl_signals(void *port_handle)
{
    const struct MsgPort *port = (const struct MsgPort *)port_handle;
    return 1UL << port->mp_SigBit;
}

/* Wait for any signal in `mask`, returning the bits that arrived. */
ULONG cl_wait_signals(ULONG mask)
{
    return Wait(mask);
}

/* Take the next message from `port_handle`, or NULL when the queue is empty. */
void *cl_get_msg(void *port_handle)
{
    return (void *)GetMsg((struct MsgPort *)port_handle);
}

/* Reply a fetched message to its reply port. A message without a reply port is
 * left untouched; the wrapper's contract requires one. */
void cl_reply_msg(void *message_handle)
{
    struct Message *message = (struct Message *)message_handle;
    if (message != NULL && message->mn_ReplyPort != NULL) {
        ReplyMsg(message);
    }
}

/* Release a message that has returned through its reply port. Do not call this
 * on a message still owned by another task. */
void cl_free_message(void *message_handle)
{
    if (message_handle != NULL) {
        FreeMem(message_handle, sizeof(cl_message));
    }
}

/* Release a fetched message that has returned through its reply port. The
 * typed wrapper uses this only for a message this program created. */
void cl_release_message(void *message_handle)
{
    cl_free_message(message_handle);
}

/* ------------------------------------------------------------------------- */
/* Exec worker tasks.                                                        */
/*                                                                           */
/* A task is a plain Exec task at priority -1, not a DOS process. The worker  */
/* runs the capture-less callback, signals the parent, and parks in Wait(0). */
/* The parent removes it with Forbid/RemTask/Permit (the pre-V36 workaround)  */
/* before freeing the record and the join signal.                            */
/* ------------------------------------------------------------------------- */

#define CL_TASK_MAGIC 0x434C544BUL /* "CLTK" */
#define CL_TASK_STACK_MIN 4096UL
#define CL_TASK_STACK_MAX 65536UL

typedef struct cl_task {
    ULONG magic;
    struct Task *task;
    struct Task *parent;
    cl_handler_fn entry;
    BYTE signal_bit;
    int joined;
    LONG result;
} cl_task;

/* The C entry point Exec calls. The record is in tc_UserData; the worker
 * signals the parent after the Clorraine callback returns, then waits for the
 * parent to remove it. */
static void cl_task_entry(void)
{
    struct Task *self = FindTask(NULL);
    cl_task *record = (cl_task *)self->tc_UserData;

    if (record != NULL && record->magic == CL_TASK_MAGIC &&
        record->entry != NULL) {
        record->entry();
    }
    if (record != NULL && record->magic == CL_TASK_MAGIC) {
        Signal(record->parent, 1UL << record->signal_bit);
    }
    Wait(0);
}

/* Wait for the worker, remove it, and free the signal and record. The
 * Forbid/Permit pair is required on Kickstart 1.3 because removing another
 * task created by CreateTask has a pre-V36 crash risk without it. */
static LONG cl_task_join_record(cl_task *record)
{
    LONG result;

    if (record == NULL || record->magic != CL_TASK_MAGIC) {
        return 0;
    }
    if (!record->joined) {
        Wait(1UL << record->signal_bit);
        record->joined = 1;
        Forbid();
        RemTask(record->task);
        Permit();
        FreeSignal(record->signal_bit);
    }
    result = record->result;
    record->magic = 0;
    FreeMem(record, (ULONG)sizeof *record);
    return result;
}

/* Start a worker task. Returns NULL on a null callback, an out-of-range stack,
 * or an allocation failure; every partial allocation is released. */
void *cl_create_task(cl_handler_fn entry, ULONG stack_size)
{
    cl_task *record;
    BYTE signal_bit;
    struct Task *parent;
    struct Task *task;

    if (entry == NULL || stack_size < CL_TASK_STACK_MIN ||
        stack_size > CL_TASK_STACK_MAX) {
        return NULL;
    }
    parent = FindTask(NULL);
    /* The worker starts at priority -1. A creator at a lower priority could
     * run the worker before the record is published, so reject it. */
    if (parent == NULL || parent->tc_Node.ln_Pri < 0) {
        return NULL;
    }
    record = (cl_task *)AllocMem((ULONG)sizeof *record,
                                 MEMF_PUBLIC | MEMF_CLEAR);
    if (record == NULL) {
        return NULL;
    }
    signal_bit = AllocSignal(-1);
    if (signal_bit < 0) {
        FreeMem(record, (ULONG)sizeof *record);
        return NULL;
    }
    record->magic = CL_TASK_MAGIC;
    record->parent = parent;
    record->entry = entry;
    record->signal_bit = signal_bit;
    task = CreateTask((STRPTR)"clorraine.worker", -1,
                      (APTR)cl_task_entry, stack_size);
    if (task == NULL) {
        FreeSignal(signal_bit);
        FreeMem(record, (ULONG)sizeof *record);
        return NULL;
    }
    record->task = task;
    task->tc_UserData = record;
    return record;
}

/* Store the current worker's result. A task not created by this module has a
 * different (or null) tc_UserData, so the magic check makes this a no-op. */
void cl_task_result(LONG value)
{
    struct Task *self = FindTask(NULL);
    cl_task *record = (cl_task *)self->tc_UserData;

    if (record != NULL && record->magic == CL_TASK_MAGIC) {
        record->result = value;
    }
}

/* Join a worker and return its result. The handle is consumed. */
LONG cl_join_task(void *handle)
{
    return cl_task_join_record((cl_task *)handle);
}

/* The generated drop for an unjoined task. */
void cl_task_free(void *handle)
{
    (void)cl_task_join_record((cl_task *)handle);
}

/* ------------------------------------------------------------------------- */
/* Exec libraries and generic devices.                                       */
/*                                                                           */
/* A device handle owns one IOStdReq and one private reply port. The reply   */
/* port is required for DoIO/SendIO/WaitIO; the request must have it before  */
/* OpenDevice. The console helper fills io_Data/io_Length from the caller's  */
/* Intuition window because console.device is bound to a window.             */
/* ------------------------------------------------------------------------- */

/* Open a library at or above `version`; a null name is rejected. The
 * `Library` handle maps to `struct Library *` in the FFI contract table, so
 * these signatures use that type rather than `void *`. */
struct Library *cl_open_library(char *name, UWORD version)
{
    if (name == NULL) {
        return NULL;
    }
    return OpenLibrary((STRPTR)name, version);
}

/* Close a library exactly once; null-safe. */
void cl_close_library(struct Library *library)
{
    if (library != NULL) {
        CloseLibrary(library);
    }
}

/* The library's major version, or 0 for a null handle. */
UWORD cl_library_version(struct Library *library)
{
    return library == NULL ? 0 : library->lib_Version;
}

/* The library's minor revision, or 0 for a null handle. */
UWORD cl_library_revision(struct Library *library)
{
    return library == NULL ? 0 : library->lib_Revision;
}

typedef struct cl_device {
    struct IOStdReq request;
    struct MsgPort *port;
    APTR buffer;             /* device-owned asynchronous buffer */
    ULONG buffer_length;
    cl_window *window_ref;   /* console window reference, or NULL */
    int pending;
} cl_device;

/* Devices that need a request larger than IOStdReq. They keep their typed
 * modules; the generic handle must not open them with a short request. */
static int cl_device_is_extended(const char *name)
{
    static const char *const names[] = {
        "audio.device",
        "timer.device",
        "serial.device",
        "parallel.device",
    };
    unsigned int index;

    for (index = 0; index < sizeof names / sizeof names[0]; index++) {
        if (strcmp(name, names[index]) == 0) {
            return 1;
        }
    }
    return 0;
}

/* Allocate a cleared device handle and its reply port. Every failure releases
 * the partial allocation. */
static cl_device *cl_device_alloc(void)
{
    cl_device *device =
        (cl_device *)AllocMem((ULONG)sizeof *device,
                              MEMF_PUBLIC | MEMF_CLEAR);

    if (device == NULL) {
        return NULL;
    }
    device->port = CreatePort(NULL, 0);
    if (device->port == NULL) {
        FreeMem(device, (ULONG)sizeof *device);
        return NULL;
    }
    device->request.io_Message.mn_ReplyPort = device->port;
    return device;
}

/* Release the device-owned asynchronous buffer, if any. */
static void cl_device_free_buffer(cl_device *device)
{
    if (device != NULL && device->buffer != NULL) {
        FreeMem(device->buffer, device->buffer_length);
        device->buffer = NULL;
        device->buffer_length = 0;
    }
}

/* Release a handle whose OpenDevice failed. The device is not open, so this
 * must not call CloseDevice. */
static void cl_device_discard(cl_device *device)
{
    cl_window *window;

    if (device == NULL) {
        return;
    }
    if (device->pending) {
        (void)AbortIO((struct IORequest *)&device->request);
        (void)WaitIO((struct IORequest *)&device->request);
        device->pending = 0;
    }
    window = device->window_ref;
    cl_device_free_buffer(device);
    if (device->port != NULL) {
        DeletePort(device->port);
    }
    FreeMem(device, (ULONG)sizeof *device);
    cl_window_release(window);
}

/* Abort a pending request and close the device. */
static void cl_device_close(cl_device *device)
{
    cl_window *window;

    if (device == NULL) {
        return;
    }
    if (device->pending) {
        (void)AbortIO((struct IORequest *)&device->request);
        (void)WaitIO((struct IORequest *)&device->request);
        device->pending = 0;
    }
    if (device->request.io_Device != NULL) {
        CloseDevice((struct IORequest *)&device->request);
    }
    window = device->window_ref;
    cl_device_free_buffer(device);
    if (device->port != NULL) {
        DeletePort(device->port);
    }
    FreeMem(device, (ULONG)sizeof *device);
    cl_window_release(window);
}

/* Open a generic device with one IOStdReq. Extended-request devices are
 * rejected because their typed modules own the larger request. */
void *cl_open_device(char *name, LONG unit, ULONG flags)
{
    cl_device *device;

    if (name == NULL || unit < 0 || unit > 255 ||
        cl_device_is_extended(name)) {
        return NULL;
    }
    device = cl_device_alloc();
    if (device == NULL) {
        return NULL;
    }
    if (OpenDevice((STRPTR)name, (ULONG)unit,
                   (struct IORequest *)&device->request, flags) != 0) {
        cl_device_discard(device);
        return NULL;
    }
    return device;
}

/* Open console.device on an Intuition window. The window is refcounted: closing
 * the Window handle while the console is open defers the actual close until the
 * device closes. */
void *cl_open_console(void *window_handle, UBYTE unit)
{
    cl_window *window = (cl_window *)window_handle;
    cl_device *device;

    if (window == NULL || window->window == NULL) {
        return NULL;
    }
    device = cl_device_alloc();
    if (device == NULL) {
        return NULL;
    }
    device->window_ref = window;
    window->console_refs++;
    device->request.io_Data = (APTR)window->window;
    device->request.io_Length = (ULONG)sizeof(struct Window);
    if (OpenDevice((STRPTR)"console.device", (ULONG)unit,
                   (struct IORequest *)&device->request, 0) != 0) {
        cl_device_discard(device);
        return NULL;
    }
    return device;
}

/* Close a device; null-safe. */
void cl_close_device(void *handle)
{
    cl_device_close((cl_device *)handle);
}

/* Set the request command; idle-only and null-safe. */
BOOL cl_device_command(void *handle, UWORD command)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL || device->pending) {
        return FALSE;
    }
    device->request.io_Command = command;
    return TRUE;
}

/* Reserve a device-owned asynchronous buffer. Length 0 releases it. */
BOOL cl_device_set_buffer(void *handle, ULONG length)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL || device->pending) {
        return FALSE;
    }
    cl_device_free_buffer(device);
    if (length == 0) {
        device->request.io_Data = NULL;
        device->request.io_Length = 0;
        return TRUE;
    }
    device->buffer = AllocMem(length, MEMF_PUBLIC | MEMF_CLEAR);
    if (device->buffer == NULL) {
        return FALSE;
    }
    device->buffer_length = length;
    device->request.io_Data = device->buffer;
    device->request.io_Length = length;
    return TRUE;
}

/* Copy data into the device-owned asynchronous buffer. */
BOOL cl_device_write_buffer(void *handle, APTR data, ULONG length)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL || device->pending || device->buffer == NULL) {
        return FALSE;
    }
    if (data == NULL && length != 0) {
        return FALSE;
    }
    if (length > device->buffer_length) {
        return FALSE;
    }
    if (length != 0) {
        memcpy(device->buffer, data, length);
    }
    device->request.io_Data = device->buffer;
    device->request.io_Length = device->buffer_length;
    return TRUE;
}

/* Set the request offset; idle-only and null-safe. */
BOOL cl_device_set_offset(void *handle, ULONG offset)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL || device->pending) {
        return FALSE;
    }
    device->request.io_Offset = offset;
    return TRUE;
}

/* Run a synchronous request over the caller's buffer. The buffer stays alive
 * for the call, so no pointer outlives the caller's scope. */
LONG cl_device_do_io(void *handle, APTR data, ULONG length)
{
    cl_device *device = (cl_device *)handle;
    LONG error;

    if (device == NULL || device->pending) {
        return -1;
    }
    if (data == NULL && length != 0) {
        return -1;
    }
    device->request.io_Data = data;
    device->request.io_Length = length;
    error = (LONG)DoIO((struct IORequest *)&device->request);
    if (device->buffer != NULL) {
        device->request.io_Data = device->buffer;
        device->request.io_Length = device->buffer_length;
    }
    return error;
}

/* Start an asynchronous request over the device-owned buffer. */
BOOL cl_device_send_io(void *handle)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL || device->pending) {
        return FALSE;
    }
    device->request.io_Data = device->buffer;
    device->request.io_Length = device->buffer_length;
    SendIO((struct IORequest *)&device->request);
    device->pending = 1;
    return TRUE;
}

/* Copy up to `length` bytes of the last result from the device buffer. */
static void cl_device_copy_out(cl_device *device, APTR data, ULONG length)
{
    ULONG actual;

    if (device == NULL || device->buffer == NULL ||
        data == NULL || length == 0) {
        return;
    }
    actual = device->request.io_Actual;
    if (actual > device->buffer_length) {
        actual = device->buffer_length;
    }
    if (actual > length) {
        actual = length;
    }
    if (actual != 0) {
        memcpy(data, device->buffer, actual);
    }
}

/* Wait for a pending request and return io_Error. An idle handle returns the
 * request's last error without copying. */
LONG cl_device_wait_io(void *handle, APTR data, ULONG length)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL) {
        return -1;
    }
    if (device->pending) {
        LONG error = (LONG)WaitIO((struct IORequest *)&device->request);
        device->pending = 0;
        cl_device_copy_out(device, data, length);
        return error;
    }
    return (LONG)device->request.io_Error;
}

/* Report whether a pending request completed. A completed request is reaped
 * with WaitIO, its result is copied out, and the pending state is cleared. */
BOOL cl_device_check_io(void *handle, APTR data, ULONG length)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL || !device->pending) {
        return FALSE;
    }
    if (CheckIO((struct IORequest *)&device->request) != NULL) {
        (void)WaitIO((struct IORequest *)&device->request);
        device->pending = 0;
        cl_device_copy_out(device, data, length);
        return TRUE;
    }
    return FALSE;
}

/* Request an abort; the caller reaps it with WaitIO or closeDevice. */
void cl_device_abort_io(void *handle)
{
    cl_device *device = (cl_device *)handle;

    if (device == NULL || !device->pending) {
        return;
    }
    (void)AbortIO((struct IORequest *)&device->request);
}

/* The last transfer length. */
ULONG cl_device_actual(void *handle)
{
    const cl_device *device = (const cl_device *)handle;

    return device == NULL ? 0 : device->request.io_Actual;
}

/* The last request error, or -1 for a null handle. */
LONG cl_device_error(void *handle)
{
    const cl_device *device = (const cl_device *)handle;

    return device == NULL ? -1 : (LONG)device->request.io_Error;
}

/* ------------------------------------------------------------------------- */
/* Native video: chip memory, bitmaps, blitter, copper, and custom displays.  */
/*                                                                           */
/* The public handles are opaque pointers to these private records. The      */
/* application never sees a `BitMap`, `ViewPort`, `UCopList`, or `ColorMap`.  */
/* A `Display` borrows two caller-owned bitmaps; it never frees them.         */
/* ------------------------------------------------------------------------- */

/* The custom-chip register block. Copper moves take the address of the       */
/* destination register, exactly like the `CMOVE(c, custom.color[n], value)`  */
/* macro does.                                                                */
#define CL_VIDEO_CUSTOM_BASE 0x00DFF000UL
#define CL_VIDEO_COLOR_BASE 0x00000180UL
#define CL_VIDEO_COPPER_MAX 256

/* One chip-memory allocation and the size needed by FreeMem. */
typedef struct cl_video_chip_buffer {
    void *memory;
    ULONG size;
} cl_video_chip_buffer;

/* One displayable bitmap. The `BitMap` is the first member so a handle can   */
/* be used directly where `struct BitMap *` is expected.                     */
typedef struct cl_video_bitmap {
    struct BitMap bitmap;
    LONG width;
    LONG height;
} cl_video_bitmap;

/* One copper builder with one instruction stream per display view. */
typedef struct cl_video_copper {
    struct UCopList *lists[2];
    int count;
    int finished;
} cl_video_copper;

/* One double-buffered custom display. */
typedef struct cl_video_display {
    struct View views[2];
    struct ViewPort view_ports[2];
    struct RasInfo ras_infos[2];
    struct BitMap *bitmaps[2];
    struct ColorMap *color_map;
    struct View *old_view;
    struct UCopList *user_copper[2];
    int width;
    int height;
    int depth;
    int front;
    int built;
    int failed;
} cl_video_display;

/* Open graphics.library once; the screen helpers share the same base. */
static int cl_video_ensure_graphics(void)
{
    if (GfxBase == NULL) {
        GfxBase = (struct GfxBase *)OpenLibrary((STRPTR) "graphics.library", 33);
    }
    return GfxBase != NULL;
}

/* Allocate a chip buffer, or NULL. The record itself lives in fast memory. */
void *cl_video_chip_alloc(ULONG size)
{
    cl_video_chip_buffer *buffer;

    if (size == 0) {
        return NULL;
    }
    buffer = (cl_video_chip_buffer *)AllocMem(
        (ULONG)sizeof *buffer, MEMF_ANY | MEMF_CLEAR);
    if (buffer == NULL) {
        return NULL;
    }
    buffer->memory = AllocMem(size, MEMF_CHIP | MEMF_CLEAR);
    if (buffer->memory == NULL) {
        FreeMem(buffer, (ULONG)sizeof *buffer);
        return NULL;
    }
    buffer->size = size;
    return buffer;
}

/* Release a chip buffer. Null-safe. */
void cl_video_chip_free(void *handle)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL) {
        return;
    }
    if (buffer->memory != NULL) {
        FreeMem(buffer->memory, buffer->size);
    }
    FreeMem(buffer, (ULONG)sizeof *buffer);
}

/* Return the payload size, or zero for a null handle. */
ULONG cl_video_chip_size(void *handle)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;

    return buffer == NULL ? 0 : buffer->size;
}

/* Bounds-checked byte reads and writes. */
UBYTE cl_video_chip_read8(void *handle, ULONG offset)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;

    if (buffer == NULL || offset >= buffer->size) {
        return 0;
    }
    return ((const UBYTE *)buffer->memory)[offset];
}

void cl_video_chip_write8(void *handle, ULONG offset, UBYTE value)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL || offset >= buffer->size) {
        return;
    }
    ((UBYTE *)buffer->memory)[offset] = value;
}

/* The 16- and 32-bit helpers require alignment so the 68000 never sees an
 * address trap. `memcpy` keeps the value independent of the compiler's
 * alignment assumptions. */
UWORD cl_video_chip_read16(void *handle, ULONG offset)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;
    UWORD value = 0;

    if (buffer == NULL || (offset & 1UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 2UL) {
        return 0;
    }
    memcpy(&value, (const UBYTE *)buffer->memory + offset, sizeof value);
    return value;
}

void cl_video_chip_write16(void *handle, ULONG offset, UWORD value)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL || (offset & 1UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 2UL) {
        return;
    }
    memcpy((UBYTE *)buffer->memory + offset, &value, sizeof value);
}

ULONG cl_video_chip_read32(void *handle, ULONG offset)
{
    const cl_video_chip_buffer *buffer =
        (const cl_video_chip_buffer *)handle;
    ULONG value = 0;

    if (buffer == NULL || (offset & 3UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 4UL) {
        return 0;
    }
    memcpy(&value, (const UBYTE *)buffer->memory + offset, sizeof value);
    return value;
}

void cl_video_chip_write32(void *handle, ULONG offset, ULONG value)
{
    cl_video_chip_buffer *buffer = (cl_video_chip_buffer *)handle;

    if (buffer == NULL || (offset & 3UL) != 0 ||
        offset > buffer->size || buffer->size - offset < 4UL) {
        return;
    }
    memcpy((UBYTE *)buffer->memory + offset, &value, sizeof value);
}

/* Create a bitmap with one AllocRaster plane per bit. */
void *cl_video_bitmap_new(LONG width, LONG height, UBYTE depth)
{
    cl_video_bitmap *bitmap;
    int plane;

    if (width < 1 || width > 1024 || height < 1 || height > 1024 ||
        depth < 1 || depth > 8) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    bitmap = (cl_video_bitmap *)AllocMem(
        (ULONG)sizeof *bitmap, MEMF_ANY | MEMF_CLEAR);
    if (bitmap == NULL) {
        return NULL;
    }
    InitBitMap(&bitmap->bitmap, (LONG)depth, width, height);
    bitmap->width = width;
    bitmap->height = height;
    for (plane = 0; plane < (int)depth; plane++) {
        bitmap->bitmap.Planes[plane] =
            AllocRaster((ULONG)width, (ULONG)height);
        if (bitmap->bitmap.Planes[plane] == NULL) {
            int free_plane;
            for (free_plane = 0; free_plane < plane; free_plane++) {
                FreeRaster(bitmap->bitmap.Planes[free_plane],
                           (ULONG)width, (ULONG)height);
            }
            FreeMem(bitmap, (ULONG)sizeof *bitmap);
            return NULL;
        }
        memset(bitmap->bitmap.Planes[plane], 0,
               (size_t)bitmap->bitmap.BytesPerRow * (size_t)bitmap->bitmap.Rows);
    }
    return bitmap;
}

/* Release every plane and the bitmap record. Null-safe. */
void cl_video_bitmap_free(void *handle)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        if (bitmap->bitmap.Planes[plane] != NULL) {
            FreeRaster(bitmap->bitmap.Planes[plane],
                       (ULONG)bitmap->width, (ULONG)bitmap->height);
        }
    }
    FreeMem(bitmap, (ULONG)sizeof *bitmap);
}

/* Geometry accessors. Null handles return zero. */
LONG cl_video_bitmap_width(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : bitmap->width;
}

LONG cl_video_bitmap_height(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : bitmap->height;
}

UBYTE cl_video_bitmap_depth(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : bitmap->bitmap.Depth;
}

ULONG cl_video_bitmap_bytes_per_row(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;

    return bitmap == NULL ? 0 : (ULONG)bitmap->bitmap.BytesPerRow;
}

/* True when (`x`,`y`) is inside the bitmap and `plane` exists. */
static int cl_video_bitmap_has_pixel(const cl_video_bitmap *bitmap,
                                     int plane, int x, int y)
{
    if (bitmap == NULL || x < 0 || y < 0) {
        return 0;
    }
    if (plane < 0 || plane >= (int)bitmap->bitmap.Depth) {
        return 0;
    }
    if (x >= (int)bitmap->width) {
        return 0;
    }
    return y < (int)bitmap->height;
}

/* Plot one pixel into one plane. */
static void cl_video_plot_plane(cl_video_bitmap *bitmap, int plane,
                                int x, int y, int on)
{
    UBYTE *row;
    UBYTE mask;

    if (!cl_video_bitmap_has_pixel(bitmap, plane, x, y)) {
        return;
    }
    row = bitmap->bitmap.Planes[plane] +
          (size_t)y * bitmap->bitmap.BytesPerRow;
    mask = (UBYTE)(0x80U >> (unsigned)(x & 7));
    if (on) {
        row[x >> 3] |= mask;
    } else {
        row[x >> 3] &= (UBYTE)~mask;
    }
}

/* Fill a horizontal span of one plane, inclusive. */
static void cl_video_fill_plane(cl_video_bitmap *bitmap, int plane,
                                int x1, int x2, int y, int on)
{
    UBYTE *row;
    int first;
    int last;
    int bit_first;
    int bit_last;
    UBYTE first_mask;
    UBYTE last_mask;
    int index;

    if (!cl_video_bitmap_has_pixel(bitmap, plane, x1, y) ||
        !cl_video_bitmap_has_pixel(bitmap, plane, x2, y)) {
        return;
    }
    row = bitmap->bitmap.Planes[plane] +
          (size_t)y * bitmap->bitmap.BytesPerRow;
    first = x1 >> 3;
    last = x2 >> 3;
    bit_first = x1 & 7;
    bit_last = x2 & 7;
    first_mask = (UBYTE)(0xFFU >> (unsigned)bit_first);
    last_mask = (UBYTE)(0xFFU << (unsigned)(7 - bit_last));
    if (first == last) {
        UBYTE mask = (UBYTE)(first_mask & last_mask);
        if (on) {
            row[first] |= mask;
        } else {
            row[first] &= (UBYTE)~mask;
        }
        return;
    }
    if (on) {
        row[first] |= first_mask;
    } else {
        row[first] &= (UBYTE)~first_mask;
    }
    for (index = first + 1; index < last; index++) {
        row[index] = on ? 0xFFU : 0x00U;
    }
    if (on) {
        row[last] |= last_mask;
    } else {
        row[last] &= (UBYTE)~last_mask;
    }
}

/* Clear the bitmap to `pen`. */
void cl_video_bitmap_clear(void *handle, UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        memset(bitmap->bitmap.Planes[plane],
               (pen & (UBYTE)(1U << (unsigned)plane)) != 0 ? 0xFF : 0x00,
               (size_t)bitmap->bitmap.BytesPerRow * (size_t)bitmap->bitmap.Rows);
    }
}

/* Plot one pixel with the full pen value. */
void cl_video_bitmap_plot(void *handle, LONG x, LONG y, UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        cl_video_plot_plane(bitmap, plane, (int)x, (int)y,
                            (pen & (UBYTE)(1U << (unsigned)plane)) != 0);
    }
}

/* Clip a line endpoint pair to the bitmap before the Bresenham loop. */
static int cl_video_clip_line(cl_video_bitmap *bitmap, int *x1, int *y1,
                              int *x2, int *y2)
{
    int width;
    int height;

    if (bitmap == NULL) {
        return 0;
    }
    width = (int)bitmap->width;
    height = (int)bitmap->height;
    if (width <= 0 || height <= 0) {
        return 0;
    }
    if (*x1 < -32768) {
        *x1 = -32768;
    }
    if (*x1 > 32767) {
        *x1 = 32767;
    }
    if (*x2 < -32768) {
        *x2 = -32768;
    }
    if (*x2 > 32767) {
        *x2 = 32767;
    }
    if (*y1 < -32768) {
        *y1 = -32768;
    }
    if (*y1 > 32767) {
        *y1 = 32767;
    }
    if (*y2 < -32768) {
        *y2 = -32768;
    }
    if (*y2 > 32767) {
        *y2 = 32767;
    }
    return 1;
}

/* Draw a line with integer Bresenham stepping. */
void cl_video_bitmap_line(void *handle, LONG x1, LONG y1, LONG x2, LONG y2,
                          UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int ix1 = (int)x1;
    int iy1 = (int)y1;
    int ix2 = (int)x2;
    int iy2 = (int)y2;
    int dx;
    int dy;
    int sx;
    int sy;
    int error;

    if (!cl_video_clip_line(bitmap, &ix1, &iy1, &ix2, &iy2)) {
        return;
    }
    dx = ix2 > ix1 ? ix2 - ix1 : ix1 - ix2;
    dy = iy2 > iy1 ? iy2 - iy1 : iy1 - iy2;
    sx = ix1 < ix2 ? 1 : -1;
    sy = iy1 < iy2 ? 1 : -1;
    error = dx - dy;
    for (;;) {
        cl_video_bitmap_plot(bitmap, ix1, iy1, pen);
        if (ix1 == ix2 && iy1 == iy2) {
            break;
        }
        if (error * 2 > -dy) {
            error -= dy;
            ix1 += sx;
        }
        if (error * 2 < dx) {
            error += dx;
            iy1 += sy;
        }
    }
}

/* Fill an inclusive rectangle, clipping to the bitmap. */
void cl_video_bitmap_fill(void *handle, LONG x1, LONG y1, LONG x2, LONG y2,
                          UBYTE pen)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    int ix1 = (int)x1;
    int iy1 = (int)y1;
    int ix2 = (int)x2;
    int iy2 = (int)y2;
    int swap;
    int width;
    int height;
    int y;
    int plane;

    if (bitmap == NULL) {
        return;
    }
    if (ix1 > ix2) {
        swap = ix1;
        ix1 = ix2;
        ix2 = swap;
    }
    if (iy1 > iy2) {
        swap = iy1;
        iy1 = iy2;
        iy2 = swap;
    }
    width = (int)bitmap->width;
    height = (int)bitmap->height;
    if (ix2 < 0 || iy2 < 0 || ix1 >= width || iy1 >= height) {
        return;
    }
    if (ix1 < 0) {
        ix1 = 0;
    }
    if (iy1 < 0) {
        iy1 = 0;
    }
    if (ix2 >= width) {
        ix2 = width - 1;
    }
    if (iy2 >= height) {
        iy2 = height - 1;
    }
    for (y = iy1; y <= iy2; y++) {
        for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
            cl_video_fill_plane(bitmap, plane, ix1, ix2, y,
                                (pen & (UBYTE)(1U << (unsigned)plane)) != 0);
        }
    }
}

/* Read one plane bit. */
BOOL cl_video_bitmap_plane_bit(void *handle, UBYTE plane, LONG x, LONG y)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;
    const UBYTE *row;
    UBYTE mask;

    if (!cl_video_bitmap_has_pixel(bitmap, (int)plane, (int)x, (int)y)) {
        return FALSE;
    }
    row = bitmap->bitmap.Planes[plane] +
          (size_t)y * bitmap->bitmap.BytesPerRow;
    mask = (UBYTE)(0x80U >> (unsigned)(x & 7));
    return (row[x >> 3] & mask) != 0 ? TRUE : FALSE;
}

/* Write one plane bit. */
void cl_video_bitmap_set_plane_bit(void *handle, UBYTE plane, LONG x, LONG y,
                                   BOOL on)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;

    cl_video_plot_plane(bitmap, (int)plane, (int)x, (int)y, on ? 1 : 0);
}

/* A rectangle is valid when it lies completely inside the bitmap. */
static int cl_video_rect_inside(const cl_video_bitmap *bitmap,
                                int x, int y, int width, int height)
{
    int limit_x;
    int limit_y;

    if (bitmap == NULL || width <= 0 || height <= 0 || x < 0 || y < 0) {
        return 0;
    }
    limit_x = (int)bitmap->width;
    limit_y = (int)bitmap->height;
    if (x > limit_x || y > limit_y) {
        return 0;
    }
    return width <= limit_x - x && height <= limit_y - y;
}

/* Plain blitter copy (minterm C0). */
void cl_video_bitmap_blit(void *dest_handle, void *src_handle,
                          LONG sx, LONG sy, LONG dx, LONG dy,
                          LONG width, LONG height)
{
    cl_video_bitmap *dest = (cl_video_bitmap *)dest_handle;
    const cl_video_bitmap *src = (const cl_video_bitmap *)src_handle;

    if (width > 992 || height > 1024) {
        return;
    }
    if (!cl_video_rect_inside(src, (int)sx, (int)sy, (int)width, (int)height) ||
        !cl_video_rect_inside(dest, (int)dx, (int)dy, (int)width, (int)height)) {
        return;
    }
    (void)BltBitMap(&src->bitmap, sx, sy, &dest->bitmap, dx, dy,
                    width, height, 0xC0, 0xFF, NULL);
    WaitBlit();
}

/* Masked blitter copy (minterm E0, single-plane mask). */
void cl_video_bitmap_blit_masked(void *dest_handle, void *src_handle,
                                 void *mask_handle,
                                 LONG sx, LONG sy, LONG dx, LONG dy,
                                 LONG width, LONG height)
{
    cl_video_bitmap *dest = (cl_video_bitmap *)dest_handle;
    const cl_video_bitmap *src = (const cl_video_bitmap *)src_handle;
    const cl_video_bitmap *mask = (const cl_video_bitmap *)mask_handle;
    struct RastPort rast_port;

    if (dest == NULL || src == NULL || mask == NULL ||
        mask->bitmap.Depth != 1 ||
        mask->width != src->width || mask->height != src->height ||
        mask->bitmap.BytesPerRow != src->bitmap.BytesPerRow) {
        return;
    }
    if (width > 992 || height > 1024) {
        return;
    }
    if (!cl_video_rect_inside(src, (int)sx, (int)sy, (int)width, (int)height) ||
        !cl_video_rect_inside(dest, (int)dx, (int)dy, (int)width, (int)height)) {
        return;
    }
    InitRastPort(&rast_port);
    rast_port.BitMap = &dest->bitmap;
    rast_port.Mask = 0xFF;
    BltMaskBitMapRastPort(&src->bitmap, sx, sy, &rast_port, dx, dy,
                          width, height, 0xE0, mask->bitmap.Planes[0]);
    WaitBlit();
}

/* FNV-1a 32-bit over the geometry and every plane byte. */
ULONG cl_video_bitmap_checksum(void *handle)
{
    const cl_video_bitmap *bitmap = (const cl_video_bitmap *)handle;
    uint32_t hash = UINT32_C(2166136261);
    int plane;

    if (bitmap == NULL) {
        return 0;
    }
    hash = (hash ^ (uint32_t)bitmap->bitmap.Depth) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->width) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->height) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->bitmap.BytesPerRow) * UINT32_C(16777619);
    hash = (hash ^ (uint32_t)bitmap->bitmap.Rows) * UINT32_C(16777619);
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        const uint8_t *bytes = (const uint8_t *)bitmap->bitmap.Planes[plane];
        size_t length =
            (size_t)bitmap->bitmap.BytesPerRow * (size_t)bitmap->bitmap.Rows;
        size_t index;
        for (index = 0; index < length; index++) {
            hash = (hash ^ (uint32_t)bytes[index]) * UINT32_C(16777619);
        }
    }
    return (ULONG)hash;
}

/* Replace every pixel with one chunky palette index. The planes are cleared
 * first, so a short index view cannot leave stale pixels or read past the
 * caller's buffer. */
void cl_video_bitmap_indices(void *handle, APTR indices, ULONG count)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)handle;
    const UBYTE *pixels = (const UBYTE *)indices;
    ULONG total;
    int plane;
    int x;
    int y;

    if (bitmap == NULL) {
        return;
    }
    total = (ULONG)bitmap->width * (ULONG)bitmap->height;
    for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
        memset(bitmap->bitmap.Planes[plane], 0,
               (size_t)bitmap->bitmap.BytesPerRow *
                   (size_t)bitmap->bitmap.Rows);
    }
    if (pixels == NULL || count < total) {
        return;
    }
    for (y = 0; y < (int)bitmap->height; y++) {
        for (x = 0; x < (int)bitmap->width; x++) {
            UBYTE pen = pixels[(ULONG)y * (ULONG)bitmap->width + (ULONG)x];
            for (plane = 0; plane < (int)bitmap->bitmap.Depth; plane++) {
                if ((pen & (UBYTE)(1U << (unsigned)plane)) != 0) {
                    cl_video_plot_plane(bitmap, plane, x, y, 1);
                }
            }
        }
    }
}

/* Free one user copper list through a temporary viewport. */
static void cl_video_free_user_copper(struct UCopList *list)
{
    struct ViewPort view_port;

    if (list == NULL) {
        return;
    }
    InitVPort(&view_port);
    view_port.UCopIns = list;
    FreeVPortCopLists(&view_port);
}

/* Free the two instruction streams of a copper that never reached a display. */
static void cl_video_copper_destroy(cl_video_copper *copper)
{
    int index;

    if (copper == NULL) {
        return;
    }
    for (index = 0; index < 2; index++) {
        cl_video_free_user_copper(copper->lists[index]);
        copper->lists[index] = NULL;
    }
    FreeMem(copper, (ULONG)sizeof *copper);
}

/* Create a copper builder, or NULL. */
void *cl_video_copper_new(void)
{
    cl_video_copper *copper;
    int index;

    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    copper = (cl_video_copper *)AllocMem(
        (ULONG)sizeof *copper, MEMF_ANY | MEMF_CLEAR);
    if (copper == NULL) {
        return NULL;
    }
    for (index = 0; index < 2; index++) {
        copper->lists[index] = (struct UCopList *)AllocMem(
            (ULONG)sizeof(struct UCopList), MEMF_PUBLIC | MEMF_CLEAR);
        if (copper->lists[index] == NULL ||
            UCopperListInit(copper->lists[index], CL_VIDEO_COPPER_MAX) == NULL) {
            cl_video_copper_destroy(copper);
            return NULL;
        }
    }
    return copper;
}

/* Release a copper that was not moved into a display. */
void cl_video_copper_free(void *handle)
{
    cl_video_copper_destroy((cl_video_copper *)handle);
}

/* Append the same wait to both streams. */
void cl_video_copper_wait(void *handle, LONG line)
{
    cl_video_copper *copper = (cl_video_copper *)handle;
    int index;

    if (copper == NULL || copper->finished ||
        copper->count >= CL_VIDEO_COPPER_MAX) {
        return;
    }
    for (index = 0; index < 2; index++) {
        (void)CWait(copper->lists[index], line, 0);
        CBump(copper->lists[index]);
    }
    copper->count++;
}

/* Append the same register move to both streams. */
void cl_video_copper_move(void *handle, UWORD reg, UWORD value)
{
    cl_video_copper *copper = (cl_video_copper *)handle;
    APTR destination;
    int index;

    if (copper == NULL || copper->finished ||
        copper->count >= CL_VIDEO_COPPER_MAX) {
        return;
    }
    destination = (APTR)(CL_VIDEO_CUSTOM_BASE + (ULONG)reg);
    for (index = 0; index < 2; index++) {
        (void)CMove(copper->lists[index], destination, (LONG)value);
        CBump(copper->lists[index]);
    }
    copper->count++;
}

/* Append a 12-bit color write to both streams. */
void cl_video_copper_color(void *handle, UBYTE index, UBYTE r, UBYTE g,
                           UBYTE b)
{
    UWORD value;

    if (index > 31) {
        return;
    }
    value = (UWORD)((((UWORD)r >> 4) << 8) |
                    (((UWORD)g >> 4) << 4) |
                    ((UWORD)b >> 4));
    cl_video_copper_move(handle,
                         (UWORD)(CL_VIDEO_COLOR_BASE + (ULONG)index * 2UL),
                         value);
}

/* Append the documented wait-forever instruction. */
void cl_video_copper_end(void *handle)
{
    cl_video_copper *copper = (cl_video_copper *)handle;
    int index;

    if (copper == NULL || copper->finished) {
        return;
    }
    for (index = 0; index < 2; index++) {
        (void)CWait(copper->lists[index], 10000, 255);
        CBump(copper->lists[index]);
    }
    copper->finished = 1;
}

/* Set one palette entry from 8-bit components. */
void cl_video_display_set_color(void *handle, UBYTE index, UBYTE r, UBYTE g,
                                UBYTE b)
{
    cl_video_display *display = (cl_video_display *)handle;

    if (display == NULL || (int)index >= (1 << display->depth)) {
        return;
    }
    SetRGB4CM(display->color_map, (LONG)index,
              (ULONG)(r >> 4), (ULONG)(g >> 4), (ULONG)(b >> 4));
}

/* Default palette: black, white, RGB, CMY, then a gray ramp. */
static void cl_video_display_default_palette(cl_video_display *display)
{
    int count = 1 << display->depth;
    int index;

    for (index = 0; index < count; index++) {
        ULONG r = 0;
        ULONG g = 0;
        ULONG b = 0;
        switch (index) {
        case 0:
            break;
        case 1:
            r = 15;
            g = 15;
            b = 15;
            break;
        case 2:
            r = 15;
            break;
        case 3:
            g = 15;
            break;
        case 4:
            b = 15;
            break;
        case 5:
            r = 15;
            g = 15;
            break;
        case 6:
            g = 15;
            b = 15;
            break;
        case 7:
            r = 15;
            b = 15;
            break;
        default:
            r = (ULONG)((index - 8) * 2);
            if (r > 15) {
                r = 15;
            }
            g = r;
            b = r;
            break;
        }
        SetRGB4CM(display->color_map, (LONG)index, r, g, b);
    }
}

/* Open a display over two caller-owned bitmaps, or NULL. */
void *cl_video_display_open(void *front_handle, void *back_handle)
{
    cl_video_bitmap *front = (cl_video_bitmap *)front_handle;
    cl_video_bitmap *back = (cl_video_bitmap *)back_handle;
    cl_video_display *display;
    int index;

    if (front == NULL || back == NULL) {
        return NULL;
    }
    if (front->bitmap.Depth != back->bitmap.Depth ||
        front->width != back->width || front->height != back->height ||
        front->bitmap.BytesPerRow != back->bitmap.BytesPerRow ||
        front->width < 1 || front->width > 320 ||
        front->height < 1 || front->height > 256 ||
        front->bitmap.Depth < 1 || front->bitmap.Depth > 5) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    display = (cl_video_display *)AllocMem(
        (ULONG)sizeof *display, MEMF_ANY | MEMF_CLEAR);
    if (display == NULL) {
        return NULL;
    }
    display->bitmaps[0] = &front->bitmap;
    display->bitmaps[1] = &back->bitmap;
    display->width = (int)front->width;
    display->height = (int)front->height;
    display->depth = (int)front->bitmap.Depth;
    display->color_map = GetColorMap(1L << display->depth);
    if (display->color_map == NULL) {
        FreeMem(display, (ULONG)sizeof *display);
        return NULL;
    }
    cl_video_display_default_palette(display);
    /* Initialize both viewports once. Sprite and copper attachments land in
     * their lists after this point; the first flip only compiles them. */
    for (index = 0; index < 2; index++) {
        InitView(&display->views[index]);
        InitVPort(&display->view_ports[index]);
        display->views[index].ViewPort = &display->view_ports[index];
        display->view_ports[index].DWidth = (WORD)display->width;
        display->view_ports[index].DHeight = (WORD)display->height;
        display->view_ports[index].DxOffset = 0x81;
        display->view_ports[index].DyOffset = 0x2c;
        display->view_ports[index].Modes = 0;
        display->views[index].Modes = 0;
        display->view_ports[index].RasInfo = &display->ras_infos[index];
        display->view_ports[index].ColorMap = display->color_map;
        display->ras_infos[index].BitMap = display->bitmaps[index];
        display->ras_infos[index].RxOffset = 0;
        display->ras_infos[index].RyOffset = 0;
        display->ras_infos[index].Next = NULL;
    }
    return display;
}

/* Queue a copper for both viewports before the first flip. */
BOOL cl_video_display_set_copper(void *handle, void *copper_handle)
{
    cl_video_display *display = (cl_video_display *)handle;
    cl_video_copper *copper = (cl_video_copper *)copper_handle;
    int index;

    if (display == NULL || copper == NULL) {
        cl_video_copper_destroy(copper);
        return FALSE;
    }
    if (display->built) {
        cl_video_copper_destroy(copper);
        return FALSE;
    }
    for (index = 0; index < 2; index++) {
        cl_video_free_user_copper(display->user_copper[index]);
        display->user_copper[index] = copper->lists[index];
        copper->lists[index] = NULL;
    }
    FreeMem(copper, (ULONG)sizeof *copper);
    return TRUE;
}

/* Build the views, or switch to the other one. */
void cl_video_display_flip(void *handle)
{
    cl_video_display *display = (cl_video_display *)handle;
    int index;

    if (display == NULL || display->failed) {
        return;
    }
    if (!display->built) {
        display->old_view = GfxBase->ActiView;
        for (index = 0; index < 2; index++) {
            display->view_ports[index].UCopIns = display->user_copper[index];
            display->user_copper[index] = NULL;
            display->view_ports[index].Modes = 0;
            display->views[index].Modes = 0;
            (void)MakeVPort(&display->views[index],
                            &display->view_ports[index]);
            (void)MrgCop(&display->views[index]);
            if (display->views[index].LOFCprList == NULL) {
                display->failed = 1;
                display->built = 1;
                return;
            }
        }
        LoadView(&display->views[0]);
        WaitTOF();
        display->front = 0;
        display->built = 1;
        return;
    }
    LoadView(&display->views[1 - display->front]);
    WaitTOF();
    display->front = 1 - display->front;
}

/* True when the display could not build its views. */
BOOL cl_video_display_failed(void *handle)
{
    const cl_video_display *display = (const cl_video_display *)handle;

    return display == NULL || display->failed ? TRUE : FALSE;
}

/* Return 0 for the front bitmap, 1 for the back bitmap. */
UBYTE cl_video_display_index(void *handle)
{
    const cl_video_display *display = (const cl_video_display *)handle;

    return display == NULL ? 0 : (UBYTE)display->front;
}

/* Restore the system view and release the display. */
void cl_video_display_free(void *handle)
{
    cl_video_display *display = (cl_video_display *)handle;
    int index;

    if (display == NULL) {
        return;
    }
    if (display->built) {
        LoadView(display->old_view);
        WaitTOF();
        WaitTOF();
        for (index = 0; index < 2; index++) {
            if (display->views[index].LOFCprList != NULL) {
                FreeCprList(display->views[index].LOFCprList);
            }
        }
    }
    for (index = 0; index < 2; index++) {
        cl_video_free_user_copper(display->user_copper[index]);
        display->user_copper[index] = NULL;
        FreeVPortCopLists(&display->view_ports[index]);
    }
    if (display->color_map != NULL) {
        FreeColorMap(display->color_map);
    }
    FreeMem(display, (ULONG)sizeof *display);
}

/* Wait for the next vertical blank. */
void cl_video_wait_vblank(void)
{
    if (cl_video_ensure_graphics()) {
        WaitTOF();
    }
}

/* ------------------------------------------------------------------------- */
/* OCS hardware sprites: eight DMA channels, pairs, collision, animation.     */
/*                                                                           */
/* The graphics.library simple-sprite machine owns the sprite copper          */
/* instructions. Sprites attach to an Intuition screen's viewport; the image  */
/* is the documented posctl[2] + data[height][2] + reserved[2] block in chip  */
/* memory. The module tracks channels so a closing screen can detach them.    */
/* ------------------------------------------------------------------------- */

#define CL_SPRITE_COUNT 8
#define CL_SPRITE_ATTACHED 0x80
#define CL_SPRITE_CUSTOM ((volatile struct Custom *)0x00DFF000UL)

/* One hardware sprite channel. */
typedef struct cl_sprite {
    struct SimpleSprite simple;
    int channel;
    int height;
    int x;
    int y;
    int visible;
    int attach;
    UWORD *image;
    UWORD *blank;
    struct Screen *screen;
} cl_sprite;

/* One animation over a fixed set of frame images. */
typedef struct cl_sprite_animation {
    cl_sprite *sprite;
    int frame_count;
    int frame;
    UWORD **frames;
} cl_sprite_animation;

/* Channel ownership is process-wide because the hardware channels are. */
static cl_sprite *cl_sprite_channels[CL_SPRITE_COUNT];

/* Bytes in one sprite image: posctl[2] + data[height][2] + reserved[2]. */
static ULONG cl_sprite_image_bytes(int height)
{
    return (ULONG)(height * 2 + 4) * (ULONG)sizeof(UWORD);
}

/* Pack a 16-pixel-per-line image after the two-word posctl header. */
static void cl_sprite_pack(UWORD *image, int height, const UBYTE *pixels)
{
    int y;

    image[0] = 0;
    image[1] = 0;
    for (y = 0; y < height; y++) {
        UWORD plane_a = 0;
        UWORD plane_b = 0;
        int x;
        for (x = 0; x < 16; x++) {
            UBYTE value = (UBYTE)(pixels[y * 16 + x] & 0x03U);
            UWORD bit = (UWORD)(0x8000U >> (unsigned)x);
            if ((value & 0x01U) != 0) {
                plane_a |= bit;
            }
            if ((value & 0x02U) != 0) {
                plane_b |= bit;
            }
        }
        image[2 + y * 2] = plane_a;
        image[3 + y * 2] = plane_b;
    }
    image[2 + height * 2] = 0;
    image[3 + height * 2] = 0;
}

/* Change the sprite image and rebuild the screen's display list. */
static void cl_sprite_change_image(cl_sprite *sprite, UWORD *image)
{
    if (sprite == NULL || sprite->screen == NULL) {
        return;
    }
    ChangeSprite(&sprite->screen->ViewPort, &sprite->simple, image);
    if (sprite->attach && sprite->simple.posctldata != NULL) {
        sprite->simple.posctldata[1] |= CL_SPRITE_ATTACHED;
    }
    RethinkDisplay();
}

/* Move the live position; the 1.3 MoveSprite preserves the ATTACH bit. */
static void cl_sprite_move_apply(cl_sprite *sprite)
{
    if (sprite == NULL || sprite->screen == NULL) {
        return;
    }
    MoveSprite(&sprite->screen->ViewPort, &sprite->simple,
               (WORD)sprite->x, (WORD)sprite->y);
    if (sprite->attach && sprite->simple.posctldata != NULL) {
        sprite->simple.posctldata[1] |= CL_SPRITE_ATTACHED;
    }
}

/* Detach the channel and free the image, blank, and record. */
static void cl_sprite_destroy(cl_sprite *sprite, int free_images)
{
    if (sprite == NULL) {
        return;
    }
    if (sprite->screen != NULL) {
        sprite->visible = 0;
        cl_sprite_change_image(sprite, sprite->blank);
        if (cl_video_ensure_graphics()) {
            WaitTOF();
        }
    }
    (void)FreeSprite((WORD)sprite->simple.num);
    if (sprite->screen != NULL) {
        RethinkDisplay();
        sprite->screen = NULL;
    }
    if (free_images) {
        if (sprite->image != NULL) {
            FreeMem(sprite->image, cl_sprite_image_bytes(sprite->height));
        }
        if (sprite->blank != NULL) {
            FreeMem(sprite->blank, cl_sprite_image_bytes(sprite->height));
        }
    }
    if (sprite->channel >= 0 && sprite->channel < CL_SPRITE_COUNT) {
        cl_sprite_channels[sprite->channel] = NULL;
    }
    FreeMem(sprite, (ULONG)sizeof *sprite);
}

/* Allocate a channel and its image/blank buffers, or NULL. */
void *cl_sprite_new(UBYTE channel, UBYTE height)
{
    cl_sprite *sprite;
    UWORD *image;
    UWORD *blank;

    if (channel >= CL_SPRITE_COUNT || height < 1) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    sprite = (cl_sprite *)AllocMem(
        (ULONG)sizeof *sprite, MEMF_ANY | MEMF_CLEAR);
    if (sprite == NULL) {
        return NULL;
    }
    sprite->simple.num = (UWORD)-1;
    if (GetSprite(&sprite->simple, (LONG)channel) < 0) {
        FreeMem(sprite, (ULONG)sizeof *sprite);
        return NULL;
    }
    sprite->simple.height = (UWORD)height;
    image = (UWORD *)AllocMem(
        cl_sprite_image_bytes((int)height), MEMF_CHIP | MEMF_CLEAR);
    blank = (UWORD *)AllocMem(
        cl_sprite_image_bytes((int)height), MEMF_CHIP | MEMF_CLEAR);
    if (image == NULL || blank == NULL) {
        if (image != NULL) {
            FreeMem(image, cl_sprite_image_bytes((int)height));
        }
        if (blank != NULL) {
            FreeMem(blank, cl_sprite_image_bytes((int)height));
        }
        (void)FreeSprite((WORD)sprite->simple.num);
        FreeMem(sprite, (ULONG)sizeof *sprite);
        return NULL;
    }
    sprite->channel = (int)channel;
    sprite->height = (int)height;
    sprite->x = 0;
    sprite->y = 0;
    sprite->visible = 0;
    sprite->attach = 0;
    sprite->image = image;
    sprite->blank = blank;
    sprite->screen = NULL;
    cl_sprite_channels[channel] = sprite;
    return sprite;
}

/* Hide the channel, wait one frame, and release the images and channel. */
void cl_sprite_free(void *handle)
{
    cl_sprite_destroy((cl_sprite *)handle, 1);
}

/* Attach a sprite to an Intuition screen's viewport. */
BOOL cl_sprite_attach_screen(void *sprite_handle, void *screen_handle)
{
    cl_sprite *sprite = (cl_sprite *)sprite_handle;
    struct Screen *screen = (struct Screen *)screen_handle;

    if (sprite == NULL || screen == NULL || sprite->screen != NULL) {
        return FALSE;
    }
    sprite->screen = screen;
    screen->ViewPort.Modes |= SPRITES;
    ChangeSprite(&screen->ViewPort, &sprite->simple,
                 sprite->visible ? sprite->image : sprite->blank);
    if (sprite->attach && sprite->simple.posctldata != NULL) {
        sprite->simple.posctldata[1] |= CL_SPRITE_ATTACHED;
    }
    MoveSprite(&screen->ViewPort, &sprite->simple,
               (WORD)sprite->x, (WORD)sprite->y);
    RethinkDisplay();
    return TRUE;
}

/* Detach a sprite from its screen before the screen is released. */
void cl_sprite_detach(void *handle)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite != NULL) {
        sprite->screen = NULL;
    }
}

/* Detach every channel that belongs to the closing screen. */
static void cl_sprite_screen_closing(struct Screen *screen)
{
    int index;

    for (index = 0; index < CL_SPRITE_COUNT; index++) {
        cl_sprite *sprite = cl_sprite_channels[index];
        if (sprite != NULL && sprite->screen == screen) {
            sprite->screen = NULL;
        }
    }
}

/* Pack `16 * height` pixels into the image; a wrong count is ignored. */
void cl_sprite_set_pixels(void *handle, APTR pixels, ULONG count)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL || pixels == NULL ||
        count != (ULONG)(sprite->height * 16)) {
        return;
    }
    cl_sprite_pack(sprite->image, sprite->height, (const UBYTE *)pixels);
    if (sprite->visible) {
        cl_sprite_change_image(sprite, sprite->image);
    }
}

/* Move the channel; the caller keeps the sprite inside the visible screen. */
void cl_sprite_move(void *handle, LONG x, LONG y)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL || x < 0 || x > 511 || y < 0 || y > 255) {
        return;
    }
    sprite->x = (int)x;
    sprite->y = (int)y;
    cl_sprite_move_apply(sprite);
}

/* Show the channel. */
void cl_sprite_show(void *handle)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL) {
        return;
    }
    sprite->visible = 1;
    cl_sprite_change_image(sprite, sprite->image);
}

/* Hide the channel with the zero image. */
void cl_sprite_hide(void *handle)
{
    cl_sprite *sprite = (cl_sprite *)handle;

    if (sprite == NULL) {
        return;
    }
    sprite->visible = 0;
    cl_sprite_change_image(sprite, sprite->blank);
}

/* Attach `odd` to `even`, making the 16-color pair. */
BOOL cl_sprite_attach(void *even_handle, void *odd_handle)
{
    cl_sprite *even = (cl_sprite *)even_handle;
    cl_sprite *odd = (cl_sprite *)odd_handle;

    if (even == NULL || odd == NULL || even->height != odd->height ||
        even->screen != odd->screen ||
        (even->channel & 1) != 0 || odd->channel != even->channel + 1) {
        return FALSE;
    }
    odd->attach = 1;
    if (odd->screen != NULL) {
        cl_sprite_change_image(odd, odd->visible ? odd->image : odd->blank);
    }
    return TRUE;
}

/* Geometry and state accessors. */
UBYTE cl_sprite_channel(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? (UBYTE)0xFF : (UBYTE)sprite->channel;
}

UBYTE cl_sprite_height(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? 0 : (UBYTE)sprite->height;
}

LONG cl_sprite_x(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? 0 : (LONG)sprite->x;
}

LONG cl_sprite_y(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite == NULL ? 0 : (LONG)sprite->y;
}

BOOL cl_sprite_visible(void *handle)
{
    const cl_sprite *sprite = (const cl_sprite *)handle;

    return sprite != NULL && sprite->visible ? TRUE : FALSE;
}

/* Enable the four sprite-pair collision groups. */
void cl_sprite_enable_collisions(void)
{
    CL_SPRITE_CUSTOM->clxcon = 0xF000;
}

/* Read and clear the collision data register. */
ULONG cl_sprite_collisions(void)
{
    return (ULONG)CL_SPRITE_CUSTOM->clxdat;
}

/* Create an animation over a fresh sprite channel and `frames` images. */
void *cl_sprite_animation_new(UBYTE channel, UBYTE height, UBYTE frames)
{
    cl_sprite_animation *animation;
    cl_sprite *sprite;
    int index;

    if (frames < 1 || frames > 8) {
        return NULL;
    }
    sprite = (cl_sprite *)cl_sprite_new(channel, height);
    if (sprite == NULL) {
        return NULL;
    }
    animation = (cl_sprite_animation *)AllocMem(
        (ULONG)sizeof *animation, MEMF_ANY | MEMF_CLEAR);
    if (animation == NULL) {
        cl_sprite_destroy(sprite, 1);
        return NULL;
    }
    animation->frames = (UWORD **)AllocMem(
        (ULONG)frames * (ULONG)sizeof(UWORD *), MEMF_ANY | MEMF_CLEAR);
    if (animation->frames == NULL) {
        FreeMem(animation, (ULONG)sizeof *animation);
        cl_sprite_destroy(sprite, 1);
        return NULL;
    }
    animation->sprite = sprite;
    animation->frame_count = (int)frames;
    animation->frame = 0;
    animation->frames[0] = sprite->image;
    for (index = 1; index < (int)frames; index++) {
        animation->frames[index] = (UWORD *)AllocMem(
            cl_sprite_image_bytes((int)height), MEMF_CHIP | MEMF_CLEAR);
        if (animation->frames[index] == NULL) {
            int free_index;
            for (free_index = 0; free_index < index; free_index++) {
                FreeMem(animation->frames[free_index],
                        cl_sprite_image_bytes((int)height));
            }
            FreeMem(animation->frames, (ULONG)frames * (ULONG)sizeof(UWORD *));
            FreeMem(animation, (ULONG)sizeof *animation);
            sprite->image = NULL;
            cl_sprite_destroy(sprite, 1);
            return NULL;
        }
    }
    return animation;
}

/* Release every frame, the frame table, the sprite, and the animation. */
void cl_sprite_animation_free(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;
    cl_sprite *sprite;
    int height;
    int index;

    if (animation == NULL) {
        return;
    }
    sprite = animation->sprite;
    if (sprite != NULL) {
        height = sprite->height;
        /* The sprite record owns the channel and the blank image; the
         * animation owns frame 0 and the other frames. */
        sprite->image = NULL;
        cl_sprite_destroy(sprite, 1);
        for (index = 0; index < animation->frame_count; index++) {
            if (animation->frames[index] != NULL) {
                FreeMem(animation->frames[index],
                        cl_sprite_image_bytes(height));
            }
        }
    }
    if (animation->frames != NULL) {
        FreeMem(animation->frames,
                (ULONG)animation->frame_count * (ULONG)sizeof(UWORD *));
    }
    FreeMem(animation, (ULONG)sizeof *animation);
}

/* Pack one animation frame. */
void cl_sprite_animation_set_frame(void *handle, UBYTE frame, APTR pixels,
                                   ULONG count)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL || pixels == NULL ||
        frame >= (UBYTE)animation->frame_count ||
        count != (ULONG)(animation->sprite->height * 16)) {
        return;
    }
    cl_sprite_pack(animation->frames[frame], animation->sprite->height,
                   (const UBYTE *)pixels);
    if (animation->frame == (int)frame && animation->sprite->visible) {
        cl_sprite_change_image(animation->sprite, animation->frames[frame]);
    }
}

/* Show one animation frame. */
void cl_sprite_animation_show_frame(void *handle, UBYTE frame)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL || frame >= (UBYTE)animation->frame_count) {
        return;
    }
    animation->frame = (int)frame;
    animation->sprite->image = animation->frames[frame];
    animation->sprite->visible = 1;
    cl_sprite_change_image(animation->sprite, animation->frames[frame]);
}

/* Advance to the next frame and show it. */
void cl_sprite_animation_next(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL) {
        return;
    }
    animation->frame = (animation->frame + 1) % animation->frame_count;
    cl_sprite_animation_show_frame(handle, (UBYTE)animation->frame);
}

/* Move, show, or hide the animation's sprite. */
void cl_sprite_animation_move(void *handle, LONG x, LONG y)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_move(animation->sprite, x, y);
    }
}

void cl_sprite_animation_show(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_show(animation->sprite);
    }
}

void cl_sprite_animation_hide(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_hide(animation->sprite);
    }
}

/* Attach the animation's sprite to an Intuition screen. */
BOOL cl_sprite_animation_attach_screen(void *handle, void *screen)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation == NULL) {
        return FALSE;
    }
    return cl_sprite_attach_screen(animation->sprite, screen);
}

/* Detach the animation's sprite from its screen. */
void cl_sprite_animation_detach(void *handle)
{
    cl_sprite_animation *animation = (cl_sprite_animation *)handle;

    if (animation != NULL) {
        cl_sprite_detach(animation->sprite);
    }
}

/* Return the current frame index. */
UBYTE cl_sprite_animation_frame(void *handle)
{
    const cl_sprite_animation *animation =
        (const cl_sprite_animation *)handle;

    return animation == NULL ? 0 : (UBYTE)animation->frame;
}

/* ------------------------------------------------------------------------- */
/* Audio, gameport input, and timer.device.                                   */
/*                                                                           */
/* `Audio` owns one IOAudio block per allocated channel plus a control block. */
/* A `Sample` is a chip-memory copy of signed 8-bit samples. The joystick     */
/* path uses the 1.3 JOY0DAT/JOY1DAT registers and the CIAAPRA fire bits.     */
/* ------------------------------------------------------------------------- */

typedef struct cl_sample {
    UBYTE *data;
    ULONG length;
    UWORD period;
    UWORD volume;
    UWORD cycles;
} cl_sample;

typedef struct cl_audio {
    struct MsgPort *port;
    struct IOAudio *open_block;
    struct IOAudio *control;
    struct IOAudio *channels[4];
    UBYTE masks[4];
    UBYTE mask;
    WORD alloc_key;
    int write_pending[4];
} cl_audio;

typedef struct cl_timer {
    struct MsgPort *port;
    struct TimeRequest *request;
} cl_timer;

/* Forward declaration for the partial-open cleanup path. */
void cl_audio_free(void *handle);

/* Create a private reply port for a device request. */
static struct MsgPort *cl_device_port(void)
{
    return CreatePort(NULL, 0);
}

/* Allocate one IOAudio block for an already-open audio device. */
static struct IOAudio *cl_audio_block(cl_audio *audio, UBYTE unit)
{
    struct IOAudio *block;

    block = (struct IOAudio *)AllocMem(
        (ULONG)sizeof *block, MEMF_PUBLIC | MEMF_CLEAR);
    if (block == NULL) {
        return NULL;
    }
    block->ioa_Request.io_Message.mn_ReplyPort = audio->port;
    block->ioa_Request.io_Message.mn_Node.ln_Pri = 0;
    block->ioa_Request.io_Device = audio->open_block->ioa_Request.io_Device;
    block->ioa_AllocKey = audio->alloc_key;
    block->ioa_Request.io_Unit = (struct Unit *)(ULONG)unit;
    return block;
}

/* Open audio.device and allocate `channels` (1..4). */
void *cl_audio_open(UBYTE channels)
{
    cl_audio *audio;
    struct IOAudio *block;
    UBYTE masks[4];
    int count;
    int index;
    int first = -1;
    BYTE error;

    if (channels < 1 || channels > 4) {
        return NULL;
    }
    audio = (cl_audio *)AllocMem(
        (ULONG)sizeof *audio, MEMF_ANY | MEMF_CLEAR);
    if (audio == NULL) {
        return NULL;
    }
    audio->port = cl_device_port();
    if (audio->port == NULL) {
        FreeMem(audio, (ULONG)sizeof *audio);
        return NULL;
    }
    audio->open_block = (struct IOAudio *)AllocMem(
        (ULONG)sizeof(struct IOAudio), MEMF_PUBLIC | MEMF_CLEAR);
    if (audio->open_block == NULL) {
        DeletePort(audio->port);
        FreeMem(audio, (ULONG)sizeof *audio);
        return NULL;
    }
    block = audio->open_block;
    block->ioa_Request.io_Message.mn_ReplyPort = audio->port;
    block->ioa_Request.io_Message.mn_Node.ln_Pri = 0;
    block->ioa_AllocKey = 0;
    /* The mask array is stored in the audio record so the device keeps a
     * valid pointer for the allocation. */
    count = 0;
    if (channels == 1) {
        masks[0] = 0x01;
        masks[1] = 0x02;
        masks[2] = 0x04;
        masks[3] = 0x08;
        count = 4;
    } else if (channels == 2) {
        masks[0] = 0x03;
        masks[1] = 0x0C;
        count = 2;
    } else if (channels == 3) {
        masks[0] = 0x07;
        masks[1] = 0x0E;
        count = 2;
    } else {
        masks[0] = 0x0F;
        count = 1;
    }
    for (index = 0; index < count; index++) {
        audio->masks[index] = masks[index];
    }
    block->ioa_Data = audio->masks;
    block->ioa_Length = (ULONG)count;
    error = OpenDevice((STRPTR) "audio.device", 0,
                       &block->ioa_Request, 0);
    if (error != 0) {
        FreeMem(block, (ULONG)sizeof(struct IOAudio));
        DeletePort(audio->port);
        FreeMem(audio, (ULONG)sizeof *audio);
        return NULL;
    }
    audio->alloc_key = block->ioa_AllocKey;
    audio->mask = (UBYTE)((ULONG)block->ioa_Request.io_Unit & 0x0F);
    if (audio->mask == 0) {
        CloseDevice(&block->ioa_Request);
        FreeMem(block, (ULONG)sizeof(struct IOAudio));
        DeletePort(audio->port);
        FreeMem(audio, (ULONG)sizeof *audio);
        return NULL;
    }
    for (index = 0; index < 4; index++) {
        UBYTE bit = (UBYTE)(1U << (unsigned)index);
        if ((audio->mask & bit) == 0) {
            continue;
        }
        if (first < 0) {
            block->ioa_Request.io_Unit = (struct Unit *)(ULONG)bit;
            block->ioa_AllocKey = audio->alloc_key;
            audio->channels[index] = block;
            first = index;
        } else {
            audio->channels[index] = cl_audio_block(audio, bit);
            if (audio->channels[index] == NULL) {
                cl_audio_free(audio);
                return NULL;
            }
        }
    }
    audio->control = cl_audio_block(audio, 0);
    if (audio->control == NULL) {
        cl_audio_free(audio);
        return NULL;
    }
    return audio;
}

/* Abort a pending write and stop a channel. */
static void cl_audio_finish_channel(cl_audio *audio, int channel)
{
    struct IOAudio *write;
    struct IOAudio *control;

    if (audio == NULL || channel < 0 || channel >= 4) {
        return;
    }
    write = audio->channels[channel];
    control = audio->control;
    if (write == NULL || control == NULL) {
        return;
    }
    control->ioa_Request.io_Unit =
        (struct Unit *)(ULONG)(1U << (unsigned)channel);
    control->ioa_AllocKey = audio->alloc_key;
    control->ioa_Request.io_Command = ADCMD_FINISH;
    control->ioa_Request.io_Flags = 0;
    if (!audio->write_pending[channel]) {
        return;
    }
    (void)DoIO(&control->ioa_Request);
    (void)WaitIO(&write->ioa_Request);
    audio->write_pending[channel] = 0;
}

/* Stop playing channels, free the allocation, and close the device. */
void cl_audio_free(void *handle)
{
    cl_audio *audio = (cl_audio *)handle;
    int index;

    if (audio == NULL) {
        return;
    }
    if (audio->open_block != NULL &&
        audio->open_block->ioa_Request.io_Device != NULL) {
        for (index = 0; index < 4; index++) {
            if (audio->channels[index] != NULL) {
                cl_audio_finish_channel(audio, index);
            }
        }
        audio->open_block->ioa_Request.io_Unit =
            (struct Unit *)(ULONG)audio->mask;
        audio->open_block->ioa_AllocKey = audio->alloc_key;
        audio->open_block->ioa_Request.io_Command = ADCMD_FREE;
        audio->open_block->ioa_Request.io_Flags = 0;
        (void)DoIO(&audio->open_block->ioa_Request);
        CloseDevice(&audio->open_block->ioa_Request);
    }
    for (index = 0; index < 4; index++) {
        if (audio->channels[index] != NULL &&
            audio->channels[index] != audio->open_block) {
            FreeMem(audio->channels[index],
                    (ULONG)sizeof(struct IOAudio));
        }
    }
    if (audio->control != NULL) {
        FreeMem(audio->control, (ULONG)sizeof(struct IOAudio));
    }
    if (audio->open_block != NULL) {
        FreeMem(audio->open_block, (ULONG)sizeof(struct IOAudio));
    }
    if (audio->port != NULL) {
        DeletePort(audio->port);
    }
    FreeMem(audio, (ULONG)sizeof *audio);
}

/* Return the allocated channel mask. */
UBYTE cl_audio_channels(void *handle)
{
    const cl_audio *audio = (const cl_audio *)handle;

    return audio == NULL ? 0 : audio->mask;
}

/* Start a sample on one allocated channel, or return false when busy. */
BOOL cl_audio_play(void *handle, UBYTE channel, void *sample_handle)
{
    cl_audio *audio = (cl_audio *)handle;
    cl_sample *sample = (cl_sample *)sample_handle;
    struct IOAudio *block;

    if (audio == NULL || sample == NULL || channel >= 4 ||
        (audio->mask & (UBYTE)(1U << (unsigned)channel)) == 0) {
        return FALSE;
    }
    block = audio->channels[channel];
    if (block == NULL) {
        return FALSE;
    }
    if (audio->write_pending[channel]) {
        if (CheckIO(&block->ioa_Request) == NULL) {
            return FALSE;
        }
        (void)WaitIO(&block->ioa_Request);
        audio->write_pending[channel] = 0;
    }
    block->ioa_Request.io_Command = CMD_WRITE;
    block->ioa_Request.io_Flags = ADIOF_PERVOL;
    block->ioa_Data = sample->data;
    block->ioa_Length = sample->length;
    block->ioa_Period = sample->period;
    block->ioa_Volume = sample->volume;
    block->ioa_Cycles = sample->cycles;
    /* SendIO keeps ADIOF_PERVOL (0x02); DoIO clears audio flags. The
     * command does not use ADIOF_NOWAIT (0x01), so it cannot alias IOF_QUICK. */
    SendIO(&block->ioa_Request);
    audio->write_pending[channel] = 1;
    return TRUE;
}

/* Stop the sound on one channel. */
void cl_audio_stop(void *handle, UBYTE channel)
{
    cl_audio *audio = (cl_audio *)handle;

    if (audio == NULL || channel >= 4 ||
        (audio->mask & (UBYTE)(1U << (unsigned)channel)) == 0) {
        return;
    }
    cl_audio_finish_channel(audio, (int)channel);
}

/* Change the period and volume of a playing channel. */
BOOL cl_audio_control(void *handle, UBYTE channel, UWORD period, UBYTE volume)
{
    cl_audio *audio = (cl_audio *)handle;
    struct IOAudio *control;
    BYTE error;

    if (audio == NULL || channel >= 4 ||
        (audio->mask & (UBYTE)(1U << (unsigned)channel)) == 0 ||
        audio->control == NULL) {
        return FALSE;
    }
    control = audio->control;
    control->ioa_Request.io_Unit =
        (struct Unit *)(ULONG)(1U << (unsigned)channel);
    control->ioa_AllocKey = audio->alloc_key;
    control->ioa_Request.io_Command = ADCMD_PERVOL;
    control->ioa_Request.io_Flags = 0;
    control->ioa_Period = period;
    control->ioa_Volume = (UWORD)volume;
    error = DoIO(&control->ioa_Request);
    return error == 0 ? TRUE : FALSE;
}

/* Copy signed 8-bit samples into chip memory. */
void *cl_sample_new(APTR data, ULONG count, UWORD period, UBYTE volume,
                    UWORD cycles)
{
    cl_sample *sample;
    UBYTE *buffer;
    ULONG length;

    if (data == NULL || count == 0 || count > 131072UL) {
        return NULL;
    }
    length = (count + 1UL) & ~1UL;
    buffer = (UBYTE *)AllocMem(length, MEMF_CHIP | MEMF_CLEAR);
    if (buffer == NULL) {
        return NULL;
    }
    memcpy(buffer, data, (size_t)count);
    sample = (cl_sample *)AllocMem(
        (ULONG)sizeof *sample, MEMF_ANY | MEMF_CLEAR);
    if (sample == NULL) {
        FreeMem(buffer, length);
        return NULL;
    }
    sample->data = buffer;
    sample->length = length;
    sample->period = period;
    sample->volume = (UWORD)volume;
    sample->cycles = cycles;
    return sample;
}

/* Free a sample. The caller must have stopped its channel. */
void cl_sample_free(void *handle)
{
    cl_sample *sample = (cl_sample *)handle;

    if (sample == NULL) {
        return;
    }
    if (sample->data != NULL) {
        FreeMem(sample->data, sample->length);
    }
    FreeMem(sample, (ULONG)sizeof *sample);
}

ULONG cl_sample_length(void *handle)
{
    const cl_sample *sample = (const cl_sample *)handle;

    return sample == NULL ? 0 : sample->length;
}

UWORD cl_sample_period(void *handle)
{
    const cl_sample *sample = (const cl_sample *)handle;

    return sample == NULL ? 0 : sample->period;
}

UBYTE cl_sample_volume(void *handle)
{
    const cl_sample *sample = (const cl_sample *)handle;

    return sample == NULL ? 0 : (UBYTE)sample->volume;
}

UWORD cl_sample_cycles(void *handle)
{
    const cl_sample *sample = (const cl_sample *)handle;

    return sample == NULL ? 0 : sample->cycles;
}

/* Return the raw joystick word and the fire bit (bit 16), or 0. */
ULONG cl_input_state(UBYTE port)
{
    volatile struct Custom *custom = (volatile struct Custom *)0x00DFF000UL;
    volatile UBYTE *ciaapra = (volatile UBYTE *)0x00BFE001UL;
    ULONG state;

    if (port > 1) {
        return 0;
    }
    state = (ULONG)(port == 0 ? custom->joy0dat : custom->joy1dat);
    if ((*ciaapra & (port == 0 ? 0x40U : 0x80U)) == 0) {
        state |= 0x10000UL;
    }
    return state;
}

/* Open timer.device UNIT_VBLANK. */
void *cl_timer_open(void)
{
    cl_timer *timer;

    timer = (cl_timer *)AllocMem(
        (ULONG)sizeof *timer, MEMF_ANY | MEMF_CLEAR);
    if (timer == NULL) {
        return NULL;
    }
    timer->port = cl_device_port();
    if (timer->port == NULL) {
        FreeMem(timer, (ULONG)sizeof *timer);
        return NULL;
    }
    timer->request = (struct TimeRequest *)AllocMem(
        (ULONG)sizeof(struct TimeRequest), MEMF_PUBLIC | MEMF_CLEAR);
    if (timer->request == NULL) {
        DeletePort(timer->port);
        FreeMem(timer, (ULONG)sizeof *timer);
        return NULL;
    }
    timer->request->tr_node.io_Message.mn_ReplyPort = timer->port;
    if (OpenDevice((STRPTR) "timer.device", UNIT_VBLANK,
                   &timer->request->tr_node, 0) != 0) {
        FreeMem(timer->request, (ULONG)sizeof(struct TimeRequest));
        DeletePort(timer->port);
        FreeMem(timer, (ULONG)sizeof *timer);
        return NULL;
    }
    return timer;
}

/* Wait `ticks` vertical blanks. */
BOOL cl_timer_wait(void *handle, ULONG ticks)
{
    cl_timer *timer = (cl_timer *)handle;
    BYTE error;

    if (timer == NULL) {
        return FALSE;
    }
    timer->request->tr_node.io_Command = TR_ADDREQUEST;
    timer->request->tr_node.io_Flags = 0;
    /* timer.device takes seconds and microseconds even for UNIT_VBLANK;
     * convert PAL 50 Hz ticks to a relative delay. */
    timer->request->tr_time.tv_secs = ticks / 50UL;
    timer->request->tr_time.tv_micro = (ticks % 50UL) * 20000UL;
    error = DoIO(&timer->request->tr_node);
    return error == 0 ? TRUE : FALSE;
}

/* Close timer.device. */
void cl_timer_free(void *handle)
{
    cl_timer *timer = (cl_timer *)handle;

    if (timer == NULL) {
        return;
    }
    if (timer->request != NULL) {
        CloseDevice(&timer->request->tr_node);
        FreeMem(timer->request, (ULONG)sizeof(struct TimeRequest));
    }
    if (timer->port != NULL) {
        DeletePort(timer->port);
    }
    FreeMem(timer, (ULONG)sizeof *timer);
}

/* ------------------------------------------------------------------------- */
/* Text/fonts and layers.                                                     */
/*                                                                           */
/* A Font owns a TextFont from OpenFont/OpenDiskFont, or borrows             */
/* GfxBase->DefaultFont. A Layer borrows the screen's LayerInfo and the       */
/* caller's Bitmap; its RastPort provides the layers-library clip.            */
/* ------------------------------------------------------------------------- */

typedef struct cl_font {
    struct TextFont *font;
    int owns;
} cl_font;

typedef struct cl_layer {
    struct Layer *layer;
    struct Layer_Info *info;
    int bitmap_width;
    int bitmap_height;
    int width;
    int height;
} cl_layer;

/* Build the TextAttr used by both font openers. */
static void cl_font_attr(struct TextAttr *attr, char *name, UWORD size,
                         UBYTE style)
{
    attr->ta_Name = (STRPTR)name;
    attr->ta_YSize = size;
    attr->ta_Style = style;
    attr->ta_Flags = 0;
}

/* Wrap an owned TextFont, or NULL. */
static void *cl_font_wrap(struct TextFont *font, int owns)
{
    cl_font *handle;

    if (font == NULL) {
        return NULL;
    }
    handle = (cl_font *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        if (owns) {
            CloseFont(font);
        }
        return NULL;
    }
    handle->font = font;
    handle->owns = owns;
    return handle;
}

/* Open a font through graphics.library. */
void *cl_font_open(char *name, UWORD size, UBYTE style)
{
    struct TextAttr attr;
    struct TextFont *font;

    if (name == NULL || size == 0) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    cl_font_attr(&attr, name, size, style);
    font = OpenFont(&attr);
    return cl_font_wrap(font, 1);
}

/* Open a disk font through diskfont.library. */
void *cl_font_open_disk(char *name, UWORD size, UBYTE style)
{
    struct TextAttr attr;
    struct TextFont *font;

    if (name == NULL || size == 0) {
        return NULL;
    }
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    if (DiskfontBase == NULL) {
        DiskfontBase = OpenLibrary((STRPTR) "diskfont.library", 33);
    }
    if (DiskfontBase == NULL) {
        return NULL;
    }
    cl_font_attr(&attr, name, size, style);
    font = OpenDiskFont(&attr);
    return cl_font_wrap(font, 1);
}

/* Borrow the ROM default font. */
void *cl_font_default(void)
{
    if (!cl_video_ensure_graphics()) {
        return NULL;
    }
    return cl_font_wrap(GfxBase->DefaultFont, 0);
}

/* Close an owned font; the default font is left untouched. */
void cl_font_free(void *handle)
{
    cl_font *font = (cl_font *)handle;

    if (font == NULL) {
        return;
    }
    if (font->owns && font->font != NULL) {
        CloseFont(font->font);
    }
    FreeMem(font, (ULONG)sizeof *font);
}

/* Measure text with the font on a temporary RastPort. */
LONG cl_text_width(void *handle, APTR text, ULONG count)
{
    cl_font *font = (cl_font *)handle;
    struct RastPort rast_port;

    if (font == NULL || font->font == NULL || text == NULL) {
        return 0;
    }
    InitRastPort(&rast_port);
    (void)SetFont(&rast_port, font->font);
    return (LONG)TextLength(&rast_port, (STRPTR)text, count);
}

/* Return the font's Y size. */
LONG cl_text_height(void *handle)
{
    const cl_font *font = (const cl_font *)handle;

    return font == NULL || font->font == NULL ? 0 : (LONG)font->font->tf_YSize;
}

/* Draw text on an Intuition screen. */
void cl_text_draw(void *screen_handle, void *font_handle, LONG x, LONG y,
                  UBYTE pen, APTR text, ULONG count)
{
    struct Screen *screen = (struct Screen *)screen_handle;
    cl_font *font = (cl_font *)font_handle;
    struct RastPort *rast_port;
    struct TextFont *old_font;

    if (screen == NULL || font == NULL || font->font == NULL || text == NULL) {
        return;
    }
    rast_port = &screen->RastPort;
    old_font = rast_port->Font;
    (void)SetFont(rast_port, font->font);
    SetAPen(rast_port, (ULONG)pen);
    Move(rast_port, x, y);
    (void)Text(rast_port, (STRPTR)text, count);
    (void)SetFont(rast_port, old_font);
}

/* Draw text on an Intuition window's inner RastPort. */
void cl_window_text_draw(void *window_handle, void *font_handle, LONG x,
                         LONG y, UBYTE pen, APTR text, ULONG count)
{
    cl_window *handle = (cl_window *)window_handle;
    cl_font *font = (cl_font *)font_handle;
    struct RastPort *rast_port;
    struct TextFont *old_font;

    if (handle == NULL || handle->window == NULL || font == NULL
        || font->font == NULL || text == NULL) {
        return;
    }
    rast_port = handle->window->RPort;
    old_font = rast_port->Font;
    (void)SetFont(rast_port, font->font);
    SetAPen(rast_port, (ULONG)pen);
    Move(rast_port, x, y);
    (void)Text(rast_port, (STRPTR)text, count);
    (void)SetFont(rast_port, old_font);
}

/* Create a front layer over a caller-owned bitmap. */
void *cl_layer_new(void *bitmap_handle, LONG x0, LONG y0, LONG x1, LONG y1)
{
    cl_video_bitmap *bitmap = (cl_video_bitmap *)bitmap_handle;
    cl_layer *handle;
    struct Layer *layer;
    struct Layer_Info *info;
    int width;
    int height;

    if (bitmap == NULL || x0 < 0 || y0 < 0 || x1 < x0 || y1 < y0) {
        return NULL;
    }
    if (x1 >= bitmap->width || y1 >= bitmap->height) {
        return NULL;
    }
    width = (int)(x1 - x0 + 1);
    height = (int)(y1 - y0 + 1);
    if (LayersBase == NULL) {
        LayersBase = OpenLibrary((STRPTR) "layers.library", 33);
    }
    if (LayersBase == NULL) {
        return NULL;
    }
    info = NewLayerInfo();
    if (info == NULL) {
        return NULL;
    }
    layer = CreateUpfrontLayer(info, &bitmap->bitmap,
                               x0, y0, x1, y1, LAYERSIMPLE, NULL);
    if (layer == NULL) {
        DisposeLayerInfo(info);
        return NULL;
    }
    handle = (cl_layer *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        (void)DeleteLayer(0, layer);
        DisposeLayerInfo(info);
        return NULL;
    }
    handle->layer = layer;
    handle->info = info;
    handle->bitmap_width = (int)bitmap->width;
    handle->bitmap_height = (int)bitmap->height;
    handle->width = width;
    handle->height = height;
    return handle;
}

/* Delete the layer; the screen and bitmap stay caller-owned. */
void cl_layer_free(void *handle)
{
    cl_layer *layer = (cl_layer *)handle;

    if (layer == NULL) {
        return;
    }
    if (layer->layer != NULL) {
        LONG deleted = DeleteLayer(0, layer->layer);
        if (!deleted) {
            /* The layer is still live; leak the Layer_Info rather than dispose
             * it under the library. */
            layer->layer = NULL;
            FreeMem(layer, (ULONG)sizeof *layer);
            return;
        }
        layer->layer = NULL;
    }
    if (layer->info != NULL) {
        DisposeLayerInfo(layer->info);
        layer->info = NULL;
    }
    FreeMem(layer, (ULONG)sizeof *layer);
}

/* Move the layer by a delta. */
BOOL cl_layer_move(void *handle, LONG dx, LONG dy)
{
    cl_layer *layer = (cl_layer *)handle;
    LONG min_x;
    LONG min_y;
    LONG max_x;
    LONG max_y;

    if (layer == NULL || layer->layer == NULL) {
        return FALSE;
    }
    min_x = (LONG)layer->layer->bounds.MinX + dx;
    min_y = (LONG)layer->layer->bounds.MinY + dy;
    max_x = (LONG)layer->layer->bounds.MaxX + dx;
    max_y = (LONG)layer->layer->bounds.MaxY + dy;
    if (min_x < 0 || min_y < 0 ||
        max_x >= layer->bitmap_width || max_y >= layer->bitmap_height) {
        return FALSE;
    }
    return MoveLayer(0, layer->layer, dx, dy) ? TRUE : FALSE;
}

/* Resize the layer by a delta. */
BOOL cl_layer_size(void *handle, LONG dw, LONG dh)
{
    cl_layer *layer = (cl_layer *)handle;
    LONG max_x;
    LONG max_y;
    LONG result;

    if (layer == NULL || layer->layer == NULL) {
        return FALSE;
    }
    max_x = (LONG)layer->layer->bounds.MaxX + dw;
    max_y = (LONG)layer->layer->bounds.MaxY + dh;
    if (max_x < (LONG)layer->layer->bounds.MinX ||
        max_y < (LONG)layer->layer->bounds.MinY ||
        max_x >= layer->bitmap_width || max_y >= layer->bitmap_height) {
        return FALSE;
    }
    result = SizeLayer(0, layer->layer, dw, dh);
    if (result) {
        layer->width += (int)dw;
        layer->height += (int)dh;
        return TRUE;
    }
    return FALSE;
}

/* Fill an inclusive rectangle through the layer's clipped RastPort. */
void cl_layer_fill(void *handle, LONG x1, LONG y1, LONG x2, LONG y2, UBYTE pen)
{
    cl_layer *layer = (cl_layer *)handle;

    if (layer == NULL || layer->layer == NULL) {
        return;
    }
    SetAPen(layer->layer->rp, (ULONG)pen);
    RectFill(layer->layer->rp, x1, y1, x2, y2);
}

/* Clear the whole layer. */
void cl_layer_clear(void *handle, UBYTE pen)
{
    cl_layer *layer = (cl_layer *)handle;

    if (layer == NULL || layer->layer == NULL) {
        return;
    }
    SetRast(layer->layer->rp, (ULONG)pen);
}

/* Draw text through the layer's clipped RastPort. */
void cl_layer_text(void *handle, void *font_handle, LONG x, LONG y, UBYTE pen,
                   APTR text, ULONG count)
{
    cl_layer *layer = (cl_layer *)handle;
    cl_font *font = (cl_font *)font_handle;
    struct RastPort *rast_port;
    struct TextFont *old_font;

    if (layer == NULL || layer->layer == NULL || font == NULL ||
        font->font == NULL || text == NULL) {
        return;
    }
    rast_port = layer->layer->rp;
    old_font = rast_port->Font;
    (void)SetFont(rast_port, font->font);
    SetAPen(rast_port, (ULONG)pen);
    Move(rast_port, x, y);
    (void)Text(rast_port, (STRPTR)text, count);
    (void)SetFont(rast_port, old_font);
}

/* Begin the layer damage protocol; false means the caller must not draw. */
BOOL cl_layer_begin_update(void *handle)
{
    cl_layer *layer = (cl_layer *)handle;

    if (layer == NULL || layer->layer == NULL) {
        return FALSE;
    }
    return BeginUpdate(layer->layer) ? TRUE : FALSE;
}

/* End the layer damage protocol; TRUE means the redraw is complete. */
void cl_layer_end_update(void *handle)
{
    cl_layer *layer = (cl_layer *)handle;

    if (layer != NULL && layer->layer != NULL) {
        EndUpdate(layer->layer, TRUE);
    }
}
