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
#include <clib/utility_protos.h>

/* utility.library base used by the DateStamp clib stub. libnix/amiga.lib
 * already provides the symbol; the shim only opens and assigns it. */
extern struct Library *UtilityBase;

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

    (void)object;
    (void)message;
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
