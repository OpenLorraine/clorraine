/* Bundle 9 optional network/hypertext suffix: bsdsocket and AmigaGuide. */

#include <libraries/bsdsocket.h>
#include <inline/bsdsocket.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <libraries/amigaguide.h>
#include <clib/amigaguide_protos.h>

struct Library *SocketBase;
struct Library *AmigaGuideBase;


/* ------------------------------------------------------------------------- */
/* IPv4 TCP sockets.                                                         */
/* ------------------------------------------------------------------------- */

typedef struct cl_socket {
    LONG descriptor;
} cl_socket;

static int cl_socket_open_library(void)
{
    if (SocketBase == NULL) {
        SocketBase = OpenLibrary((STRPTR)"bsdsocket.library", 4);
    }
    return SocketBase != NULL;
}

void *cl_socket_open(LONG kind, LONG protocol)
{
    cl_socket *handle;
    LONG descriptor;

    if (!cl_socket_open_library()) {
        return NULL;
    }
    descriptor = socket(AF_INET, kind, protocol);
    if (descriptor < 0) {
        return NULL;
    }
    handle = (cl_socket *)AllocMem((ULONG)sizeof *handle, MEMF_ANY | MEMF_CLEAR);
    if (handle == NULL) {
        (void)CloseSocket(descriptor);
        return NULL;
    }
    handle->descriptor = descriptor;
    return handle;
}

BOOL cl_socket_connect(void *handle_void, char *host, UWORD port)
{
    cl_socket *handle = (cl_socket *)handle_void;
    struct sockaddr_in address;
    in_addr_t address_value;

    if (handle == NULL || host == NULL) {
        return FALSE;
    }
    address_value = inet_addr((STRPTR)host);
    if (address_value == INADDR_NONE) {
        return FALSE;
    }
    memset(&address, 0, sizeof address);
    address.sin_len = (UBYTE)sizeof address;
    address.sin_family = AF_INET;
    address.sin_port = htons(port);
    address.sin_addr.s_addr = address_value;
    return connect(handle->descriptor, (struct sockaddr *)&address,
                   (socklen_t)sizeof address) == 0 ? TRUE : FALSE;
}

LONG cl_socket_send(void *handle_void, APTR data, ULONG length)
{
    cl_socket *handle = (cl_socket *)handle_void;

    if (handle == NULL || data == NULL) {
        return -1;
    }
    return send(handle->descriptor, data, (LONG)length, 0);
}

LONG cl_socket_recv(void *handle_void, APTR data, ULONG length)
{
    cl_socket *handle = (cl_socket *)handle_void;

    if (handle == NULL || data == NULL) {
        return -1;
    }
    return recv(handle->descriptor, data, (LONG)length, 0);
}

void cl_socket_close(void *handle_void)
{
    cl_socket *handle = (cl_socket *)handle_void;

    if (handle == NULL) {
        return;
    }
    if (handle->descriptor >= 0) {
        (void)CloseSocket(handle->descriptor);
    }
    FreeMem(handle, (ULONG)sizeof *handle);
}

/* ------------------------------------------------------------------------- */
/* AmigaGuide.                                                               */
/* ------------------------------------------------------------------------- */

typedef struct cl_guide {
    AMIGAGUIDECONTEXT context;
    struct NewAmigaGuide guide;
    char *path;
    char *screen;
} cl_guide;

typedef struct cl_guide_message {
    struct AmigaGuideMsg *message;
} cl_guide_message;

static int cl_guide_open_library(void)
{
    if (AmigaGuideBase == NULL) {
        AmigaGuideBase = OpenLibrary((STRPTR)"amigaguide.library", 37);
    }
    return AmigaGuideBase != NULL;
}

