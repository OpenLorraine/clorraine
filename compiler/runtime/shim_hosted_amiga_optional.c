/* Bundle 9 optional shim: hosted-amiga optional suffix.
 *
 * This source is appended after the existing 2.0 Workbench/icon/expansion
 * suffix. It hosts the Bundle 9 surfaces: advanced Exec/utility, Intuition,
 * DOS, graphics/input, Workbench services, networking, and AmigaGuide.
 * Keeping it after the generated program means the OS headers' macros cannot
 * rewrite user identifiers.
 */

#include <exec/memory.h>
#include <exec/semaphores.h>
#include <utility/date.h>
#include <utility/hooks.h>
#include <utility/tagitem.h>
#include <clib/exec_protos.h>

/* utility.library base used by the DateStamp clib stub. libnix provides the
 * symbol for gcc, while vbcc's amiga.lib does not; define it for vbcc. */
#ifdef __VBCC__
struct Library *UtilityBase;
#else
extern struct Library *UtilityBase;
#endif

static int cl_utility_open(void)
{
    if (UtilityBase == NULL) {
        UtilityBase = OpenLibrary((STRPTR) "utility.library", 37);
    }
    return UtilityBase != NULL;
}

/* ------------------------------------------------------------------------- */
/* Signal semaphores.                                                        */
/* ------------------------------------------------------------------------- */

typedef struct cl_semaphore {
    struct SignalSemaphore semaphore;
} cl_semaphore;

void *cl_create_semaphore(void)
{
    cl_semaphore *handle = (cl_semaphore *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);

    if (handle == NULL) {
        return NULL;
    }
    InitSemaphore(&handle->semaphore);
    return handle;
}

void cl_free_semaphore(void *handle)
{
    if (handle != NULL) {
        FreeMem(handle, (ULONG)sizeof(cl_semaphore));
    }
}

void cl_obtain_semaphore(void *handle)
{
    cl_semaphore *semaphore = (cl_semaphore *)handle;

    if (semaphore != NULL) {
        ObtainSemaphore(&semaphore->semaphore);
    }
}

void cl_release_semaphore(void *handle)
{
    cl_semaphore *semaphore = (cl_semaphore *)handle;

    if (semaphore != NULL) {
        ReleaseSemaphore(&semaphore->semaphore);
    }
}

BOOL cl_attempt_semaphore(void *handle)
{
    cl_semaphore *semaphore = (cl_semaphore *)handle;

    if (semaphore == NULL) {
        return FALSE;
    }
    return AttemptSemaphore(&semaphore->semaphore) ? TRUE : FALSE;
}

/* ------------------------------------------------------------------------- */
/* Tag lists.                                                                */
/* ------------------------------------------------------------------------- */

typedef struct cl_tag_list {
    struct TagItem *items;
    ULONG capacity;
    ULONG count;
} cl_tag_list;

void *cl_new_tag_list(LONG capacity)
{
    cl_tag_list *list;
    ULONG size;

    if (capacity < 1 || capacity > 63) {
        return NULL;
    }
    size = ((ULONG)capacity + 1UL) * (ULONG)sizeof(struct TagItem);
    list = (cl_tag_list *)AllocMem((ULONG)sizeof *list, MEMF_ANY | MEMF_CLEAR);
    if (list == NULL) {
        return NULL;
    }
    list->items = (struct TagItem *)AllocMem(size, MEMF_ANY | MEMF_CLEAR);
    if (list->items == NULL) {
        FreeMem(list, (ULONG)sizeof *list);
        return NULL;
    }
    list->capacity = (ULONG)capacity;
    list->count = 0;
    list->items[0].ti_Tag = TAG_DONE;
    return list;
}

void cl_free_tag_list(void *handle)
{
    cl_tag_list *list = (cl_tag_list *)handle;

    if (list == NULL) {
        return;
    }
    if (list->items != NULL) {
        FreeMem(list->items,
                (list->capacity + 1UL) * (ULONG)sizeof(struct TagItem));
    }
    FreeMem(list, (ULONG)sizeof *list);
}

BOOL cl_tag_set(void *handle, ULONG tag, ULONG data)
{
    cl_tag_list *list = (cl_tag_list *)handle;
    ULONG index;

    if (list == NULL || tag == TAG_DONE) {
        return FALSE;
    }
    for (index = 0; index < list->count; index++) {
        if (list->items[index].ti_Tag == tag) {
            list->items[index].ti_Data = data;
            return TRUE;
        }
    }
    if (list->count >= list->capacity) {
        return FALSE;
    }
    list->items[list->count].ti_Tag = tag;
    list->items[list->count].ti_Data = data;
    list->count++;
    list->items[list->count].ti_Tag = TAG_DONE;
    return TRUE;
}

