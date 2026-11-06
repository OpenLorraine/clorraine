/* Hosted-amiga 2.0+ shim suffix.
 *
 * This source is appended after the generated program, only for the
 * `hosted-amiga` profile. Keeping the 2.0 headers and their many macros after
 * the user code prevents names such as `WBTOOL` or `AM_VERSION` from
 * rewriting user identifiers. The 1.3 build never sees this file.
 */

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
        /* The Workbench removal call returns pointer residue rather than a
         * usable BOOL; follow the Workbench example and free the owned
         * image/title after the remove call. */
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

#include <clib/expansion_protos.h>
#include <clib/keymap_protos.h>
#include <clib/cardres_protos.h>
#include <resources/card.h>
#include <devices/inputevent.h>
#include <devices/keymap.h>

/* The base globals the clib stubs use. */
struct Library *ExpansionBase;
struct Library *KeymapBase;
struct Library *CardResource;

/* The text builders are defined later in the shim; declare them here so the
 * keymap call can build its owned result. */
void *cl_string_new(ULONG cap);
void cl_string_append(void *handle, APTR src, ULONG n);

/* Open expansion.library 37 once. */
static int cl_expansion_open(void)
{
    if (ExpansionBase == NULL) {
        ExpansionBase = OpenLibrary((STRPTR)"expansion.library", 37);
    }
    return ExpansionBase != NULL;
}

/* Walk FindConfigDev to the requested index; the nodes are library-owned. */
static struct ConfigDev *cl_expansion_board(LONG index)
{
    struct ConfigDev *board;
    LONG current = 0;

    if (index < 0 || !cl_expansion_open()) {
        return NULL;
    }
    board = FindConfigDev(NULL, -1, -1);
    while (board != NULL && current < index) {
        board = FindConfigDev(board, -1, -1);
        current++;
    }
    return board;
}

LONG cl_expansion_board_count(void)
{
    struct ConfigDev *board;
    LONG count = 0;

    if (!cl_expansion_open()) {
        return 0;
    }
    board = FindConfigDev(NULL, -1, -1);
    while (board != NULL) {
        count++;
        board = FindConfigDev(board, -1, -1);
    }
    return count;
}

UWORD cl_expansion_board_manufacturer(LONG index)
{
    const struct ConfigDev *board = cl_expansion_board(index);

    return board == NULL ? 0 : board->cd_Rom.er_Manufacturer;
}

UBYTE cl_expansion_board_product(LONG index)
{
    const struct ConfigDev *board = cl_expansion_board(index);

    return board == NULL ? 0 : board->cd_Rom.er_Product;
}

ULONG cl_expansion_board_serial(LONG index)
{
    const struct ConfigDev *board = cl_expansion_board(index);

    return board == NULL ? 0 : board->cd_Rom.er_SerialNumber;
}

UBYTE cl_expansion_board_kind(LONG index)
{
    const struct ConfigDev *board = cl_expansion_board(index);

    return board == NULL ? 0 : board->cd_Rom.er_Type;
}

/* The optional PCMCIA resource. */
BOOL cl_card_present(void)
{
    APTR card = OpenResource((STRPTR)"card.resource");

    if (card == NULL) {
        return FALSE;
    }
    CardResource = (struct Library *)card;
    RemResource(card);
    CardResource = NULL;
    return TRUE;
}

ULONG cl_card_status(void)
{
    APTR card = OpenResource((STRPTR)"card.resource");
    UBYTE status;

    if (card == NULL) {
        return 0;
    }
    CardResource = (struct Library *)card;
    status = ReadCardStatus();
    RemResource(card);
    CardResource = NULL;
    return (ULONG)status;
}

/* Open keymap.library 37 once. */
static int cl_keymap_open(void)
{
    if (KeymapBase == NULL) {
        KeymapBase = OpenLibrary((STRPTR)"keymap.library", 37);
    }
    return KeymapBase != NULL;
}

/* Map one raw key through the current keymap into an owned String, or NULL. */
void *cl_map_raw_key(UWORD code, UWORD qualifier)
{
    struct InputEvent event;
    struct KeyMap *keymap;
    char buffer[8];
    WORD length;
    void *text;

    if (!cl_keymap_open()) {
        return NULL;
    }
    keymap = AskKeyMapDefault();
    memset(&event, 0, sizeof event);
    event.ie_Class = IECLASS_RAWKEY;
    event.ie_SubClass = 0;
    event.ie_Code = code;
    event.ie_Qualifier = qualifier;
    event.ie_NextEvent = NULL;
    length = MapRawKey(&event, (STRPTR)buffer, (LONG)sizeof buffer, keymap);
    if (length <= 0) {
        return NULL;
    }
    text = cl_string_new((ULONG)length);
    if (text == NULL) {
        return NULL;
    }
    cl_string_append(text, buffer, (ULONG)length);
    return text;
}
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
