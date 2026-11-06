/* Bundle 9 optional UI suffix: gadtools, ASL, and BOOPSI.
 *
 * Appended after the main optional suffix, only for `hosted-amiga`. The
 * structures and hooks stay after the generated program so the OS headers
 * cannot rewrite user identifiers.
 */

#include <libraries/gadtools.h>
#include <clib/gadtools_protos.h>
#include <libraries/asl.h>
#include <clib/asl_protos.h>
#include <intuition/classusr.h>
#include <intuition/gadgetclass.h>
#include <gadgets/colorwheel.h>

struct Library *GadToolsBase;
struct Library *AslBase;

static char *cl_ui_strdup(const char *text)
{
    char *copy;
    ULONG length;

    if (text == NULL) {
        return NULL;
    }
    length = (ULONG)strlen(text) + 1UL;
    copy = (char *)AllocMem(length, MEMF_ANY | MEMF_CLEAR);
    if (copy != NULL) {
        memcpy(copy, text, (size_t)length);
    }
    return copy;
}

static int cl_gadtools_open(void)
{
    if (GadToolsBase == NULL) {
        GadToolsBase = OpenLibrary((STRPTR)"gadtools.library", 37);
    }
    return GadToolsBase != NULL;
}

static int cl_asl_open(void)
{
    if (AslBase == NULL) {
        AslBase = OpenLibrary((STRPTR)"asl.library", 37);
    }
    return AslBase != NULL;
}

/* ------------------------------------------------------------------------- */
/* gadtools.                                                                 */
/* ------------------------------------------------------------------------- */

typedef struct cl_gt_item {
    struct NewGadget ng;
    struct Gadget *gadget;
    struct TagItem tags[3];
    char *text;
    ULONG kind;
} cl_gt_item;

typedef struct cl_gt_list {
    struct Window *window;
    struct VisualInfo *visual;
    struct Gadget *glist;
    cl_gt_item *items;
    ULONG count;
    ULONG capacity;
    char *font_name;
    struct TextAttr attr;
    int built;
} cl_gt_list;

void *cl_gt_new(void *window_handle, char *font_name, UWORD font_size)
{
    cl_window *window = (cl_window *)window_handle;
    cl_gt_list *list;

    if (window == NULL || window->window == NULL || font_size < 1) {
        return NULL;
    }
    if (!cl_gadtools_open() || GfxBase == NULL) {
        return NULL;
    }
    list = (cl_gt_list *)AllocMem((ULONG)sizeof *list, MEMF_ANY | MEMF_CLEAR);
    if (list == NULL) {
        return NULL;
    }
    list->window = window->window;
    list->visual = (struct VisualInfo *)GetVisualInfoA(
        window->window->WScreen, NULL);
    if (list->visual == NULL) {
        FreeMem(list, (ULONG)sizeof *list);
        return NULL;
    }
    list->capacity = 8;
    list->items = (cl_gt_item *)AllocMem(
        list->capacity * (ULONG)sizeof(cl_gt_item), MEMF_ANY | MEMF_CLEAR);
    list->font_name = cl_ui_strdup(font_name == NULL ? "topaz.font" : font_name);
    if (list->items == NULL || list->font_name == NULL) {
        if (list->items != NULL) {
            FreeMem(list->items,
                    list->capacity * (ULONG)sizeof(cl_gt_item));
        }
        if (list->font_name != NULL) {
            FreeMem(list->font_name,
                    (ULONG)strlen(list->font_name) + 1UL);
        }
        FreeVisualInfo(list->visual);
        FreeMem(list, (ULONG)sizeof *list);
        return NULL;
    }
    list->attr.ta_Name = (STRPTR)list->font_name;
    list->attr.ta_YSize = (UBYTE)font_size;
    list->attr.ta_Style = 0;
    list->attr.ta_Flags = 0;
    return list;
}

