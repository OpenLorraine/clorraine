/*
 * Clorraine Phase 0 - hosted walking skeleton.
 *
 * Purpose: validate the hosted AmigaOS target model with no compiler yet.
 * This is the C that Clorraine will eventually emit, written by hand:
 *
 *   - every acquired resource is owned and released exactly once
 *   - releases are null-safe and run on every exit path
 *   - exec/dos calls follow the ownership contracts in ffi_contracts.h
 *   - the same source builds with GCC and vbcc in a strict common subset
 *
 * Two cleanup shapes are implemented so the findings document can pick the
 * canonical emitter pattern:
 *
 *   Shape A (single exit):    one "goto cleanup" label, releases in reverse
 *                             declaration order, null-safe drops.
 *   Shape B (scoped):         each resource is released at the end of the
 *                             scope that acquired it; no goto, deeper nesting.
 *
 * Three variants per shape exercise success and failure paths:
 *
 *   happy       all acquisitions succeed, the payload is written
 *   alloc-fail  AllocVec is asked for an absurd size and must fail
 *   open-fail   Open targets a directory that does not exist and must fail
 *
 * The program checks, for each variant, that the number of acquisitions
 * equals the number of releases and that the outcome is the expected one.
 * Any deviation is printed and makes the program exit with code 1.
 */

#include <exec/types.h>
#include <exec/memory.h>
#include <dos/dos.h>
#include <proto/exec.h>
#include <proto/dos.h>
#include <stdio.h>

#include "ffi_contracts.h"

static const char cl_payload[] = "phase0 payload\n";
#define CL_PAYLOAD_LEN ((LONG)(sizeof(cl_payload) - 1))

/* A size that AllocVec cannot satisfy on any Amiga; forces the failure path. */
#define CL_FAIL_ALLOC_SIZE 0x7FFFFFFFUL

#define CL_GOOD_PATH   (STRPTR) "RAM:phase0-a.txt"
#define CL_BAD_PATH    (STRPTR) "RAM:phase0-no-dir/x.txt"

/*
 * Shape A: single exit with reverse-order null-safe drops.
 *
 * Returns 1 when the observed behavior matches the expected one for the
 * requested failure variant, 0 otherwise.
 */
static int scenario_single_exit(int fail_alloc, int fail_open)
{
    struct Library *lib;
    BPTR file;
    APTR buf;
    LONG written;
    unsigned acq_before;
    unsigned rel_before;
    unsigned expected_count;
    int completed;

    lib = NULL;
    file = 0;
    buf = NULL;
    written = 0;
    completed = 0;

    acq_before = cl_acquisitions;
    rel_before = cl_releases;

    lib = cl_acquire_library((STRPTR) "dos.library", 36);
    if (lib == NULL) {
        goto cleanup;
    }

    file = cl_acquire_file(fail_open ? CL_BAD_PATH : CL_GOOD_PATH, MODE_NEWFILE);
    if (file == 0) {
        goto cleanup;
    }

    buf = cl_acquire_vec(fail_alloc ? CL_FAIL_ALLOC_SIZE : 64UL, MEMF_ANY);
    if (buf == NULL) {
        goto cleanup;
    }

    CopyMem((APTR) cl_payload, buf, (ULONG) CL_PAYLOAD_LEN);
    written = Write(file, (APTR) buf, CL_PAYLOAD_LEN);
    if (written == CL_PAYLOAD_LEN) {
        completed = 1;
    }

cleanup:
    cl_drop_vec(buf);
    cl_drop_file(file);
    cl_drop_library(lib);

    expected_count = fail_open ? 1U : (fail_alloc ? 2U : 3U);
    if ((cl_acquisitions - acq_before) != expected_count) {
        return 0;
    }
    if ((cl_releases - rel_before) != expected_count) {
        return 0;
    }
    return completed == (!fail_alloc && !fail_open);
}

/*
 * Shape B: scoped releases. Each resource is dropped at the end of the scope
 * that acquired it; no goto, but nesting grows with the acquisition chain.
 */
static int scenario_scoped(int fail_alloc, int fail_open)
{
    unsigned acq_before;
    unsigned rel_before;
    unsigned expected_count;
    int completed;

    completed = 0;
    acq_before = cl_acquisitions;
    rel_before = cl_releases;

    {
        struct Library *lib = cl_acquire_library((STRPTR) "dos.library", 36);
        if (lib != NULL) {
            BPTR file = cl_acquire_file(fail_open ? CL_BAD_PATH : CL_GOOD_PATH,
                                        MODE_NEWFILE);
            if (file != 0) {
                APTR buf = cl_acquire_vec(fail_alloc ? CL_FAIL_ALLOC_SIZE : 64UL,
                                          MEMF_ANY);
                if (buf != NULL) {
                    LONG written;
                    CopyMem((APTR) cl_payload, buf, (ULONG) CL_PAYLOAD_LEN);
                    written = Write(file, (APTR) buf, CL_PAYLOAD_LEN);
                    completed = (written == CL_PAYLOAD_LEN);
                    cl_drop_vec(buf);
                }
                cl_drop_file(file);
            }
            cl_drop_library(lib);
        }
    }

    expected_count = fail_open ? 1U : (fail_alloc ? 2U : 3U);
    if ((cl_acquisitions - acq_before) != expected_count) {
        return 0;
    }
    if ((cl_releases - rel_before) != expected_count) {
        return 0;
    }
    return completed == (!fail_alloc && !fail_open);
}

static void report(const char *shape, const char *variant, int ok)
{
    printf("phase0: %-6s %-10s %s\n", shape, variant, ok ? "ok" : "FAIL");
}

int main(void)
{
    int all_ok;
    int ok;

    cl_stats_reset();
    all_ok = 1;

    ok = scenario_single_exit(0, 0);
    report("single", "happy", ok);
    all_ok &= ok;

    ok = scenario_single_exit(1, 0);
    report("single", "alloc-fail", ok);
    all_ok &= ok;

    ok = scenario_single_exit(0, 1);
    report("single", "open-fail", ok);
    all_ok &= ok;

    ok = scenario_scoped(0, 0);
    report("scoped", "happy", ok);
    all_ok &= ok;

    ok = scenario_scoped(1, 0);
    report("scoped", "alloc-fail", ok);
    all_ok &= ok;

    ok = scenario_scoped(0, 1);
    report("scoped", "open-fail", ok);
    all_ok &= ok;

    printf("phase0: acquisitions=%u releases=%u\n",
           cl_acquisitions, cl_releases);

    if (cl_acquisitions != cl_releases) {
        all_ok = 0;
    }

    printf("phase0: %s\n", all_ok ? "all scenarios ok" : "FAILURES PRESENT");
    return all_ok ? 0 : 1;
}