ULONG cl_tag_get(void *handle, ULONG tag)
{
    const cl_tag_list *list = (const cl_tag_list *)handle;
    ULONG index;

    if (list == NULL) {
        return 0;
    }
    for (index = 0; index < list->count; index++) {
        if (list->items[index].ti_Tag == tag) {
            return list->items[index].ti_Data;
        }
    }
    return 0;
}

BOOL cl_tag_present(void *handle, ULONG tag)
{
    const cl_tag_list *list = (const cl_tag_list *)handle;
    ULONG index;

    if (list == NULL) {
        return FALSE;
    }
    for (index = 0; index < list->count; index++) {
        if (list->items[index].ti_Tag == tag) {
            return TRUE;
        }
    }
    return FALSE;
}

LONG cl_tag_count(void *handle)
{
    const cl_tag_list *list = (const cl_tag_list *)handle;

    return list == NULL ? 0 : (LONG)list->count;
}

/* ------------------------------------------------------------------------- */
/* Date stamps.                                                              */
/* ------------------------------------------------------------------------- */

typedef struct cl_date_stamp {
    struct DateStamp stamp;
} cl_date_stamp;

void *cl_date_stamp_new(void)
{
    cl_date_stamp *handle = (cl_date_stamp *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);

    if (handle == NULL || !cl_utility_open()) {
        if (handle != NULL) {
            FreeMem(handle, (ULONG)sizeof *handle);
        }
        return NULL;
    }
    DateStamp(&handle->stamp);
    return handle;
}

void cl_free_date_stamp(void *handle)
{
    if (handle != NULL) {
        FreeMem(handle, (ULONG)sizeof(cl_date_stamp));
    }
}

LONG cl_date_stamp_days(void *handle)
{
    const cl_date_stamp *stamp = (const cl_date_stamp *)handle;

    return stamp == NULL ? 0 : stamp->stamp.ds_Days;
}

LONG cl_date_stamp_minute(void *handle)
{
    const cl_date_stamp *stamp = (const cl_date_stamp *)handle;

    return stamp == NULL ? 0 : stamp->stamp.ds_Minute;
}

LONG cl_date_stamp_tick(void *handle)
{
    const cl_date_stamp *stamp = (const cl_date_stamp *)handle;

    return stamp == NULL ? 0 : stamp->stamp.ds_Tick;
}

/* ------------------------------------------------------------------------- */
/* Hooks.                                                                    */
/* ------------------------------------------------------------------------- */

typedef void (*cl_hook_fn)(void);

typedef struct cl_hook {
    struct Hook hook;
    cl_hook_fn entry;
    LONG data;
    ULONG calls;
} cl_hook;

static LONG cl_hook_data_value;
static LONG cl_hook_current_result;

static ULONG cl_hook_trampoline(struct Hook *hook, APTR object, APTR message)
{
    cl_hook *record = (cl_hook *)hook->h_Data;

#ifndef __VBCC__
    (void)object;
    (void)message;
#endif
    cl_hook_data_value = record->data;
    cl_hook_current_result = 0;
    record->entry();
    record->calls++;
    return (ULONG)cl_hook_current_result;
}

void *cl_new_hook(cl_hook_fn entry, LONG data)
{
    cl_hook *record;

    if (entry == NULL) {
        return NULL;
    }
    record = (cl_hook *)AllocMem((ULONG)sizeof *record, MEMF_ANY | MEMF_CLEAR);
    if (record == NULL) {
        return NULL;
    }
    record->hook.h_Entry = cl_hook_trampoline;
    record->hook.h_Data = record;
    record->entry = entry;
    record->data = data;
    return record;
}

LONG cl_call_hook(void *handle)
{
    cl_hook *record = (cl_hook *)handle;

    if (record == NULL) {
        return 0;
    }
    cl_hook_data_value = record->data;
    cl_hook_current_result = 0;
    (void)record->hook.h_Entry(&record->hook, NULL, NULL);
    record->calls++;
    return cl_hook_current_result;
}

LONG cl_hook_data(void *handle)
{
    const cl_hook *record = (const cl_hook *)handle;

    return record == NULL ? 0 : record->data;
}

LONG cl_hook_current_data(void)
{
    return cl_hook_data_value;
}

