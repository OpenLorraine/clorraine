/* Bundle 9 optional network/hypertext suffix: bsdsocket and AmigaGuide. */

#include <libraries/amigaguide.h>
#include <clib/amigaguide_protos.h>

struct Library *AmigaGuideBase;

#ifndef __VBCC__
/* vbcc cannot parse the NDK bsdsocket headers, so the socket half is built
 * only with the m68k GCC toolchain. AmigaGuide stays toolchain-neutral. */
#include <libraries/bsdsocket.h>
#include <inline/bsdsocket.h>
#include <netinet/in.h>
#include <sys/socket.h>

struct Library *SocketBase;


/* ------------------------------------------------------------------------- */
/* IPv4 TCP sockets.                                                         */
/* ------------------------------------------------------------------------- */

typedef struct cl_socket {
    LONG descriptor;
} cl_socket;

static LONG __attribute__((noinline, optimize("O0")))
cl_socket_send_raw(LONG sock, APTR data, LONG length)
{
    return send(sock, data, length, 0);
}

static LONG __attribute__((noinline, optimize("O0")))
cl_socket_recv_raw(LONG sock, APTR data, LONG length)
{
    return recv(sock, data, length, 0);
}

static LONG __attribute__((noinline, optimize("O0")))
cl_socket_connect_raw(LONG sock, struct sockaddr *address, socklen_t length)
{
    return connect(sock, address, length);
}

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
    /* The emulated library may retain the address for the asynchronous
     * host connect, so keep it alive beyond this call. */
    static struct sockaddr_in address;
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
    {
        LONG descriptor = handle->descriptor;
        LONG ret = cl_socket_connect_raw(descriptor,
                                         (struct sockaddr *)&address,
                                         (socklen_t)sizeof address);
        return ret == 0 ? TRUE : FALSE;
    }
}

LONG cl_socket_send(void *handle_void, APTR data, ULONG length)
{
    cl_socket *handle = (cl_socket *)handle_void;

    if (handle == NULL || data == NULL) {
        return -1;
    }
    {
        LONG descriptor = handle->descriptor;
        return cl_socket_send_raw(descriptor, data, (LONG)length);
    }
}

LONG cl_socket_recv(void *handle_void, APTR data, ULONG length)
{
    cl_socket *handle = (cl_socket *)handle_void;

    if (handle == NULL || data == NULL) {
        return -1;
    }
    {
        LONG descriptor = handle->descriptor;
        return cl_socket_recv_raw(descriptor, data, (LONG)length);
    }
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

#endif /* !__VBCC__ */

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

#ifndef __VBCC__
/* One-shot TCP echo used by the acceptance gate. Keeping the socket,
 * connect, send, and receive in one function matches the proven C probe;
 * the typed split API remains available for applications. */
LONG cl_net_echo(char *host, UWORD port, APTR data, ULONG length,
                 APTR out, ULONG outlen)
{
    struct sockaddr_in address;
    in_addr_t address_value;
    LONG descriptor;
    LONG sent;
    LONG received;

    if (host == NULL || data == NULL || out == NULL ||
        !cl_socket_open_library()) {
        return -1;
    }
    descriptor = socket(AF_INET, SOCK_STREAM, 0);
    if (descriptor < 0) {
        return -1;
    }
    address_value = inet_addr((STRPTR)host);
    if (address_value == INADDR_NONE) {
        (void)CloseSocket(descriptor);
        return -1;
    }
    memset(&address, 0, sizeof address);
    address.sin_len = (UBYTE)sizeof address;
    address.sin_family = AF_INET;
    address.sin_port = htons(port);
    address.sin_addr.s_addr = address_value;
    if (connect(descriptor, (struct sockaddr *)&address,
                (socklen_t)sizeof address) != 0) {
        (void)CloseSocket(descriptor);
        return -1;
    }
    sent = send(descriptor, data, (LONG)length, 0);
    if (sent <= 0) {
        (void)CloseSocket(descriptor);
        return -1;
    }
    received = recv(descriptor, out, (LONG)outlen, 0);
    (void)CloseSocket(descriptor);
    return received;
}

#endif /* !__VBCC__ */
