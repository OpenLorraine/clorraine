/*
 * Spike S1 fixture: how far does vbcc honor #line into usable debug and
 * diagnostic information?
 *
 * The file is hand-written to look like the C the Clorraine compiler will
 * emit for a small copy routine: an owned file handle and an owned buffer, a
 * single-exit cleanup ladder with null-safe drops, and a #line directive
 * directly before each emitted statement. The directives map the statements
 * back to a Clorraine source file, demo.clor, that does not exist on disk.
 *
 * Copying a payload exercises every part of the routine, so the normal build
 * is a meaningful program rather than a syntax husk. Compiling with -DERR
 * adds one statement that names an undefined symbol; the compiler then has to
 * report an error at a mapped location, which shows which file name and line
 * number its diagnostics use.
 */
#include <exec/types.h>
#include <exec/memory.h>
#include <dos/dos.h>
#include <proto/exec.h>
#include <proto/dos.h>
#include <stdio.h>

static const char payload[] = "spike-s1\n";
#define PAYLOAD_LEN ((LONG)(sizeof(payload) - 1))

/*
 * Single-exit cleanup ladder: one label, drops in reverse declaration order,
 * each guarded so that a resource that was never acquired is skipped. This is
 * the shape the compiler will emit, so the spike tests the real pattern.
 */
static int copy_via_ladder(void)
{
    BPTR file = 0;
    APTR buf = NULL;
    LONG written = 0;
    int ok = 0;

#line 20 "demo.clor"
    file = Open((STRPTR) "RAM:s1-demo.txt", MODE_NEWFILE);
#line 21 "demo.clor"
    if (file == 0) {
        goto cleanup;
    }
#line 24 "demo.clor"
    buf = AllocVec(64, MEMF_ANY);
#line 25 "demo.clor"
    if (buf == NULL) {
        goto cleanup;
    }
#line 28 "demo.clor"
    CopyMem((APTR) payload, buf, (ULONG) sizeof(payload));
#line 29 "demo.clor"
    written = Write(file, (APTR) buf, PAYLOAD_LEN);
#line 30 "demo.clor"
    ok = (written == PAYLOAD_LEN);

#ifdef ERR
#line 32 "demo.clor"
    ok = ok + undefined_spike_symbol;
#endif

cleanup:
#line 36 "demo.clor"
    if (buf != NULL) {
        FreeVec(buf);
    }
#line 37 "demo.clor"
    if (file != 0) {
        Close(file);
    }
#line 38 "demo.clor"
    return ok;
}

/*
 * A second mapped region in another function: it shows whether a new #line
 * sequence restarts cleanly, and whether debug commands can name a function
 * whose statements all come from the mapped file.
 */
int main(void)
{
    int ok;

#line 50 "demo.clor"
    ok = copy_via_ladder();
#line 51 "demo.clor"
    printf("spike-s1: %s\n", ok ? "ok" : "FAIL");
#line 52 "demo.clor"
    return ok ? 0 : 1;
}