void cl_hook_set_result(LONG value)
{
    cl_hook_current_result = value;
}

void cl_free_hook(void *handle)
{
    if (handle != NULL) {
        FreeMem(handle, (ULONG)sizeof(cl_hook));
    }
}

/* ------------------------------------------------------------------------- */
/* Utility memory pools (V39).                                               */
/* ------------------------------------------------------------------------- */

typedef struct cl_utility_pool {
    APTR pool;
    ULONG live_blocks;
    int released;
} cl_utility_pool;

typedef struct cl_utility_block {
    cl_utility_pool *pool;
    APTR memory;
    ULONG size;
} cl_utility_block;

static void cl_utility_pool_release(cl_utility_pool *pool)
{
    if (pool == NULL) {
        return;
    }
    if (pool->released && pool->live_blocks == 0) {
        if (pool->pool != NULL) {
            DeletePool(pool->pool);
        }
        FreeMem(pool, (ULONG)sizeof *pool);
    }
}

void *cl_create_utility_pool(ULONG mem_flags, ULONG puddle_size,
                             ULONG thresh_size)
{
    cl_utility_pool *pool;

    /* CreatePool is an exec.library V39 vector. */
    if (SysBase == NULL || SysBase->LibNode.lib_Version < 39) {
        return NULL;
    }
    pool = (cl_utility_pool *)AllocMem(
        (ULONG)sizeof *pool, MEMF_ANY | MEMF_CLEAR);
    if (pool == NULL) {
        return NULL;
    }
    pool->pool = CreatePool(mem_flags, puddle_size, thresh_size);
    if (pool->pool == NULL) {
        FreeMem(pool, (ULONG)sizeof *pool);
        return NULL;
    }
    return pool;
}

void *cl_alloc_utility_block(void *handle, ULONG size)
{
    cl_utility_pool *pool = (cl_utility_pool *)handle;
    cl_utility_block *block;

    if (pool == NULL || pool->released || size == 0) {
        return NULL;
    }
    block = (cl_utility_block *)AllocMem(
        (ULONG)sizeof *block, MEMF_ANY | MEMF_CLEAR);
    if (block == NULL) {
        return NULL;
    }
    block->memory = AllocPooled(pool->pool, size);
    if (block->memory == NULL) {
        FreeMem(block, (ULONG)sizeof *block);
        return NULL;
    }
    block->pool = pool;
    block->size = size;
    pool->live_blocks++;
    return block;
}

ULONG cl_utility_block_size(void *handle)
{
    const cl_utility_block *block = (const cl_utility_block *)handle;

    return block == NULL ? 0 : block->size;
}

BOOL cl_write_utility_block(void *handle, APTR data, ULONG length)
{
    cl_utility_block *block = (cl_utility_block *)handle;
    ULONG count;

    if (block == NULL || data == NULL) {
        return FALSE;
    }
    count = length < block->size ? length : block->size;
    if (count > 0) {
        memcpy(block->memory, data, (size_t)count);
    }
    return TRUE;
}

LONG cl_read_utility_block(void *handle, APTR data, ULONG length)
{
    const cl_utility_block *block = (const cl_utility_block *)handle;
    ULONG count;

    if (block == NULL || data == NULL) {
        return -1;
    }
    count = length < block->size ? length : block->size;
    if (count > 0) {
        memcpy(data, block->memory, (size_t)count);
    }
    return (LONG)count;
}

void cl_free_utility_block(void *handle)
{
    cl_utility_block *block = (cl_utility_block *)handle;
    cl_utility_pool *pool;

    if (block == NULL) {
        return;
    }
    pool = block->pool;
    if (pool != NULL && pool->pool != NULL) {
        FreePooled(pool->pool, block->memory, block->size);
        if (pool->live_blocks > 0) {
            pool->live_blocks--;
        }
    }
    FreeMem(block, (ULONG)sizeof *block);
    cl_utility_pool_release(pool);
}

void cl_free_utility_pool(void *handle)
{
    cl_utility_pool *pool = (cl_utility_pool *)handle;

    if (pool == NULL) {
        return;
    }
    pool->released = 1;
    cl_utility_pool_release(pool);
}

/* ------------------------------------------------------------------------- */
/* ReadArgs.                                                                 */
/* ------------------------------------------------------------------------- */

#include <dos/rdargs.h>

/* The DOS prototypes cannot be included wholesale: they conflict with the
 * generated file functions. Declare only the argument helpers. */
