/*
 * Spike S2 fixture: the C prototypes the compiler emits for FFI declarations,
 * compiled and run against the real NDK.
 *
 * An FFI declaration is ABI-typed: the parameter modes (take, borrow, const)
 * never change the C type. The emitted prototype therefore uses the target's
 * real type - BPTR for a File handle, APTR for a buffer view, struct Library *
 * for the exec base - while the Clorraine-side binding keeps an opaque handle
 * typedef. The two meet at one explicit cast on the call boundary, which this
 * fixture also proves is a no-op on m68k because every handle is 32 bits.
 *
 * The program copies a payload to a RAM: file, reads it back, and releases
 * every acquisition on every path. Release counters turn a leak or a double
 * release into a test failure. A view struct and a small native pair are
 * passed by value to check the calling convention.
 */
#include <exec/types.h>
#include <exec/memory.h>
#include <dos/dos.h>
#include <stdio.h>

/* Fixed-width prelude names: the compiler emits these, so the fixture does too. */
typedef unsigned char ClU8;
typedef unsigned long ClU32;

/* Opaque Clorraine-side handles: 32-bit pointers, NULL is the empty state. */
typedef struct ClFileClosure    *ClFile;
typedef struct ClLibraryClosure *ClLibrary;

/* A str-like view: a pointer plus a separate length, copied by value. */
typedef struct { const ClU8 *ptr; ClU32 len; } ClStr;

/* A native struct passed by value. */
typedef struct { LONG a; LONG b; } ClPair;

/*
 * FFI ABI prototypes exactly as the emitter writes them. No proto header is
 * included, so these are the only declarations and the linker resolves them
 * from the stub archive. Close is declared void although the NDK returns BOOL:
 * the compiler discards the result, which is ABI-safe on m68k.
 */
struct Library;
extern struct Library *OpenLibrary(const char *name, ULONG version);
extern void            CloseLibrary(struct Library *base);
extern BPTR            Open(const char *name, LONG mode);
extern LONG            Read(BPTR f, APTR buf, LONG len);
extern LONG            Write(BPTR f, APTR buf, LONG len);
extern void            Close(BPTR f);
extern APTR            AllocVec(ULONG size, ULONG flags);
extern void            FreeVec(APTR mem);

static const char payload[] = "spike-s2 payload\n";
#define PAYLOAD_LEN ((LONG)(sizeof(payload) - 1))

static unsigned acquisitions;
static unsigned releases;

/* Null-safe owned acquisitions: only a real result counts as an acquisition. */
static struct Library *acquire_library(const char *name, ULONG version)
{
    struct Library *lib = OpenLibrary(name, version);
    if (lib != NULL) {
        acquisitions++;
    }
    return lib;
}

static void drop_library(struct Library *lib)
{
    if (lib != NULL) {
        CloseLibrary(lib);
        releases++;
    }
}

static BPTR acquire_file(const char *name, LONG mode)
{
    BPTR f = Open(name, mode);
    if (f != 0) {
        acquisitions++;
    }
    return f;
}

static void drop_file(BPTR f)
{
    if (f != 0) {
        Close(f);
        releases++;
    }
}

static APTR acquire_vec(ULONG size)
{
    APTR mem = AllocVec(size, MEMF_ANY);
    if (mem != NULL) {
        acquisitions++;
    }
    return mem;
}

static void drop_vec(APTR mem)
{
    if (mem != NULL) {
        FreeVec(mem);
        releases++;
    }
}

/*
 * The emitted view wrappers: the view crosses by value, and the ABI call
 * receives the pointer and the length as separate arguments.
 */
static LONG write_view(BPTR f, ClStr view)
{
    return Write(f, (APTR)view.ptr, (LONG)view.len);
}

static LONG read_view(BPTR f, ClStr view)
{
    return Read(f, (APTR)view.ptr, (LONG)view.len);
}

/* A native struct passed and consumed by value. */
static LONG pair_sum(ClPair p)
{
    return p.a + p.b;
}

/*
 * Write the payload, reopen the file for reading, and compare. The cleanup
 * ladder drops every acquired resource in reverse order, null-safely.
 */
static int copy_round_trip(void)
{
    struct Library *lib = NULL;
    BPTR file = 0;
    APTR buf = NULL;
    ClStr out;
    ClStr in;
    LONG wrote = 0;
    LONG got = 0;
    int ok = 0;

    out.ptr = (const ClU8 *)payload;
    out.len = (ClU32)PAYLOAD_LEN;

    lib = acquire_library("dos.library", 36);
    if (lib == NULL) {
        goto cleanup;
    }
    /* Handle width: the opaque handle and the ABI pointer are interchangeable. */
    if ((struct Library *)(ClLibrary)lib != lib) {
        goto cleanup;
    }

    file = acquire_file("RAM:s2-abi.txt", MODE_NEWFILE);
    if (file == 0) {
        goto cleanup;
    }
    if ((BPTR)(ClFile)file != file) {
        goto cleanup;
    }

    buf = acquire_vec(64);
    if (buf == NULL) {
        goto cleanup;
    }

    wrote = write_view(file, out);
    drop_file(file);
    file = 0;

    if (wrote != PAYLOAD_LEN) {
        goto cleanup;
    }

    file = acquire_file("RAM:s2-abi.txt", MODE_OLDFILE);
    if (file == 0) {
        goto cleanup;
    }

    in.ptr = (const ClU8 *)buf;
    in.len = 64;
    got = read_view(file, in);

    if (got == PAYLOAD_LEN) {
        const char *want = payload;
        const ClU8 *have = (const ClU8 *)buf;
        LONG i;
        ok = 1;
        for (i = 0; i < PAYLOAD_LEN; i++) {
            if ((ClU8)want[i] != have[i]) {
                ok = 0;
            }
        }
    }

cleanup:
    drop_file(file);
    drop_vec(buf);
    drop_library(lib);
    return ok;
}

int main(void)
{
    ClPair pair;
    LONG pair_result;
    int round_trip;
    int all_ok;

    pair.a = 3;
    pair.b = 4;
    pair_result = pair_sum(pair);

    round_trip = copy_round_trip();

    printf("spike-s2: widths BPTR=%lu APTR=%lu ClFile=%lu ClLibrary=%lu\n",
           (unsigned long)sizeof(BPTR), (unsigned long)sizeof(APTR),
           (unsigned long)sizeof(ClFile), (unsigned long)sizeof(ClLibrary));
    printf("spike-s2: struct-by-value pair_sum=%ld\n", (long)pair_result);
    printf("spike-s2: round-trip=%s\n", round_trip ? "ok" : "FAIL");
    printf("spike-s2: acquisitions=%u releases=%u\n", acquisitions, releases);

    all_ok = round_trip && (pair_result == 7) && (acquisitions == releases);
    if (!all_ok) {
        printf("spike-s2: FAILURES PRESENT\n");
        return 1;
    }
    printf("spike-s2: all ok\n");
    return 0;
}
