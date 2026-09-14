/*
 * Clorraine Phase 0 - FFI ownership contracts (v0) and null-safe drop helpers.
 *
 * This header is hand-written on purpose: it is the seed of the runtime shim
 * and of the annotation format that the Clorraine compiler will grow from.
 * Every declaration below names the ownership contract for one AmigaOS
 * function, using the vocabulary the language spec will formalize:
 *
 *   returns-owns(Deleter)  caller becomes the owner; nullable result
 *   consumes               the call releases the resource; no-op on NULL
 *   borrows / borrows-mut  callee uses the resource but never releases it
 *   value                  no ownership involved
 *
 * Contract table for the functions used by this skeleton:
 *
 *   OpenLibrary(name, ver)      returns-owns(CloseLibrary), nullable
 *   CloseLibrary(base)          consumes
 *   AllocVec(size, flags)       returns-owns(FreeVec), nullable
 *   FreeVec(mem)                consumes
 *   Open(name, mode)            returns-owns(Close), nullable (BPTR)
 *   Close(file)                 consumes
 *   Write(file, buf, len)       borrows(buf, len bytes), value result
 *
 * Later phases add CreateMsgPort/DeleteMsgPort, CreateIORequest/
 * DeleteIORequest, pools, and the message-passing ownership transfers
 * (GetMsg/PutMsg/ReplyMsg), which are deliberately out of scope here.
 */
#ifndef CLORRAINE_FFI_CONTRACTS_H
#define CLORRAINE_FFI_CONTRACTS_H

#include <exec/types.h>
#include <exec/memory.h>
#include <dos/dos.h>
#include <proto/exec.h>
#include <proto/dos.h>

/*
 * Release counters. The skeleton asserts that every acquisition is released
 * exactly once, which is the property the borrow checker will eventually
 * prove statically. Counting here turns silent leaks into test failures.
 */
static unsigned cl_acquisitions;
static unsigned cl_releases;

static void cl_stats_reset(void)
{
    cl_acquisitions = 0;
    cl_releases = 0;
}

/* OpenLibrary: returns-owns(CloseLibrary), nullable. */
static struct Library *cl_acquire_library(STRPTR name, ULONG version)
{
    struct Library *lib = OpenLibrary(name, version);
    if (lib != NULL) {
        cl_acquisitions++;
    }
    return lib;
}

/* CloseLibrary: consumes; null-safe. */
static void cl_drop_library(struct Library *lib)
{
    if (lib != NULL) {
        CloseLibrary(lib);
        cl_releases++;
    }
}

/* Open: returns-owns(Close), nullable. */
static BPTR cl_acquire_file(STRPTR name, LONG mode)
{
    BPTR file = Open(name, mode);
    if (file != 0) {
        cl_acquisitions++;
    }
    return file;
}

/* Close: consumes; null-safe (0 is the AmigaDOS invalid handle). */
static void cl_drop_file(BPTR file)
{
    if (file != 0) {
        Close(file);
        cl_releases++;
    }
}

/* AllocVec: returns-owns(FreeVec), nullable. */
static APTR cl_acquire_vec(ULONG size, ULONG flags)
{
    APTR mem = AllocVec(size, flags);
    if (mem != NULL) {
        cl_acquisitions++;
    }
    return mem;
}

/* FreeVec: consumes; null-safe. */
static void cl_drop_vec(APTR mem)
{
    if (mem != NULL) {
        FreeVec(mem);
        cl_releases++;
    }
}

#endif /* CLORRAINE_FFI_CONTRACTS_H */