extern struct RDArgs * __stdargs ReadArgs(STRPTR template, LONG *array,
                                          struct RDArgs *args);
extern void __stdargs FreeArgs(struct RDArgs *args);
extern APTR __stdargs AllocDosObject(ULONG type, struct TagItem *tags);
extern void __stdargs FreeDosObject(ULONG type, APTR object);

typedef struct cl_args {
    struct RDArgs *rdargs;
    int owns_rdargs;            /* From reads allocate the RDArgs record */
    LONG *slots;
    UBYTE *kinds;               /* 0 string, 1 number, 2 switch */
    UBYTE *source;
    ULONG source_size;          /* exact allocation size of `source` */
    LONG count;
} cl_args;

/* Count and classify a simple comma-separated ReadArgs template. */
static LONG cl_args_parse(const char *template, UBYTE *kinds, LONG max)
{
    const char *cursor = template;
    LONG count = 0;

    if (template == NULL) {
        return -1;
    }
    while (*cursor != '\0') {
        UBYTE kind = 0;

        while (*cursor == ' ' || *cursor == '\t') {
            cursor++;
        }
        if (*cursor == '\0') {
            break;
        }
        if (count >= max) {
            return -1;
        }
        while (*cursor != '\0' && *cursor != '/' && *cursor != ',') {
            cursor++;
        }
        if (*cursor == '/') {
            cursor++;
            switch (*cursor) {
            case 'A':
            case 'K':
                kind = 0;
                break;
            case 'N':
                kind = 1;
                break;
            case 'S':
                kind = 2;
                break;
            default:
                return -1;
            }
            cursor++;
        }
        kinds[count] = kind;
        count++;
        if (*cursor == ',') {
            cursor++;
        } else if (*cursor != '\0') {
            return -1;
        }
    }
    return count;
}

static void cl_args_discard(cl_args *args)
{
    if (args == NULL) {
        return;
    }
    if (args->rdargs != NULL) {
        FreeArgs(args->rdargs);
        if (args->owns_rdargs) {
            FreeDosObject(DOS_RDARGS, args->rdargs);
        }
    }
    if (args->slots != NULL) {
        FreeMem(args->slots, (ULONG)args->count * (ULONG)sizeof(LONG));
    }
    if (args->kinds != NULL) {
        FreeMem(args->kinds, (ULONG)args->count);
    }
    if (args->source != NULL) {
        FreeMem(args->source, args->source_size);
    }
    FreeMem(args, (ULONG)sizeof *args);
}

void *cl_read_args(char *template, LONG slots)
{
    cl_args *args;
    UBYTE *kinds;
    LONG count;

    if (template == NULL || slots < 1 || slots > 32) {
        return NULL;
    }
    kinds = (UBYTE *)AllocMem((ULONG)slots, MEMF_ANY | MEMF_CLEAR);
    if (kinds == NULL) {
        return NULL;
    }
    count = cl_args_parse(template, kinds, slots);
    if (count != slots) {
        FreeMem(kinds, (ULONG)slots);
        return NULL;
    }
    args = (cl_args *)AllocMem((ULONG)sizeof *args, MEMF_ANY | MEMF_CLEAR);
    if (args == NULL) {
        FreeMem(kinds, (ULONG)slots);
        return NULL;
    }
    args->slots = (LONG *)AllocMem(
        (ULONG)slots * (ULONG)sizeof(LONG), MEMF_ANY | MEMF_CLEAR);
    if (args->slots == NULL) {
        FreeMem(kinds, (ULONG)slots);
        FreeMem(args, (ULONG)sizeof *args);
        return NULL;
    }
    args->kinds = kinds;
    args->count = slots;
    args->rdargs = ReadArgs((STRPTR)template, args->slots, NULL);
    if (args->rdargs == NULL) {
        cl_args_discard(args);
        return NULL;
    }
    return args;
}

