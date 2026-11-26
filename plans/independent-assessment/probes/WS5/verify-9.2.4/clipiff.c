/* Writes clipboard unit 0 with the shim's exact sequence (io_ClipID 0,
 * CMD_WRITE, CMD_UPDATE): argv[1] "raw" writes 4 raw bytes, "iff" writes an
 * IFF FORM FTXT CHRS clip holding the same 4 bytes. */
#include <stdio.h>
#include <string.h>
#include <exec/memory.h>
#include <exec/io.h>
#include <devices/clipboard.h>
#include <proto/exec.h>
#include <clib/alib_protos.h>

static const UBYTE iff[24] = {
    'F', 'O', 'R', 'M', 0, 0, 0, 16, 'F', 'T', 'X', 'T',
    'C', 'H', 'R', 'S', 0, 0, 0, 4, 'w', 'x', 'y', 'z'
};
static const UBYTE raw[4] = { 'w', 'x', 'y', 'z' };

int main(int argc, char **argv)
{
    struct MsgPort *port = CreatePort(NULL, 0);
    struct IOClipReq *req;
    int use_iff = argc > 1 && strcmp(argv[1], "iff") == 0;
    BYTE e1, e2;

    if (port == NULL) {
        return 5;
    }
    req = (struct IOClipReq *)CreateExtIO(port, sizeof *req);
    if (req == NULL || OpenDevice((STRPTR)"clipboard.device", 0, (struct IORequest *)req, 0) != 0) {
        printf("w: no device\n");
        return 5;
    }
    req->io_ClipID = 0;
    req->io_Command = CMD_WRITE;
    req->io_Data = (STRPTR)(use_iff ? iff : raw);
    req->io_Length = use_iff ? sizeof iff : sizeof raw;
    req->io_Offset = 0;
    e1 = DoIO((struct IORequest *)req);
    req->io_Command = CMD_UPDATE;
    e2 = DoIO((struct IORequest *)req);
    printf("w %s: write %d update %d id %ld\n", use_iff ? "iff" : "raw", (int)e1, (int)e2, (long)req->io_ClipID);
    CloseDevice((struct IORequest *)req);
    DeleteExtIO((struct IORequest *)req);
    DeletePort(port);
    return 0;
}