void *cl_guide_open(char *path, char *public_screen)
{
    cl_guide *guide;

    if (path == NULL || !cl_guide_open_library()) {
        return NULL;
    }
    guide = (cl_guide *)AllocMem((ULONG)sizeof *guide, MEMF_ANY | MEMF_CLEAR);
    if (guide == NULL) {
        return NULL;
    }
    guide->path = (char *)AllocMem((ULONG)strlen(path) + 1UL,
                                   MEMF_ANY | MEMF_CLEAR);
    if (guide->path == NULL) {
        FreeMem(guide, (ULONG)sizeof *guide);
        return NULL;
    }
    memcpy(guide->path, path, strlen(path) + 1UL);
    if (public_screen != NULL) {
        guide->screen = (char *)AllocMem((ULONG)strlen(public_screen) + 1UL,
                                         MEMF_ANY | MEMF_CLEAR);
        if (guide->screen == NULL) {
            FreeMem(guide->path, (ULONG)strlen(guide->path) + 1UL);
            FreeMem(guide, (ULONG)sizeof *guide);
            return NULL;
        }
        memcpy(guide->screen, public_screen, strlen(public_screen) + 1UL);
    }
    memset(&guide->guide, 0, sizeof guide->guide);
    guide->guide.nag_Name = (STRPTR)guide->path;
    guide->guide.nag_PubScreen = (STRPTR)guide->screen;
    guide->guide.nag_Flags = HTF_NOACTIVATE;
    guide->context = OpenAmigaGuideAsyncA(&guide->guide, NULL);
    if (guide->context == NULL) {
        if (guide->screen != NULL) {
            FreeMem(guide->screen, (ULONG)strlen(guide->screen) + 1UL);
        }
        FreeMem(guide->path, (ULONG)strlen(guide->path) + 1UL);
        FreeMem(guide, (ULONG)sizeof *guide);
        return NULL;
    }
    return guide;
}

void cl_guide_close(void *handle_void)
{
    cl_guide *guide = (cl_guide *)handle_void;

    if (guide == NULL) {
        return;
    }
    if (guide->context != NULL) {
        CloseAmigaGuide(guide->context);
    }
    if (guide->screen != NULL) {
        FreeMem(guide->screen, (ULONG)strlen(guide->screen) + 1UL);
    }
    if (guide->path != NULL) {
        FreeMem(guide->path, (ULONG)strlen(guide->path) + 1UL);
    }
    FreeMem(guide, (ULONG)sizeof *guide);
}

ULONG cl_guide_signal(void *handle_void)
{
    cl_guide *guide = (cl_guide *)handle_void;

    return guide == NULL || guide->context == NULL
        ? 0 : AmigaGuideSignal(guide->context);
}

void *cl_guide_wait(void *handle_void)
{
    cl_guide *guide = (cl_guide *)handle_void;
    cl_guide_message *message;
    struct AmigaGuideMsg *raw;
    ULONG signal_mask;

    if (guide == NULL || guide->context == NULL) {
        return NULL;
    }
    signal_mask = AmigaGuideSignal(guide->context);
    if (signal_mask == 0) {
        return NULL;
    }
    Wait(signal_mask);
    raw = GetAmigaGuideMsg(guide->context);
    if (raw == NULL) {
        return NULL;
    }
    message = (cl_guide_message *)AllocMem(
        (ULONG)sizeof *message, MEMF_ANY | MEMF_CLEAR);
    if (message == NULL) {
        ReplyAmigaGuideMsg(raw);
        return NULL;
    }
    message->message = raw;
    return message;
}

ULONG cl_guide_message_type(void *handle_void)
{
    const cl_guide_message *message = (const cl_guide_message *)handle_void;

    return message == NULL || message->message == NULL
        ? 0 : message->message->agm_Type;
}

ULONG cl_guide_message_primary(void *handle_void)
{
    const cl_guide_message *message = (const cl_guide_message *)handle_void;

    return message == NULL || message->message == NULL
        ? 0 : message->message->agm_Pri_Ret;
}

ULONG cl_guide_message_secondary(void *handle_void)
{
    const cl_guide_message *message = (const cl_guide_message *)handle_void;

    return message == NULL || message->message == NULL
        ? 0 : message->message->agm_Sec_Ret;
}

void cl_guide_reply(void *handle_void)
{
    cl_guide_message *message = (cl_guide_message *)handle_void;

    if (message == NULL) {
        return;
    }
    if (message->message != NULL) {
        ReplyAmigaGuideMsg(message->message);
    }
    FreeMem(message, (ULONG)sizeof *message);
}

BOOL cl_guide_send(void *handle_void, char *command)
{
    cl_guide *guide = (cl_guide *)handle_void;

    if (guide == NULL || guide->context == NULL || command == NULL) {
        return FALSE;
    }
    return SendAmigaGuideCmdA(guide->context, (STRPTR)command, NULL) == 0
        ? TRUE : FALSE;
}