void *cl_read_args_from(char *template, LONG slots, APTR source,
                        ULONG length)
{
    cl_args *args;
    UBYTE *kinds;
    UBYTE *buffer;
    ULONG source_size;
    LONG count;

    if (template == NULL || source == NULL || slots < 1 || slots > 32 ||
        length >= 0xFFFFFFF0UL) {
        return NULL;
    }
    kinds = (UBYTE *)AllocMem((ULONG)slots, MEMF_ANY | MEMF_CLEAR);
    if (kinds == NULL) {
        return NULL;
    }
    count = cl_args_parse(template, kinds, slots);
    if (count != slots) {
        FreeMem(kinds, (ULONG)slots);
        return NULL;
    }
    args = (cl_args *)AllocMem((ULONG)sizeof *args, MEMF_ANY | MEMF_CLEAR);
    if (args == NULL) {
        FreeMem(kinds, (ULONG)slots);
        return NULL;
    }
    args->slots = (LONG *)AllocMem(
        (ULONG)slots * (ULONG)sizeof(LONG), MEMF_ANY | MEMF_CLEAR);
    source_size = length + 1UL;
    buffer = (UBYTE *)AllocMem(source_size, MEMF_ANY | MEMF_CLEAR);
    if (args->slots == NULL || buffer == NULL) {
        if (args->slots != NULL) {
            FreeMem(args->slots, (ULONG)slots * (ULONG)sizeof(LONG));
        }
        if (buffer != NULL) {
            FreeMem(buffer, source_size);
        }
        FreeMem(kinds, (ULONG)slots);
        FreeMem(args, (ULONG)sizeof *args);
        return NULL;
    }
    memcpy(buffer, source, (size_t)length);
    if (length == 0 || buffer[length - 1] != '\n') {
        buffer[length] = '\n';
        length++;
    }
    args->kinds = kinds;
    args->count = slots;
    args->source = buffer;
    args->source_size = source_size;
    args->rdargs = AllocDosObject(DOS_RDARGS, NULL);
    if (args->rdargs == NULL) {
        FreeMem(buffer, source_size);
        FreeMem(args->slots, (ULONG)slots * (ULONG)sizeof(LONG));
        FreeMem(kinds, (ULONG)slots);
        FreeMem(args, (ULONG)sizeof *args);
        return NULL;
    }
    args->owns_rdargs = 1;
    args->rdargs->RDA_Source.CS_Buffer = (STRPTR)buffer;
    args->rdargs->RDA_Source.CS_Length = (LONG)length;
    args->rdargs->RDA_Source.CS_CurChr = 0;
    args->rdargs->RDA_Flags = RDAF_NOPROMPT;
    if (ReadArgs((STRPTR)template, args->slots, args->rdargs) == NULL) {
        cl_args_discard(args);
        return NULL;
    }
    return args;
}

void cl_free_args(void *handle)
{
    cl_args_discard((cl_args *)handle);
}

LONG cl_arg_count(void *handle)
{
    const cl_args *args = (const cl_args *)handle;

    return args == NULL ? 0 : args->count;
}

BOOL cl_arg_present(void *handle, LONG index)
{
    const cl_args *args = (const cl_args *)handle;

    if (args == NULL || index < 0 || index >= args->count) {
        return FALSE;
    }
    return args->slots[index] != 0 ? TRUE : FALSE;
}

LONG cl_arg_number(void *handle, LONG index)
{
    const cl_args *args = (const cl_args *)handle;

    if (args == NULL || index < 0 || index >= args->count ||
        args->kinds[index] != 1 || args->slots[index] == 0) {
        return 0;
    }
    return *(LONG *)args->slots[index];
}

LONG cl_arg_text(void *handle, LONG index, APTR buffer, ULONG length)
{
    const cl_args *args = (const cl_args *)handle;
    const char *text;
    ULONG count;

    if (args == NULL || buffer == NULL || length == 0 || index < 0 ||
        index >= args->count || args->kinds[index] != 0 ||
        args->slots[index] == 0) {
        return -1;
    }
    text = (const char *)args->slots[index];
    count = (ULONG)strlen(text);
    if (count >= length) {
        count = length - 1UL;
    }
    memcpy(buffer, text, (size_t)count);
    ((char *)buffer)[count] = '\0';
    return (LONG)count;
}

BOOL cl_arg_switch(void *handle, LONG index)
{
    const cl_args *args = (const cl_args *)handle;

    if (args == NULL || index < 0 || index >= args->count ||
        args->kinds[index] != 2) {
        return FALSE;
    }
    return args->slots[index] != 0 ? TRUE : FALSE;
}

/* ------------------------------------------------------------------------- */
/* EClock.                                                                   */
/* ------------------------------------------------------------------------- */

#include <devices/timer.h>
#include <clib/timer_protos.h>

/* libnix/amiga.lib does not provide TimerBase; the shim owns it. */
struct Device *TimerBase;

typedef struct cl_eclock {
    struct TimeRequest *request;
    struct EClockVal last;
    ULONG frequency;
} cl_eclock;

