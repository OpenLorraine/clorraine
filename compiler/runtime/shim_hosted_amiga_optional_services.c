/* Bundle 9 optional services suffix: commodities, locale, ARexx, clipboard,
 * serial/parallel/printer, and nonvolatile. Appended after the UI suffix.
 */

#include <libraries/commodities.h>
#include <clib/commodities_protos.h>
#include <libraries/locale.h>
#include <clib/locale_protos.h>
#include <rexx/storage.h>
#include <rexx/rxslib.h>
#include <clib/rexxsyslib_protos.h>
#include <devices/clipboard.h>
#include <devices/serial.h>
#include <devices/parallel.h>
#include <devices/printer.h>
#include <libraries/nonvolatile.h>
#include <clib/nonvolatile_protos.h>

struct Library *CxBase;
struct LocaleBase *LocaleBase;
struct RxsLib *RexxSysBase;
struct Library *NVBase;

static char *cl_service_strdup(const char *text)
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

/* ------------------------------------------------------------------------- */
/* Commodities.                                                              */
/* ------------------------------------------------------------------------- */

typedef struct cl_commodity {
    CxObj *broker;
    char *name;
    char *title;
    char *description;
} cl_commodity;

static void cl_commodity_free_strings(cl_commodity *commodity)
{
    if (commodity->name != NULL) {
        FreeMem(commodity->name, (ULONG)strlen(commodity->name) + 1UL);
    }
    if (commodity->title != NULL) {
        FreeMem(commodity->title, (ULONG)strlen(commodity->title) + 1UL);
    }
    if (commodity->description != NULL) {
        FreeMem(commodity->description,
                (ULONG)strlen(commodity->description) + 1UL);
    }
}

void *cl_commodity_open(char *name, char *title, char *description,
                        void *port, BOOL unique, LONG priority)
{
    cl_commodity *commodity;
    struct NewBroker broker;
    LONG error = 0;

    if (name == NULL || port == NULL) {
        return NULL;
    }
    if (CxBase == NULL) {
        CxBase = OpenLibrary((STRPTR)"commodities.library", 37);
    }
    if (CxBase == NULL) {
        return NULL;
    }
    commodity = (cl_commodity *)AllocMem(
        (ULONG)sizeof *commodity, MEMF_ANY | MEMF_CLEAR);
    if (commodity == NULL) {
        return NULL;
    }
    commodity->name = cl_service_strdup(name);
    commodity->title = cl_service_strdup(title);
    commodity->description = cl_service_strdup(description);
    if (commodity->name == NULL) {
        cl_commodity_free_strings(commodity);
        FreeMem(commodity, (ULONG)sizeof *commodity);
        return NULL;
    }
    memset(&broker, 0, sizeof broker);
    broker.nb_Version = NB_VERSION;
    broker.nb_Name = (STRPTR)commodity->name;
    broker.nb_Title = (STRPTR)commodity->title;
    broker.nb_Descr = (STRPTR)commodity->description;
    broker.nb_Unique = unique ? NBU_UNIQUE : NBU_DUPLICATE;
    broker.nb_Flags = 0;
    if (priority < -128) {
        priority = -128;
    } else if (priority > 127) {
        priority = 127;
    }
    broker.nb_Pri = (BYTE)priority;
    broker.nb_Port = (struct MsgPort *)port;
    broker.nb_ReservedChannel = 0;
    commodity->broker = CxBroker(&broker, &error);
    if (commodity->broker == NULL) {
        cl_commodity_free_strings(commodity);
        FreeMem(commodity, (ULONG)sizeof *commodity);
        return NULL;
    }
    return commodity;
}

void cl_commodity_close(void *handle)
{
    cl_commodity *commodity = (cl_commodity *)handle;

    if (commodity == NULL) {
        return;
    }
    if (commodity->broker != NULL) {
        DeleteCxObjAll(commodity->broker);
    }
    cl_commodity_free_strings(commodity);
    FreeMem(commodity, (ULONG)sizeof *commodity);
}

/* ------------------------------------------------------------------------- */
/* Locale.                                                                   */
/* ------------------------------------------------------------------------- */

typedef struct cl_locale {
    struct Locale *locale;
} cl_locale;

