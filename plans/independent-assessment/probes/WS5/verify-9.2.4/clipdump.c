/* Reads clipboard unit 0 the autodoc way (io_ClipID 0, offset 0, then a
 * read past the end) and prints the byte count and the first bytes. */
#include <stdio.h>
#include <exec/memory.h>
#include <exec/io.h>
#include <devices/clipboard.h>
#include <proto/exec.h>
#include <clib/alib_protos.h>

int main(void)
{
    struct MsgPort *port = CreatePort(NULL, 0);
    struct IOClipReq *req;
    UBYTE buf[65];
    LONG got;
    int i;

    if (port == NULL) {
        printf("dump: no port\n");
        return 5;
    }
    req = (struct IOClipReq *)CreateExtIO(port, sizeof *req);
    if (req == NULL || OpenDevice((STRPTR)"clipboard.device", 0, (struct IORequest *)req, 0) != 0) {
        printf("dump: no device\n");
        return 5;
    }
    req->io_Command = CMD_READ;
    req->io_Data = (STRPTR)buf;
    req->io_Length = 64;
    req->io_Offset = 0;
    req->io_ClipID = 0;
    DoIO((struct IORequest *)req);
    got = (LONG)req->io_Actual;
    printf("dump: err %d actual %ld id %ld bytes", (int)req->io_Error, (long)got, (long)req->io_ClipID);
    for (i = 0; i < got && i < 8; i++) {
        printf(" %d", buf[i]);
    }
    printf("\n");
    while (req->io_Actual != 0) {
        req->io_Command = CMD_READ;
        req->io_Data = NULL;
        req->io_Length = 0x7fffffff;
        DoIO((struct IORequest *)req);
    }
    CloseDevice((struct IORequest *)req);
    DeleteExtIO((struct IORequest *)req);
    DeletePort(port);
    return 0;
}