static cl_gt_item *cl_gt_add(cl_gt_list *list, ULONG kind, LONG id,
                             char *text, LONG x, LONG y, LONG width,
                             LONG height)
{
    cl_gt_item *item;

    if (list == NULL || list->built || list->count >= list->capacity ||
        id < 1 || id > 65535) {
        return NULL;
    }
    item = &list->items[list->count];
    memset(item, 0, sizeof *item);
    item->kind = kind;
    item->text = cl_ui_strdup(text);
    if (text != NULL && item->text == NULL) {
        return NULL;
    }
    item->ng.ng_LeftEdge = (WORD)x;
    item->ng.ng_TopEdge = (WORD)y;
    item->ng.ng_Width = (WORD)width;
    item->ng.ng_Height = (WORD)height;
    item->ng.ng_GadgetText = (STRPTR)item->text;
    item->ng.ng_TextAttr = &list->attr;
    item->ng.ng_GadgetID = (UWORD)id;
    item->ng.ng_Flags = 0;
    item->ng.ng_VisualInfo = list->visual;
    item->ng.ng_UserData = NULL;
    item->tags[0].ti_Tag = TAG_DONE;
    item->tags[0].ti_Data = 0;
    list->count++;
    return item;
}

BOOL cl_gt_add_button(void *handle, LONG id, char *text, LONG x, LONG y,
                      LONG width, LONG height)
{
    return cl_gt_add((cl_gt_list *)handle, BUTTON_KIND, id, text,
                     x, y, width, height) != NULL ? TRUE : FALSE;
}

BOOL cl_gt_add_text(void *handle, LONG id, char *text, LONG x, LONG y,
                    LONG width, LONG height)
{
    cl_gt_item *item = cl_gt_add((cl_gt_list *)handle, TEXT_KIND, id, text,
                                 x, y, width, height);
    if (item == NULL) {
        return FALSE;
    }
    item->tags[0].ti_Tag = GTTX_Text;
    item->tags[0].ti_Data = (ULONG)item->text;
    item->tags[1].ti_Tag = GTTX_CopyText;
    item->tags[1].ti_Data = TRUE;
    item->tags[2].ti_Tag = TAG_DONE;
    item->tags[2].ti_Data = 0;
    return TRUE;
}

BOOL cl_gt_add_string(void *handle, LONG id, char *initial, LONG max_chars,
                      LONG x, LONG y, LONG width, LONG height)
{
    cl_gt_item *item = cl_gt_add((cl_gt_list *)handle, STRING_KIND, id,
                                 initial, x, y, width, height);
    if (item == NULL || max_chars < 1 || max_chars > 255) {
        return FALSE;
    }
    item->tags[0].ti_Tag = GTST_String;
    item->tags[0].ti_Data = (ULONG)item->text;
    item->tags[1].ti_Tag = GTST_MaxChars;
    item->tags[1].ti_Data = (ULONG)max_chars;
    item->tags[2].ti_Tag = TAG_DONE;
    item->tags[2].ti_Data = 0;
    return TRUE;
}

BOOL cl_gt_add_integer(void *handle, LONG id, LONG value, LONG max_chars,
                       LONG x, LONG y, LONG width, LONG height)
{
    cl_gt_item *item = cl_gt_add((cl_gt_list *)handle, INTEGER_KIND, id,
                                 NULL, x, y, width, height);
    if (item == NULL || max_chars < 1 || max_chars > 9) {
        return FALSE;
    }
    item->tags[0].ti_Tag = GTIN_Number;
    item->tags[0].ti_Data = (ULONG)value;
    item->tags[1].ti_Tag = GTIN_MaxChars;
    item->tags[1].ti_Data = (ULONG)max_chars;
    item->tags[2].ti_Tag = TAG_DONE;
    item->tags[2].ti_Data = 0;
    return TRUE;
}