void *cl_locale_open(char *name)
{
    cl_locale *handle;

    if (LocaleBase == NULL) {
        LocaleBase = (struct LocaleBase *)OpenLibrary(
            (STRPTR)"locale.library", 38);
    }
    if (LocaleBase == NULL) {
        return NULL;
    }
    handle = (cl_locale *)AllocMem((ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        return NULL;
    }
    handle->locale = OpenLocale(
        name == NULL || name[0] == '\0' ? NULL : (STRPTR)name);
    if (handle->locale == NULL) {
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    return handle;
}

void cl_locale_close(void *handle)
{
    cl_locale *locale = (cl_locale *)handle;

    if (locale == NULL) {
        return;
    }
    if (locale->locale != NULL) {
        CloseLocale(locale->locale);
    }
    FreeMem(locale, (ULONG)sizeof *locale);
}

void *cl_locale_language(void *handle)
{
    const cl_locale *locale = (const cl_locale *)handle;
    const char *text;
    void *string;

    if (locale == NULL || locale->locale == NULL ||
        locale->locale->loc_LanguageName == NULL) {
        return NULL;
    }
    text = (const char *)locale->locale->loc_LanguageName;
    string = cl_string_new((ULONG)strlen(text));
    if (string != NULL) {
        cl_string_append(string, (APTR)text, (ULONG)strlen(text));
    }
    return string;
}

ULONG cl_locale_country(void *handle)
{
    const cl_locale *locale = (const cl_locale *)handle;

    return locale == NULL || locale->locale == NULL
        ? 0 : locale->locale->loc_CountryCode;
}

LONG cl_locale_gmt(void *handle)
{
    const cl_locale *locale = (const cl_locale *)handle;

    return locale == NULL || locale->locale == NULL
        ? 0 : locale->locale->loc_GMTOffset;
}

void *cl_locale_date_format(void *handle)
{
    const cl_locale *locale = (const cl_locale *)handle;
    const char *text;
    void *string;

    if (locale == NULL || locale->locale == NULL ||
        locale->locale->loc_DateTimeFormat == NULL) {
        return NULL;
    }
    text = (const char *)locale->locale->loc_DateTimeFormat;
    string = cl_string_new((ULONG)strlen(text));
    if (string != NULL) {
        cl_string_append(string, (APTR)text, (ULONG)strlen(text));
    }
    return string;
}

/* ------------------------------------------------------------------------- */
/* ARexx messages.                                                           */
/* ------------------------------------------------------------------------- */

typedef struct cl_rexx_message {
    struct RexxMsg *message;
    int owns;
} cl_rexx_message;

/* CreateRexxMsg stores the extension and host pointers without copying them,
 * but cl_rexx_send releases the wrapper before the reply arrives. Keep the
 * copies in a packet-lifetime record keyed by the message pointer. */
typedef struct cl_rexx_lifetime {
    struct RexxMsg *message;
    char *extension;
    char *host;
    struct cl_rexx_lifetime *next;
} cl_rexx_lifetime;

static cl_rexx_lifetime *cl_rexx_lifetimes;

static int cl_rexx_open(void)
{
    if (RexxSysBase == NULL) {
        RexxSysBase = (struct RxsLib *)OpenLibrary(
            (STRPTR)"rexxsyslib.library", 36);
    }
    return RexxSysBase != NULL;
}

/* Returns a received message when it has a reply port. A message without one
 * cannot be replied to; dropping it is safer than dereferencing a null port. */
static void cl_rexx_return(struct RexxMsg *message)
{
    if (message != NULL && message->rm_Node.mn_ReplyPort != NULL) {
        ReplyMsg((struct Message *)message);
    }
}

/* True when this process created the packet and still holds its lifetime
 * record. The record is the ownership truth: a received reply that is not
 * ours must never be passed to DeleteRexxMsg or have its arguments freed. */
static int cl_rexx_has_lifetime(const struct RexxMsg *message)
{
    cl_rexx_lifetime *record = cl_rexx_lifetimes;

    while (record != NULL) {
        if (record->message == message) {
            return 1;
        }
        record = record->next;
    }
    return 0;
}

/* True when the message is safe to read as a RexxMsg. IsRexxMsg accepts a
 * packet from a real ARexx host; a packet created by CreateRexxMsg round-trips
 * through this process's own ports and is identified by its lifetime record.
 * The packet length is checked first: a generic Message is 20-24 bytes, while
 * RexxMsg is 128, so a short foreign message is rejected before any field
 * beyond the standard Message is read. */
static int cl_rexx_is_packet(const struct RexxMsg *message)
{
    if (message == NULL ||
        message->rm_Node.mn_Length < sizeof(struct RexxMsg)) {
        return 0;
    }
    return IsRexxMsg(message) || cl_rexx_has_lifetime(message);
}

/* Releases every argument string owned by a created packet. DeleteRexxMsg
 * does not release arguments. rm_Result2 is a string only when the request
 * asked for RXFF_RESULT and the call succeeded; otherwise it is a numeric
 * result or error code and must be left alone. */
static void cl_rexx_free_args(struct RexxMsg *message)
{
    int index;

    if (message == NULL) {
        return;
    }
    for (index = 0; index < 16; index++) {
        if (message->rm_Args[index] != NULL) {
            DeleteArgstring((UBYTE *)message->rm_Args[index]);
            message->rm_Args[index] = NULL;
        }
    }
    if (message->rm_Result1 == 0 &&
        (message->rm_Action & RXFF_RESULT) != 0 &&
        message->rm_Result2 != 0) {
        DeleteArgstring((UBYTE *)message->rm_Result2);
        message->rm_Result2 = 0;
    }
}

static cl_rexx_lifetime *cl_rexx_take_lifetime(struct RexxMsg *message)
{
    cl_rexx_lifetime **link = &cl_rexx_lifetimes;

    while (*link != NULL) {
        if ((*link)->message == message) {
            cl_rexx_lifetime *record = *link;
            *link = record->next;
            return record;
        }
        link = &(*link)->next;
    }
    return NULL;
}

static void cl_rexx_free_lifetime(cl_rexx_lifetime *record)
{
    if (record == NULL) {
        return;
    }
    if (record->extension != NULL) {
        FreeMem(record->extension, (ULONG)strlen(record->extension) + 1UL);
    }
    if (record->host != NULL) {
        FreeMem(record->host, (ULONG)strlen(record->host) + 1UL);
    }
    FreeMem(record, (ULONG)sizeof *record);
}

/* Releases a packet this process created. The lifetime record must be the one
 * taken from the list by the caller; it is freed after DeleteRexxMsg so the
 * extension/host copies outlive the packet as CreateRexxMsg requires. */
static void cl_rexx_release_packet(struct RexxMsg *message,
                                   cl_rexx_lifetime *record)
{
    if (message == NULL) {
        cl_rexx_free_lifetime(record);
        return;
    }
    cl_rexx_free_args(message);
    DeleteRexxMsg(message);
    cl_rexx_free_lifetime(record);
}

void *cl_rexx_create(void *reply_port, char *extension, char *host)
{
    cl_rexx_message *handle;
    cl_rexx_lifetime *lifetime;

    if (reply_port == NULL || !cl_rexx_open()) {
        return NULL;
    }
    handle = (cl_rexx_message *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        return NULL;
    }
    lifetime = (cl_rexx_lifetime *)AllocMem(
        (ULONG)sizeof *lifetime, MEMF_ANY | MEMF_CLEAR);
    if (lifetime == NULL) {
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    lifetime->extension = cl_service_strdup(extension);
    lifetime->host = cl_service_strdup(host);
    if ((extension != NULL && lifetime->extension == NULL) ||
        (host != NULL && lifetime->host == NULL)) {
        cl_rexx_free_lifetime(lifetime);
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    lifetime->message = CreateRexxMsg((struct MsgPort *)reply_port,
                                      (STRPTR)lifetime->extension,
                                      (STRPTR)lifetime->host);
    if (lifetime->message == NULL) {
        cl_rexx_free_lifetime(lifetime);
        FreeMem(handle, (ULONG)sizeof *handle);
        return NULL;
    }
    lifetime->next = cl_rexx_lifetimes;
    cl_rexx_lifetimes = lifetime;
    handle->message = lifetime->message;
    handle->owns = 1;
    return handle;
}

void cl_rexx_delete(void *handle_void)
{
    cl_rexx_message *handle = (cl_rexx_message *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->message != NULL) {
        cl_rexx_lifetime *record = cl_rexx_take_lifetime(handle->message);
        if (record != NULL) {
            cl_rexx_release_packet(handle->message, record);
        } else if (handle->owns) {
            cl_rexx_free_args(handle->message);
            DeleteRexxMsg(handle->message);
        } else {
            cl_rexx_return(handle->message);
        }
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

BOOL cl_rexx_set_action(void *handle_void, LONG action)
{
    cl_rexx_message *handle = (cl_rexx_message *)handle_void;

    if (handle == NULL || handle->message == NULL) {
        return FALSE;
    }
    handle->message->rm_Action = action;
    return TRUE;
}

BOOL cl_rexx_set_arg(void *handle_void, LONG index, char *value)
{
    cl_rexx_message *handle = (cl_rexx_message *)handle_void;
    UBYTE *argument;

    if (handle == NULL || handle->message == NULL || value == NULL ||
        index < 0 || index > 15) {
        return FALSE;
    }
    argument = CreateArgstring((STRPTR)value, (ULONG)strlen(value));
    if (argument == NULL) {
        return FALSE;
    }
    if (handle->message->rm_Args[index] != NULL) {
        DeleteArgstring((UBYTE *)handle->message->rm_Args[index]);
    }
    handle->message->rm_Args[index] = (STRPTR)argument;
    return TRUE;
}

void cl_rexx_send(void *port, void *handle_void)
{
    cl_rexx_message *handle = (cl_rexx_message *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->message != NULL && port != NULL) {
        PutMsg((struct MsgPort *)port, (struct Message *)handle->message);
        FreeMem(handle, (ULONG)sizeof *handle);
        return;
    }
    if (handle->message != NULL) {
        cl_rexx_lifetime *record = cl_rexx_take_lifetime(handle->message);
        if (record != NULL) {
            cl_rexx_release_packet(handle->message, record);
        } else if (handle->owns) {
            cl_rexx_free_args(handle->message);
            DeleteRexxMsg(handle->message);
        } else {
            cl_rexx_return(handle->message);
        }
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

void *cl_rexx_receive(void *port)
{
    cl_rexx_message *handle;
    struct RexxMsg *message;

    if (port == NULL || !cl_rexx_open()) {
        return NULL;
    }
    message = (struct RexxMsg *)GetMsg((struct MsgPort *)port);
    if (message == NULL) {
        return NULL;
    }
    if (!cl_rexx_is_packet(message)) {
        /* A foreign message must be returned to its sender, not inspected as
         * a RexxMsg (whose argument fields start at byte 40). */
        cl_rexx_return(message);
        return NULL;
    }
    handle = (cl_rexx_message *)AllocMem(
        (ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        cl_rexx_return(message);
        return NULL;
    }
    handle->message = message;
    handle->owns = 0;
    return handle;
}

void cl_rexx_reply(void *handle_void)
{
    cl_rexx_message *handle = (cl_rexx_message *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->message != NULL) {
        cl_rexx_return(handle->message);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

void cl_rexx_reclaim(void *handle_void)
{
    cl_rexx_message *handle = (cl_rexx_message *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->message != NULL) {
        cl_rexx_lifetime *record = cl_rexx_take_lifetime(handle->message);
        if (record != NULL) {
            cl_rexx_release_packet(handle->message, record);
        } else {
            /* Not a packet this process created; never delete or free the
             * sender's arguments. Return it to its reply port instead. */
            cl_rexx_return(handle->message);
        }
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

LONG cl_rexx_action(void *handle_void)
{
    const cl_rexx_message *handle = (const cl_rexx_message *)handle_void;

    return handle == NULL || handle->message == NULL
        ? 0 : handle->message->rm_Action;
}

LONG cl_rexx_result1(void *handle_void)
{
    const cl_rexx_message *handle = (const cl_rexx_message *)handle_void;

    return handle == NULL || handle->message == NULL
        ? 0 : handle->message->rm_Result1;
}

LONG cl_rexx_result2(void *handle_void)
{
    const cl_rexx_message *handle = (const cl_rexx_message *)handle_void;

    return handle == NULL || handle->message == NULL
        ? 0 : handle->message->rm_Result2;
}

LONG cl_rexx_arg(void *handle_void, LONG index, APTR buffer, ULONG length)
{
    const cl_rexx_message *handle = (const cl_rexx_message *)handle_void;
    const char *text;
    ULONG count;

    if (handle == NULL || handle->message == NULL || buffer == NULL ||
        length == 0 || index < 0 || index > 15 ||
        handle->message->rm_Args[index] == NULL) {
        return -1;
    }
    text = (const char *)handle->message->rm_Args[index];
    count = (ULONG)LengthArgstring((STRPTR)text);
    if (count >= length) {
        count = length - 1UL;
    }
    memcpy(buffer, text, (size_t)count);
    ((char *)buffer)[count] = '\0';
    return (LONG)count;
}

/* ------------------------------------------------------------------------- */
/* Clipboard.                                                                */
/* ------------------------------------------------------------------------- */

typedef struct cl_clipboard {
    struct IOClipReq *request;
    struct MsgPort *port;
} cl_clipboard;

void *cl_clipboard_open(LONG unit)
{
    cl_clipboard *clipboard;

    if (unit < 0) {
        return NULL;
    }
    clipboard = (cl_clipboard *)AllocMem(
        (ULONG)sizeof *clipboard, MEMF_ANY | MEMF_CLEAR);
    if (clipboard == NULL) {
        return NULL;
    }
    clipboard->port = cl_device_port();
    if (clipboard->port == NULL) {
        FreeMem(clipboard, (ULONG)sizeof *clipboard);
        return NULL;
    }
    clipboard->request = (struct IOClipReq *)AllocMem(
        (ULONG)sizeof(struct IOClipReq), MEMF_PUBLIC | MEMF_CLEAR);
    if (clipboard->request == NULL) {
        DeletePort(clipboard->port);
        FreeMem(clipboard, (ULONG)sizeof *clipboard);
        return NULL;
    }
    clipboard->request->io_Message.mn_ReplyPort = clipboard->port;
    clipboard->request->io_Message.mn_Length =
        (UWORD)sizeof(struct IOClipReq);
    if (OpenDevice((STRPTR)"clipboard.device", unit,
                   (struct IORequest *)clipboard->request, 0) != 0) {
        FreeMem(clipboard->request, (ULONG)sizeof(struct IOClipReq));
        DeletePort(clipboard->port);
        FreeMem(clipboard, (ULONG)sizeof *clipboard);
        return NULL;
    }
    return clipboard;
}

void cl_clipboard_close(void *handle)
{
    cl_clipboard *clipboard = (cl_clipboard *)handle;

    if (clipboard == NULL) {
        return;
    }
    if (clipboard->request != NULL) {
        CloseDevice((struct IORequest *)clipboard->request);
        FreeMem(clipboard->request, (ULONG)sizeof(struct IOClipReq));
    }
    if (clipboard->port != NULL) {
        DeletePort(clipboard->port);
    }
    FreeMem(clipboard, (ULONG)sizeof *clipboard);
}

BOOL cl_clipboard_write(void *handle, APTR data, ULONG length)
{
    cl_clipboard *clipboard = (cl_clipboard *)handle;
    LONG attempts;

    if (clipboard == NULL || data == NULL) {
        return FALSE;
    }
    for (attempts = 0; attempts < 3; attempts++) {
        /* A new cut starts with io_ClipID zero; CMD_WRITE assigns the ID. */
        clipboard->request->io_ClipID = 0;
        clipboard->request->io_Command = CMD_WRITE;
        clipboard->request->io_Data = (STRPTR)data;
        clipboard->request->io_Length = length;
        clipboard->request->io_Offset = 0;
        if (DoIO((struct IORequest *)clipboard->request) != 0) {
            return FALSE;
        }
        clipboard->request->io_Command = CMD_UPDATE;
        if (DoIO((struct IORequest *)clipboard->request) == 0) {
            return TRUE;
        }
        if (clipboard->request->io_Error != CBERR_OBSOLETEID) {
            return FALSE;
        }
    }
    return FALSE;
}

LONG cl_clipboard_read(void *handle, APTR data, ULONG length)
{
    cl_clipboard *clipboard = (cl_clipboard *)handle;
    LONG attempts;

    if (clipboard == NULL || data == NULL) {
        return -1;
    }
    for (attempts = 0; attempts < 3; attempts++) {
        clipboard->request->io_Command = CBD_CURRENTREADID;
        if (DoIO((struct IORequest *)clipboard->request) != 0) {
            return -1;
        }
        clipboard->request->io_Command = CMD_READ;
        clipboard->request->io_Data = (STRPTR)data;
        clipboard->request->io_Length = length;
        clipboard->request->io_Offset = 0;
        if (DoIO((struct IORequest *)clipboard->request) == 0) {
            return (LONG)clipboard->request->io_Actual;
        }
        if (clipboard->request->io_Error != CBERR_OBSOLETEID) {
            return -1;
        }
    }
    return -1;
}

BOOL cl_clipboard_clear(void *handle)
{
    return cl_clipboard_write(handle, (APTR)"", 0);
}

/* ------------------------------------------------------------------------- */
/* Serial, parallel, and printer.                                            */
/* ------------------------------------------------------------------------- */

typedef struct cl_serial {
    struct IOExtSer *request;
    struct MsgPort *port;
} cl_serial;

typedef struct cl_parallel {
    struct IOExtPar *request;
    struct MsgPort *port;
} cl_parallel;

typedef struct cl_printer {
    struct IOStdReq *request;
    struct MsgPort *port;
} cl_printer;

void *cl_serial_open(LONG unit, ULONG baud)
{
    cl_serial *serial;

    if (unit < 0 || baud < 300 || baud > 115200) {
        return NULL;
    }
    serial = (cl_serial *)AllocMem((ULONG)sizeof *serial, MEMF_ANY | MEMF_CLEAR);
    if (serial == NULL) {
        return NULL;
    }
    serial->port = cl_device_port();
    serial->request = (struct IOExtSer *)AllocMem(
        (ULONG)sizeof(struct IOExtSer), MEMF_PUBLIC | MEMF_CLEAR);
    if (serial->port == NULL || serial->request == NULL) {
        if (serial->port != NULL) { DeletePort(serial->port); }
        if (serial->request != NULL) {
            FreeMem(serial->request, (ULONG)sizeof(struct IOExtSer));
        }
        FreeMem(serial, (ULONG)sizeof *serial);
        return NULL;
    }
    serial->request->io_SerFlags = 0;
    serial->request->io_Baud = baud;
    serial->request->IOSer.io_Message.mn_ReplyPort = serial->port;
    if (OpenDevice((STRPTR)"serial.device", unit,
                   (struct IORequest *)serial->request, 0) != 0) {
        FreeMem(serial->request, (ULONG)sizeof(struct IOExtSer));
        DeletePort(serial->port);
        FreeMem(serial, (ULONG)sizeof *serial);
        return NULL;
    }
    serial->request->IOSer.io_Command = SDCMD_SETPARAMS;
    if (DoIO((struct IORequest *)serial->request) != 0) {
        CloseDevice((struct IORequest *)serial->request);
        FreeMem(serial->request, (ULONG)sizeof(struct IOExtSer));
        DeletePort(serial->port);
        FreeMem(serial, (ULONG)sizeof *serial);
        return NULL;
    }
    return serial;
}

void cl_serial_close(void *handle)
{
    cl_serial *serial = (cl_serial *)handle;

    if (serial == NULL) {
        return;
    }
    if (serial->request != NULL) {
        CloseDevice((struct IORequest *)serial->request);
        FreeMem(serial->request, (ULONG)sizeof(struct IOExtSer));
    }
    if (serial->port != NULL) {
        DeletePort(serial->port);
    }
    FreeMem(serial, (ULONG)sizeof *serial);
}

LONG cl_serial_write(void *handle, APTR data, ULONG length)
{
    cl_serial *serial = (cl_serial *)handle;

    if (serial == NULL || data == NULL) {
        return -1;
    }
    serial->request->IOSer.io_Command = CMD_WRITE;
    serial->request->IOSer.io_Data = (APTR)data;
    serial->request->IOSer.io_Length = length;
    if (DoIO((struct IORequest *)serial->request) != 0) {
        return -1;
    }
    return (LONG)serial->request->IOSer.io_Actual;
}

LONG cl_serial_read(void *handle, APTR data, ULONG length)
{
    cl_serial *serial = (cl_serial *)handle;

    if (serial == NULL || data == NULL) {
        return -1;
    }
    serial->request->IOSer.io_Command = CMD_READ;
    serial->request->IOSer.io_Data = (APTR)data;
    serial->request->IOSer.io_Length = length;
    if (DoIO((struct IORequest *)serial->request) != 0) {
        return -1;
    }
    return (LONG)serial->request->IOSer.io_Actual;
}

void *cl_parallel_open(LONG unit, ULONG flags)
{
    cl_parallel *parallel;

    if (unit < 0) {
        return NULL;
    }
    parallel = (cl_parallel *)AllocMem(
        (ULONG)sizeof *parallel, MEMF_ANY | MEMF_CLEAR);
    if (parallel == NULL) {
        return NULL;
    }
    parallel->port = cl_device_port();
    parallel->request = (struct IOExtPar *)AllocMem(
        (ULONG)sizeof(struct IOExtPar), MEMF_PUBLIC | MEMF_CLEAR);
    if (parallel->port == NULL || parallel->request == NULL) {
        if (parallel->port != NULL) { DeletePort(parallel->port); }
        if (parallel->request != NULL) {
            FreeMem(parallel->request, (ULONG)sizeof(struct IOExtPar));
        }
        FreeMem(parallel, (ULONG)sizeof *parallel);
        return NULL;
    }
    parallel->request->io_ParFlags = (UBYTE)flags;
    parallel->request->IOPar.io_Message.mn_ReplyPort = parallel->port;
    if (OpenDevice((STRPTR)"parallel.device", unit,
                   (struct IORequest *)parallel->request, 0) != 0) {
        FreeMem(parallel->request, (ULONG)sizeof(struct IOExtPar));
        DeletePort(parallel->port);
        FreeMem(parallel, (ULONG)sizeof *parallel);
        return NULL;
    }
    parallel->request->IOPar.io_Command = PDCMD_SETPARAMS;
    if (DoIO((struct IORequest *)parallel->request) != 0) {
        CloseDevice((struct IORequest *)parallel->request);
        FreeMem(parallel->request, (ULONG)sizeof(struct IOExtPar));
        DeletePort(parallel->port);
        FreeMem(parallel, (ULONG)sizeof *parallel);
        return NULL;
    }
    return parallel;
}

void cl_parallel_close(void *handle)
{
    cl_parallel *parallel = (cl_parallel *)handle;

    if (parallel == NULL) {
        return;
    }
    if (parallel->request != NULL) {
        CloseDevice((struct IORequest *)parallel->request);
        FreeMem(parallel->request, (ULONG)sizeof(struct IOExtPar));
    }
    if (parallel->port != NULL) {
        DeletePort(parallel->port);
    }
    FreeMem(parallel, (ULONG)sizeof *parallel);
}

LONG cl_parallel_write(void *handle, APTR data, ULONG length)
{
    cl_parallel *parallel = (cl_parallel *)handle;

    if (parallel == NULL || data == NULL) {
        return -1;
    }
    parallel->request->IOPar.io_Command = CMD_WRITE;
    parallel->request->IOPar.io_Data = (APTR)data;
    parallel->request->IOPar.io_Length = length;
    if (DoIO((struct IORequest *)parallel->request) != 0) {
        return -1;
    }
    return (LONG)parallel->request->IOPar.io_Actual;
}

void *cl_printer_open(LONG unit)
{
    cl_printer *printer;

    if (unit < 0) {
        return NULL;
    }
    printer = (cl_printer *)AllocMem(
        (ULONG)sizeof *printer, MEMF_ANY | MEMF_CLEAR);
    if (printer == NULL) {
        return NULL;
    }
    printer->port = cl_device_port();
    printer->request = (struct IOStdReq *)AllocMem(
        (ULONG)sizeof(struct IOStdReq), MEMF_PUBLIC | MEMF_CLEAR);
    if (printer->port == NULL || printer->request == NULL) {
        if (printer->port != NULL) { DeletePort(printer->port); }
        if (printer->request != NULL) {
            FreeMem(printer->request, (ULONG)sizeof(struct IOStdReq));
        }
        FreeMem(printer, (ULONG)sizeof *printer);
        return NULL;
    }
    printer->request->io_Message.mn_ReplyPort = printer->port;
    if (OpenDevice((STRPTR)"printer.device", unit,
                   (struct IORequest *)printer->request, 0) != 0) {
        FreeMem(printer->request, (ULONG)sizeof(struct IOStdReq));
        DeletePort(printer->port);
        FreeMem(printer, (ULONG)sizeof *printer);
        return NULL;
    }
    return printer;
}

void cl_printer_close(void *handle)
{
    cl_printer *printer = (cl_printer *)handle;

    if (printer == NULL) {
        return;
    }
    if (printer->request != NULL) {
        CloseDevice((struct IORequest *)printer->request);
        FreeMem(printer->request, (ULONG)sizeof(struct IOStdReq));
    }
    if (printer->port != NULL) {
        DeletePort(printer->port);
    }
    FreeMem(printer, (ULONG)sizeof *printer);
}

LONG cl_printer_write(void *handle, APTR data, ULONG length)
{
    cl_printer *printer = (cl_printer *)handle;

    if (printer == NULL || data == NULL) {
        return -1;
    }
    printer->request->io_Command = PRD_RAWWRITE;
    printer->request->io_Data = (APTR)data;
    printer->request->io_Length = length;
    if (DoIO((struct IORequest *)printer->request) != 0) {
        return -1;
    }
    return (LONG)printer->request->io_Actual;
}

/* ------------------------------------------------------------------------- */
/* Nonvolatile storage.                                                      */
/* ------------------------------------------------------------------------- */

typedef struct cl_nv_data {
    APTR data;
    ULONG size;
} cl_nv_data;

static int cl_nv_open(void)
{
    if (NVBase == NULL) {
        NVBase = OpenLibrary((STRPTR)"nonvolatile.library", 40);
    }
    return NVBase != NULL;
}

void *cl_nv_get(char *app, char *item)
{
    cl_nv_data *handle;
    APTR data;

    if (app == NULL || item == NULL || !cl_nv_open()) {
        return NULL;
    }
    data = GetCopyNV((STRPTR)app, (STRPTR)item, TRUE);
    if (data == NULL) {
        return NULL;
    }
    handle = (cl_nv_data *)AllocMem((ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        FreeNVData(data);
        return NULL;
    }
    handle->data = data;
    handle->size = SizeNVData(data);
    return handle;
}

void cl_nv_free(void *handle_void)
{
    cl_nv_data *handle = (cl_nv_data *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->data != NULL) {
        FreeNVData(handle->data);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

ULONG cl_nv_size(void *handle_void)
{
    const cl_nv_data *handle = (const cl_nv_data *)handle_void;

    return handle == NULL ? 0 : handle->size;
}

LONG cl_nv_read(void *handle_void, APTR buffer, ULONG length)
{
    const cl_nv_data *handle = (const cl_nv_data *)handle_void;
    ULONG count;

    if (handle == NULL || buffer == NULL) {
        return -1;
    }
    count = length < handle->size ? length : handle->size;
    if (count > 0) {
        memcpy(buffer, handle->data, (size_t)count);
    }
    return (LONG)count;
}

BOOL cl_nv_store(char *app, char *item, APTR data, ULONG length)
{
    ULONG units;

    if (app == NULL || item == NULL || data == NULL || !cl_nv_open()) {
        return FALSE;
    }
    units = (length + 9UL) / 10UL;
    return StoreNV((STRPTR)app, (STRPTR)item, data, units, TRUE) == 0
        ? TRUE : FALSE;
}

BOOL cl_nv_delete(char *app, char *item)
{
    if (app == NULL || item == NULL || !cl_nv_open()) {
        return FALSE;
    }
    return DeleteNV((STRPTR)app, (STRPTR)item, TRUE) ? TRUE : FALSE;
}