void *cl_eclock_open(void)
{
    cl_eclock *clock = (cl_eclock *)AllocMem(
        (ULONG)sizeof *clock, MEMF_ANY | MEMF_CLEAR);

    if (clock == NULL) {
        return NULL;
    }
    clock->request = (struct TimeRequest *)AllocMem(
        (ULONG)sizeof(struct TimeRequest), MEMF_PUBLIC | MEMF_CLEAR);
    if (clock->request == NULL) {
        FreeMem(clock, (ULONG)sizeof *clock);
        return NULL;
    }
    if (OpenDevice((STRPTR)"timer.device", UNIT_ECLOCK,
                   &clock->request->tr_node, 0) != 0) {
        FreeMem(clock->request, (ULONG)sizeof(struct TimeRequest));
        FreeMem(clock, (ULONG)sizeof *clock);
        return NULL;
    }
    return clock;
}

ULONG cl_eclock_read(void *handle)
{
    cl_eclock *clock = (cl_eclock *)handle;

    if (clock == NULL) {
        return 0;
    }
    TimerBase = clock->request->tr_node.io_Device;
    clock->frequency = ReadEClock(&clock->last);
    return clock->frequency;
}

ULONG cl_eclock_hi(void *handle)
{
    const cl_eclock *clock = (const cl_eclock *)handle;

    return clock == NULL ? 0 : clock->last.ev_hi;
}

ULONG cl_eclock_lo(void *handle)
{
    const cl_eclock *clock = (const cl_eclock *)handle;

    return clock == NULL ? 0 : clock->last.ev_lo;
}

ULONG cl_eclock_frequency(void *handle)
{
    const cl_eclock *clock = (const cl_eclock *)handle;

    return clock == NULL ? 0 : clock->frequency;
}

void cl_free_eclock(void *handle)
{
    cl_eclock *clock = (cl_eclock *)handle;

    if (clock == NULL) {
        return;
    }
    if (clock->request != NULL) {
        CloseDevice(&clock->request->tr_node);
        FreeMem(clock->request, (ULONG)sizeof(struct TimeRequest));
    }
    FreeMem(clock, (ULONG)sizeof *clock);
}

/* ------------------------------------------------------------------------- */
/* Lowlevel ReadJoyPort.                                                     */
/* ------------------------------------------------------------------------- */

#include <libraries/lowlevel.h>
#include <clib/lowlevel_protos.h>

struct Library *LowLevelBase;

static int cl_lowlevel_open(void)
{
    if (LowLevelBase == NULL) {
        LowLevelBase = OpenLibrary((STRPTR)"lowlevel.library", 40);
    }
    return LowLevelBase != NULL;
}

ULONG cl_lowlevel_read(ULONG port)
{
    if (!cl_lowlevel_open() || port > 1) {
        return 0;
    }
    return ReadJoyPort(port);
}

ULONG cl_lowlevel_version(void)
{
    if (!cl_lowlevel_open()) {
        return 0;
    }
    return LowLevelBase->lib_Version;
}

ULONG cl_lowlevel_type(ULONG raw)
{
    return (raw & JP_TYPE_MASK) >> 28;
}

UBYTE cl_lowlevel_decode(ULONG raw)
{
    ULONG type = raw & JP_TYPE_MASK;
    UBYTE bits = 0;

    if (type == JP_TYPE_GAMECTLR || type == JP_TYPE_JOYSTK) {
        if ((raw & JPF_JOY_UP) != 0) { bits |= 1; }
        if ((raw & JPF_JOY_DOWN) != 0) { bits |= 2; }
        if ((raw & JPF_JOY_LEFT) != 0) { bits |= 4; }
        if ((raw & JPF_JOY_RIGHT) != 0) { bits |= 8; }
    }
    if (type != JP_TYPE_NOTAVAIL && (raw & JPF_BUTTON_RED) != 0) {
        bits |= 16;
    }
    return bits;
}

/* ------------------------------------------------------------------------- */
/* Regions and layer clipping.                                               */
/* ------------------------------------------------------------------------- */

#include <graphics/regions.h>
#include <clib/graphics_protos.h>
#include <clib/layers_protos.h>

typedef struct cl_region {
    struct Region *region;
} cl_region;

static int cl_region_open(void)
{
    if (GfxBase == NULL) {
        GfxBase = (struct GfxBase *)OpenLibrary((STRPTR)"graphics.library", 37);
    }
    if (LayersBase == NULL) {
        LayersBase = OpenLibrary((STRPTR)"layers.library", 37);
    }
    return GfxBase != NULL && LayersBase != NULL;
}

