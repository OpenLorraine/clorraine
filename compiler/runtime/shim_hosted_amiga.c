/* Hosted-Amiga runtime shim for generated Clorraine C.
 *
 * This shim has the same surface as the host shim but binds to the AmigaOS
 * exec and dos libraries. It carries no ownership logic: it only names the
 * helpers the emitted translation unit calls and the C-side VBlank server the
 * standard-library interrupt dispatcher installs. Every helper is
 * `static inline` or a macro, so a program that uses only some of them still
 * compiles clean under `-Wall -Wextra -Werror`.
 */

#include <exec/types.h>
#include <exec/interrupts.h>
#include <hardware/intbits.h>
#include <proto/exec.h>
/* `dos/dos.h` declares the `BPTR` file-handle type the ABI mapping uses; the
 * generated code emits no AmigaOS typedef of its own, so the shim must supply
 * it. `stdbool.h` supplies the C99 `bool` the Clorraine boolean lowers to. */
#include <dos/dos.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Zero one object through its address. A macro so a call site writes the
 * single argument `cl_zero(&value)`; `sizeof` supplies the byte count. */
#define cl_zero(pointer) memset((void *)(pointer), 0, sizeof(*(pointer)))

/* IEEE-754 NaN test on the raw bit pattern.
 *
 * The platform soft-float comparisons are not IEEE for NaN, so the exponent
 * and mantissa are inspected directly. A `double` parameter accepts a `float`
 * argument through the usual promotion. */
static inline int cl_isnan(double value)
{
    uint64_t bits = 0;
    memcpy(&bits, &value, sizeof bits);
    if ((bits & UINT64_C(0x7ff0000000000000)) != UINT64_C(0x7ff0000000000000)) {
        return 0;
    }
    return (bits & UINT64_C(0x000fffffffffffff)) != 0;
}

/* Leave the process after a runtime fault. The Amiga fault contract writes the
 * first line `error CODE: message` to stdout and then exits, so the run-fault
 * leg reads it from the program's standard output. */
static inline void cl_trap(const char *code, const char *message)
{
    printf("error %s: %s\n", code, message);
    fflush(stdout);
    exit(20);
}

/* Leave the process after a user-level panic. */
static inline void cl_panic(const char *message)
{
    printf("panic: %s\n", message);
    fflush(stdout);
    exit(20);
}

/* The C-side VBlank dispatcher surface.
 *
 * The standard-library interrupt dispatcher registers a capture-less Clorraine
 * function here. A handler is a plain function pointer, and the server
 * trampoline calls it and returns, so the handler stays allocation-free,
 * drop-free, and panic-free as the interrupt contract requires. */
typedef void (*cl_handler_fn)(void);

static cl_handler_fn cl_vblank_handler;
static struct Interrupt cl_vblank_server;

static void cl_vblank_code(void)
{
    if (cl_vblank_handler != NULL) {
        cl_vblank_handler();
    }
}

/* Installs `handler` as the VBlank server, replacing any current one.
 *
 * A null handler is rejected so an accidental unregister cannot leave a
 * dangling callback. */
static inline int cl_add_handler(cl_handler_fn handler)
{
    if (handler == NULL) {
        return 0;
    }
    if (cl_vblank_handler != NULL) {
        RemIntServer(INTB_VERTB, &cl_vblank_server);
    }
    cl_vblank_handler = handler;
    cl_vblank_server.is_Node.ln_Type = NT_INTERRUPT;
    cl_vblank_server.is_Node.ln_Pri = 0;
    cl_vblank_server.is_Node.ln_Name = (char *)"clorraine.vblank";
    cl_vblank_server.is_Data = NULL;
    cl_vblank_server.is_Code = (VOID (*)())cl_vblank_code;
    AddIntServer(INTB_VERTB, &cl_vblank_server);
    return 1;
}

/* Removes the VBlank server if one is installed. */
static inline void cl_remove_handler(void)
{
    if (cl_vblank_handler != NULL) {
        RemIntServer(INTB_VERTB, &cl_vblank_server);
        cl_vblank_handler = NULL;
    }
}