BOOL cl_gt_build(void *handle)
{
    cl_gt_list *list = (cl_gt_list *)handle;
    struct Gadget *previous;
    ULONG index;

    if (list == NULL || list->built || list->count == 0) {
        return FALSE;
    }
    previous = CreateContext(&list->glist);
    if (previous == NULL) {
        return FALSE;
    }
    for (index = 0; index < list->count; index++) {
        cl_gt_item *item = &list->items[index];
        previous = CreateGadgetA(item->kind, previous, &item->ng,
                                 item->tags);
        if (previous == NULL) {
            return FALSE;
        }
        item->gadget = previous;
    }
    if (AddGList(list->window, list->glist, -1, (LONG)list->count, NULL) == 0) {
        return FALSE;
    }
    RefreshGList(list->glist, list->window, NULL, (LONG)list->count);
    GT_RefreshWindow(list->window, NULL);
    list->built = 1;
    return TRUE;
}

void cl_gt_free(void *handle)
{
    cl_gt_list *list = (cl_gt_list *)handle;
    ULONG index;

    if (list == NULL) {
        return;
    }
    if (list->built) {
        RemoveGList(list->window, list->glist, (LONG)list->count);
    }
    if (list->glist != NULL) {
        FreeGadgets(list->glist);
    }
    if (list->visual != NULL) {
        FreeVisualInfo(list->visual);
    }
    for (index = 0; index < list->count; index++) {
        if (list->items[index].text != NULL) {
            FreeMem(list->items[index].text,
                    (ULONG)strlen(list->items[index].text) + 1UL);
        }
    }
    if (list->items != NULL) {
        FreeMem(list->items,
                list->capacity * (ULONG)sizeof(cl_gt_item));
    }
    if (list->font_name != NULL) {
        FreeMem(list->font_name, (ULONG)strlen(list->font_name) + 1UL);
    }
    FreeMem(list, (ULONG)sizeof *list);
}

BOOL cl_gt_next_event(void *window_handle, BOOL wait)
{
    cl_window *handle = (cl_window *)window_handle;
    struct IntuiMessage *msg;

    if (handle == NULL || handle->window == NULL ||
        handle->window->UserPort == NULL) {
        return FALSE;
    }
    for (;;) {
        if (wait) {
            (void)WaitPort(handle->window->UserPort);
        }
        msg = GT_GetIMsg(handle->window->UserPort);
        if (msg == NULL) {
            return FALSE;
        }
        if (msg->Class == IDCMP_REFRESHWINDOW) {
            GT_BeginRefresh(handle->window);
            GT_EndRefresh(handle->window, TRUE);
            GT_ReplyIMsg(msg);
            continue;
        }
        if (!cl_window_fill_event(handle, msg)) {
            GT_ReplyIMsg(msg);
            continue;
        }
        GT_ReplyIMsg(msg);
        return TRUE;
    }
}

/* ------------------------------------------------------------------------- */
/* ASL file requester.                                                       */
/* ------------------------------------------------------------------------- */

typedef struct cl_asl {
    struct FileRequester *request;
    char *title;
    char *initial;
    int last_ok;
} cl_asl;

static void cl_asl_split(char *initial, char **drawer, char **file)
{
    char *last = NULL;
    char *cursor;

    *drawer = NULL;
    *file = NULL;
    if (initial == NULL) {
        return;
    }
    for (cursor = initial; *cursor != '\0'; cursor++) {
        if (*cursor == '/' || *cursor == ':') {
            last = cursor;
        }
    }
    if (last == NULL) {
        *file = initial;
        return;
    }
    if (*last == ':') {
        *drawer = initial;
        *file = last + 1;
    } else {
        *last = '\0';
        *drawer = initial;
        *file = last + 1;
    }
}