void *cl_region_new(void)
{
    cl_region *handle;

    if (!cl_region_open()) {
        return NULL;
    }
    handle = (cl_region *)AllocMem((ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        return NULL;
    }
    handle->region = NewRegion();
    if (handle->region == NULL) {
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    return handle;
}

void cl_free_region(void *handle)
{
    cl_region *region = (cl_region *)handle;

    if (region == NULL) {
        return;
    }
    if (region->region != NULL) {
        DisposeRegion(region->region);
    }
    FreeMem(region, (ULONG)sizeof *region);
}

static void cl_region_rect(struct Rectangle *rect, LONG x1, LONG y1,
                           LONG x2, LONG y2)
{
    rect->MinX = (WORD)x1;
    rect->MinY = (WORD)y1;
    rect->MaxX = (WORD)x2;
    rect->MaxY = (WORD)y2;
}

BOOL cl_region_or_rect(void *handle, LONG x1, LONG y1, LONG x2, LONG y2)
{
    cl_region *region = (cl_region *)handle;
    struct Rectangle rect;

    if (region == NULL || region->region == NULL) {
        return FALSE;
    }
    cl_region_rect(&rect, x1, y1, x2, y2);
    return OrRectRegion(region->region, &rect) ? TRUE : FALSE;
}

void cl_region_and_rect(void *handle, LONG x1, LONG y1, LONG x2, LONG y2)
{
    cl_region *region = (cl_region *)handle;
    struct Rectangle rect;

    if (region == NULL || region->region == NULL) {
        return;
    }
    cl_region_rect(&rect, x1, y1, x2, y2);
    AndRectRegion(region->region, &rect);
}

BOOL cl_region_clear_rect(void *handle, LONG x1, LONG y1, LONG x2, LONG y2)
{
    cl_region *region = (cl_region *)handle;
    struct Rectangle rect;

    if (region == NULL || region->region == NULL) {
        return FALSE;
    }
    cl_region_rect(&rect, x1, y1, x2, y2);
    return ClearRectRegion(region->region, &rect) ? TRUE : FALSE;
}

LONG cl_region_x1(void *handle)
{
    const cl_region *region = (const cl_region *)handle;
    return region == NULL ? 0 : region->region->bounds.MinX;
}

LONG cl_region_y1(void *handle)
{
    const cl_region *region = (const cl_region *)handle;
    return region == NULL ? 0 : region->region->bounds.MinY;
}

LONG cl_region_x2(void *handle)
{
    const cl_region *region = (const cl_region *)handle;
    return region == NULL ? 0 : region->region->bounds.MaxX;
}

LONG cl_region_y2(void *handle)
{
    const cl_region *region = (const cl_region *)handle;
    return region == NULL ? 0 : region->region->bounds.MaxY;
}

BOOL cl_region_empty(void *handle)
{
    const cl_region *region = (const cl_region *)handle;

    if (region == NULL || region->region == NULL) {
        return TRUE;
    }
    return region->region->bounds.MaxX < region->region->bounds.MinX ||
           region->region->bounds.MaxY < region->region->bounds.MinY;
}

void cl_layer_set_clip(void *layer_handle, void *region_handle)
{
    cl_layer *layer = (cl_layer *)layer_handle;
    cl_region *region = (cl_region *)region_handle;

    if (layer == NULL || layer->layer == NULL || region == NULL ||
        region->region == NULL) {
        return;
    }
    (void)InstallClipRegion(layer->layer, region->region);
}

void cl_layer_clear_clip(void *layer_handle)
{
    cl_layer *layer = (cl_layer *)layer_handle;

    if (layer == NULL || layer->layer == NULL) {
        return;
    }
    (void)InstallClipRegion(layer->layer, NULL);
}

/* ------------------------------------------------------------------------- */
/* Datatypes picture objects (V40).                                          */
/* ------------------------------------------------------------------------- */

#include <datatypes/datatypes.h>
#include <datatypes/datatypesclass.h>
#include <datatypes/pictureclass.h>
#include <clib/datatypes_protos.h>

struct Library *DataTypesBase;

typedef struct cl_dt {
    Object *object;
    Object *template_object;
    LONG width;
    LONG height;
    LONG depth;
    LONG colors;
} cl_dt;

static int cl_datatypes_open(void)
{
    if (DataTypesBase == NULL) {
        DataTypesBase = OpenLibrary((STRPTR)"datatypes.library", 40);
        /* The datatype class list is populated by AddDataTypes; give the
         * library a moment before the first class lookup. */
        if (DataTypesBase != NULL) {
            Delay(5);
        }
    }
    return DataTypesBase != NULL;
}

void *cl_dt_open_picture(char *path)
{
    cl_dt *handle;
    struct TagItem tags[2];
    static char path_copy[256];

    if (path == NULL || !cl_datatypes_open()) {
        return NULL;
    }
    strncpy(path_copy, path, sizeof path_copy - 1);
    path_copy[sizeof path_copy - 1] = '\0';
    tags[0].ti_Tag = DTA_GroupID;
    tags[0].ti_Data = GID_PICTURE;
    tags[1].ti_Tag = TAG_DONE;
    tags[1].ti_Data = 0;
    handle = (cl_dt *)AllocMem((ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        return NULL;
    }
    /* The first lookup initializes the disk datatype class; keep it alive
     * while the second object parses the bitmap. */
    handle->template_object = NewDTObjectA((STRPTR)path_copy, tags);
    Delay(25);
    handle->object = NewDTObjectA((STRPTR)path_copy, tags);
    if (handle->object == NULL) {
        if (handle->template_object != NULL) {
            DisposeDTObject(handle->template_object);
        }
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    {
        struct TagItem a[3];
        struct BitMapHeader *h = NULL;
        /* PDTA_NumColors is documented as a UWORD value, but GetDTAttrsA
         * writes tag data through a ULONG slot. A UWORD destination corrupts
         * the adjacent stack on the licensed 3.1 class. */
        ULONG colors = 0;

        a[0].ti_Tag = PDTA_BitMapHeader; a[0].ti_Data = (ULONG)&h;
        a[1].ti_Tag = PDTA_NumColors; a[1].ti_Data = (ULONG)&colors;
        a[2].ti_Tag = TAG_DONE; a[2].ti_Data = 0;
        (void)GetDTAttrsA(handle->object, a);
        if (h != NULL) {
            handle->width = h->bmh_Width;
            handle->height = h->bmh_Height;
            handle->depth = h->bmh_Depth;
        }
        if (colors != 0) {
            handle->colors = (LONG)colors;
        } else if (handle->depth > 0 && handle->depth <= 8) {
            /* The 3.1 ilbm.datatype may leave PDTA_NumColors zero. */
            handle->colors = 1L << handle->depth;
        }
    }
    return handle;
}

void cl_dispose_dt_object(void *handle)
{
    cl_dt *object = (cl_dt *)handle;

    if (object == NULL) {
        return;
    }
    if (object->object != NULL) {
        DisposeDTObject(object->object);
    }
    if (object->template_object != NULL) {
        DisposeDTObject(object->template_object);
    }
    FreeMem(object, (ULONG)sizeof *object);
}

LONG cl_dt_width(void *handle)
{
    const cl_dt *object = (const cl_dt *)handle;
    return object == NULL ? 0 : object->width;
}

LONG cl_dt_height(void *handle)
{
    const cl_dt *object = (const cl_dt *)handle;
    return object == NULL ? 0 : object->height;
}

LONG cl_dt_depth(void *handle)
{
    const cl_dt *object = (const cl_dt *)handle;
    return object == NULL ? 0 : object->depth;
}

LONG cl_dt_colors(void *handle)
{
    const cl_dt *object = (const cl_dt *)handle;

    if (object == NULL) {
        return 0;
    }
    if (object->colors != 0) {
        return object->colors;
    }
    /* The 3.1 ilbm.datatype may leave PDTA_NumColors zero; a bitmap of
     * `depth` planes has at most 2^depth palette entries. */
    if (object->depth > 0 && object->depth <= 8) {
        return 1L << object->depth;
    }
    return 0;
}

BOOL cl_dt_draw(void *handle, void *window_handle, LONG x, LONG y)
{
    cl_dt *object = (cl_dt *)handle;
    cl_window *window = (cl_window *)window_handle;
    struct dtDraw draw;

    if (object == NULL || object->object == NULL || window == NULL ||
        window->window == NULL || object->width <= 0 || object->height <= 0) {
        return FALSE;
    }
    memset(&draw, 0, sizeof draw);
    draw.MethodID = DTM_DRAW;
    draw.dtd_RPort = window->window->RPort;
    draw.dtd_Left = x;
    draw.dtd_Top = y;
    draw.dtd_Width = object->width;
    draw.dtd_Height = object->height;
    draw.dtd_AttrList = NULL;
    (void)DoDTMethodA(object->object, window->window, NULL, (Msg)&draw);
    return TRUE;
}
