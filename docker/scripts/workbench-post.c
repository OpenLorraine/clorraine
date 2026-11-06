/* Test-only AppMessage fixture.
 *
 * Starts the Clorraine Workbench demo asynchronously, finds its public port,
 * posts the three application-object AppMessages (icon, window, menu), waits
 * for the replies, and prints `posted`. It is not part of the public SDK.
 */
#include <exec/types.h>
#include <exec/ports.h>
#include <dos/dos.h>
#include <dos/dostags.h>
#include <workbench/workbench.h>
#include <workbench/startup.h>
#include <utility/tagitem.h>
#include <clib/alib_protos.h>
#include <proto/exec.h>
#include <proto/dos.h>
#include <stdio.h>

static struct AppMessage *make_message(struct MsgPort *reply, UWORD type,
                                       ULONG id, ULONG user)
{
    struct AppMessage *message =
        (struct AppMessage *)AllocMem((ULONG)sizeof *message,
                                      MEMF_PUBLIC | MEMF_CLEAR);

    if (message == NULL) {
        return NULL;
    }
    message->am_Message.mn_Node.ln_Type = NT_MESSAGE;
    message->am_Message.mn_Length = (UWORD)sizeof *message;
    message->am_Message.mn_ReplyPort = reply;
    message->am_Type = type;
    message->am_ID = id;
    message->am_UserData = user;
    message->am_Class = 0;
    message->am_NumArgs = 0;
    message->am_Version = AM_VERSION;
    return message;
}

int main(void)
{
    struct MsgPort *port = NULL;
    struct MsgPort *reply;
    struct AppMessage *messages[3];
    LONG started;
    int index;

    started = SystemTags((STRPTR)"CNEXT:wb-demo", SYS_Asynch, TRUE, TAG_END);
    if (started == -1) {
        printf("error system\n");
        fflush(stdout);
        return 1;
    }
    for (index = 0; index < 500 && port == NULL; index++) {
        port = FindPort((STRPTR)"ClorraineWbTest");
        Delay(1);
    }
    if (port == NULL) {
        printf("error no port\n");
        fflush(stdout);
        return 1;
    }
    reply = CreatePort(NULL, 0);
    if (reply == NULL) {
        printf("error no reply\n");
        fflush(stdout);
        return 1;
    }
    messages[0] = make_message(reply, AMTYPE_APPICON, 7, 42);
    messages[1] = make_message(reply, AMTYPE_APPWINDOW, 8, 43);
    messages[2] = make_message(reply, AMTYPE_APPMENUITEM, 9, 44);
    if (messages[0] == NULL || messages[1] == NULL || messages[2] == NULL) {
        for (index = 0; index < 3; index++) {
            if (messages[index] != NULL) {
                FreeMem(messages[index], (ULONG)sizeof(struct AppMessage));
            }
        }
        DeletePort(reply);
        printf("error no message\n");
        fflush(stdout);
        return 1;
    }
    for (index = 0; index < 3; index++) {
        PutMsg(port, (struct Message *)messages[index]);
    }
    for (index = 0; index < 3; index++) {
        struct Message *replied;

        (void)WaitPort(reply);
        replied = GetMsg(reply);
        if (replied != NULL) {
            /* The demo replied; the fixture owns the storage. */
            FreeMem(replied, (ULONG)sizeof(struct AppMessage));
        }
    }
    DeletePort(reply);
    printf("posted\n");
    fflush(stdout);
    return 0;
}