void *cl_asl_new(char *title, char *initial, BOOL save)
{
    cl_asl *handle;
    struct TagItem tags[5];
    char *drawer;
    char *file;
    int tag_count = 0;

    if (!cl_asl_open()) {
        return NULL;
    }
    handle = (cl_asl *)AllocMem((ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        return NULL;
    }
    handle->title = cl_ui_strdup(title);
    handle->initial = cl_ui_strdup(initial);
    if ((title != NULL && handle->title == NULL) ||
        (initial != NULL && handle->initial == NULL)) {
        if (handle->title != NULL) {
            FreeMem(handle->title, (ULONG)strlen(handle->title) + 1UL);
        }
        if (handle->initial != NULL) {
            FreeMem(handle->initial, (ULONG)strlen(handle->initial) + 1UL);
        }
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    tags[tag_count].ti_Tag = ASLFR_TitleText;
    tags[tag_count].ti_Data = (ULONG)handle->title;
    tag_count++;
    cl_asl_split(handle->initial, &drawer, &file);
    if (drawer != NULL) {
        tags[tag_count].ti_Tag = ASLFR_InitialDrawer;
        tags[tag_count].ti_Data = (ULONG)drawer;
        tag_count++;
    }
    if (file != NULL) {
        tags[tag_count].ti_Tag = ASLFR_InitialFile;
        tags[tag_count].ti_Data = (ULONG)file;
        tag_count++;
    }
    if (save) {
        tags[tag_count].ti_Tag = ASLFR_DoSaveMode;
        tags[tag_count].ti_Data = TRUE;
        tag_count++;
    }
    tags[tag_count].ti_Tag = TAG_DONE;
    tags[tag_count].ti_Data = 0;
    handle->request = (struct FileRequester *)AllocAslRequest(
        ASL_FileRequest, tags);
    if (handle->request == NULL) {
        FreeMem(handle->title, (ULONG)strlen(handle->title) + 1UL);
        FreeMem(handle->initial, (ULONG)strlen(handle->initial) + 1UL);
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    return handle;
}

BOOL cl_asl_request(void *handle_void, void *window_handle)
{
    cl_asl *handle = (cl_asl *)handle_void;
    cl_window *window = (cl_window *)window_handle;
    struct TagItem tags[2];

    if (handle == NULL || handle->request == NULL ||
        window == NULL || window->window == NULL) {
        return FALSE;
    }
    tags[0].ti_Tag = ASLFR_Window;
    tags[0].ti_Data = (ULONG)window->window;
    tags[1].ti_Tag = TAG_DONE;
    tags[1].ti_Data = 0;
    handle->last_ok = AslRequest(handle->request, tags) ? 1 : 0;
    return handle->last_ok ? TRUE : FALSE;
}

LONG cl_asl_path(void *handle_void, APTR buffer, ULONG length)
{
    cl_asl *handle = (cl_asl *)handle_void;
    const unsigned char *file;
    const unsigned char *drawer;
    ULONG count;

    if (handle == NULL || buffer == NULL || length == 0 || !handle->last_ok) {
        return -1;
    }
    file = handle->request->fr_File;
    drawer = handle->request->fr_Drawer;
    if (file != NULL && (strchr((const char *)file, ':') != NULL ||
                         file[0] == '/')) {
        drawer = NULL;
    }
    count = 0;
    if (drawer != NULL && drawer[0] != '\0') {
        while (drawer[count] != '\0' && count + 1UL < length) {
            ((char *)buffer)[count] = drawer[count];
            count++;
        }
        if (file != NULL && file[0] != '\0' && count + 1UL < length) {
            ((char *)buffer)[count++] = '/';
        }
    }
    if (file != NULL) {
        ULONG index = 0;
        while (file[index] != '\0' && count + 1UL < length) {
            ((char *)buffer)[count++] = file[index++];
        }
    }
    ((char *)buffer)[count] = '\0';
    return (LONG)count;
}

void cl_asl_free(void *handle_void)
{
    cl_asl *handle = (cl_asl *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->request != NULL) {
        FreeAslRequest(handle->request);
    }
    if (handle->title != NULL) {
        FreeMem(handle->title, (ULONG)strlen(handle->title) + 1UL);
    }
    if (handle->initial != NULL) {
        FreeMem(handle->initial, (ULONG)strlen(handle->initial) + 1UL);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

/* ------------------------------------------------------------------------- */
/* BOOPSI colorwheel.                                                        */
/* ------------------------------------------------------------------------- */

typedef struct cl_boopsi {
    Object *object;
    struct Window *window;
    int attached;
} cl_boopsi;

void *cl_boopsi_new_colorwheel(void *window_handle, LONG id, LONG x, LONG y,
                               LONG width, LONG height)
{
    cl_window *window = (cl_window *)window_handle;
    cl_boopsi *handle;
    struct TagItem tags[7];

    if (window == NULL || window->window == NULL || id < 1 || id > 65535) {
        return NULL;
    }
    tags[0].ti_Tag = GA_ID;
    tags[0].ti_Data = (ULONG)id;
    tags[1].ti_Tag = GA_Left;
    tags[1].ti_Data = (ULONG)x;
    tags[2].ti_Tag = GA_Top;
    tags[2].ti_Data = (ULONG)y;
    tags[3].ti_Tag = GA_Width;
    tags[3].ti_Data = (ULONG)width;
    tags[4].ti_Tag = GA_Height;
    tags[4].ti_Data = (ULONG)height;
    tags[5].ti_Tag = WHEEL_Screen;
    tags[5].ti_Data = (ULONG)window->window->WScreen;
    tags[6].ti_Tag = TAG_DONE;
    tags[6].ti_Data = 0;
    handle = (cl_boopsi *)AllocMem((ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        return NULL;
    }
    handle->object = (Object *)NewObjectA(NULL,
        (STRPTR)"colorwheel.gadget", tags);
    if (handle->object == NULL) {
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    return handle;
}

BOOL cl_boopsi_attach(void *handle_void, void *window_handle)
{
    cl_boopsi *handle = (cl_boopsi *)handle_void;
    cl_window *window = (cl_window *)window_handle;

    if (handle == NULL || handle->object == NULL || handle->attached ||
        window == NULL || window->window == NULL) {
        return FALSE;
    }
    if (AddGList(window->window, (struct Gadget *)handle->object, -1, 1,
                 NULL) == 0) {
        return FALSE;
    }
    RefreshGList((struct Gadget *)handle->object, window->window, NULL, 1);
    handle->window = window->window;
    handle->attached = 1;
    return TRUE;
}

void cl_boopsi_detach(void *handle_void)
{
    cl_boopsi *handle = (cl_boopsi *)handle_void;

    if (handle == NULL || !handle->attached || handle->window == NULL) {
        return;
    }
    RemoveGList(handle->window, (struct Gadget *)handle->object, 1);
    RefreshGList(handle->window->FirstGadget, handle->window, NULL, -1);
    handle->attached = 0;
    handle->window = NULL;
}

BOOL cl_boopsi_set_hue(void *handle_void, ULONG hue)
{
    cl_boopsi *handle = (cl_boopsi *)handle_void;
    struct TagItem tags[2];

    if (handle == NULL || handle->object == NULL || !handle->attached ||
        handle->window == NULL) {
        return FALSE;
    }
    tags[0].ti_Tag = WHEEL_Hue;
    tags[0].ti_Data = hue;
    tags[1].ti_Tag = TAG_DONE;
    tags[1].ti_Data = 0;
    SetGadgetAttrsA((struct Gadget *)handle->object, handle->window, NULL,
                    tags);
    return TRUE;
}

ULONG cl_boopsi_hue(void *handle_void)
{
    cl_boopsi *handle = (cl_boopsi *)handle_void;
    ULONG value = 0;

    if (handle == NULL || handle->object == NULL) {
        return 0;
    }
    (void)GetAttr(WHEEL_Hue, handle->object, &value);
    return value;
}

void cl_boopsi_dispose(void *handle_void)
{
    cl_boopsi *handle = (cl_boopsi *)handle_void;

    if (handle == NULL) {
        return;
    }
    cl_boopsi_detach(handle);
    if (handle->object != NULL) {
        DisposeObject(handle->object);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}
