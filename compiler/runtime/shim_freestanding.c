/* Freestanding runtime shim for generated Clorraine C.
 *
 * The freestanding profile has no operating system and no C library: the
 * program is linked with `-nostdlib` and starts at `_start`, which this shim
 * defines. Three layers make that work:
 *
 *   1. A portable runtime: the byte helpers GCC injects, the console and trap
 *      helpers `std.io` binds, and a file-I/O stub that traps.
 *   2. A platform layer, chosen at compile time: real raw syscalls on x86_64
 *      and aarch64, documented no-op stubs on m68k until LorraineOS exists, and
 *      the host C runtime elsewhere.
 *   3. The entry point.
 *
 * The shim includes no header, so it uses only C built-in types. It is pushed
 * before the emitter's typedefs, so it cannot name `usize` or the ABI types.
 *
 * No helper here references a libc symbol on m68k: the m68k link additionally
 * reads the compiler's soft-float archive (`libg.a`), whose exit and allocator
 * objects reach the AmigaOS global `SysBase`; keeping the runtime clear of
 * `exit`/`abort`/`malloc` keeps that archive's other objects out of the link.
 */

/* The generated cast from a raw pointer to an integer names the standard
 * pointer-sized integer. The freestanding shim includes no header, so it
 * supplies the C99 spelling here; on m68k `unsigned long` is 32-bit, matching
 * the profile's `usize`. */
typedef unsigned long uintptr_t;

/* The freestanding shim includes no header, so the generated `offsetof`
 * spelling needs the C99 macro here. The member is never evaluated; the
 * address arithmetic is a compile-time constant. */
#define offsetof(type, member) ((unsigned long)&(((type *)0)->member))

/* --- portable runtime ---------------------------------------------------- */

/* The byte helpers GCC lowers aggregate copies and string literals to. They are
 * ordinary definitions, not `static`, because the injected calls resolve to
 * these external symbols; `-fno-builtin` on the command line stops the
 * optimizer from turning them into calls to themselves at `-O2`. */
void *memcpy(void *destination, const void *source, unsigned long count)
{
    unsigned char *to = (unsigned char *)destination;
    const unsigned char *from = (const unsigned char *)source;
    unsigned long index;
    for (index = 0; index < count; index++) {
        to[index] = from[index];
    }
    return destination;
}

void *memset(void *destination, int value, unsigned long count)
{
    unsigned char *to = (unsigned char *)destination;
    unsigned long index;
    for (index = 0; index < count; index++) {
        to[index] = (unsigned char)value;
    }
    return destination;
}

int memcmp(const void *left, const void *right, unsigned long count)
{
    const unsigned char *a = (const unsigned char *)left;
    const unsigned char *b = (const unsigned char *)right;
    unsigned long index;
    for (index = 0; index < count; index++) {
        if (a[index] != b[index]) {
            return a[index] < b[index] ? -1 : 1;
        }
    }
    return 0;
}

unsigned long strlen(const char *text)
{
    unsigned long length = 0;
    while (text[length] != '\0') {
        length++;
    }
    return length;
}

/* Zero one object through its address. A macro so a call site writes the single
 * argument `cl_zero(&value)`; `sizeof` supplies the byte count. */
#define cl_zero(pointer) memset((void *)(pointer), 0, sizeof(*(pointer)))

/* IEEE-754 NaN test on the raw bit pattern, copied from the host shim. The
 * exponent and mantissa are inspected directly because the comparison-based
 * tests are not IEEE for NaN on the soft-float targets. */
static inline int cl_isnan(double value)
{
    unsigned long long bits = 0;
    memcpy(&bits, &value, sizeof bits);
    if ((bits & 0x7ff0000000000000ULL) != 0x7ff0000000000000ULL) {
        return 0;
    }
    return (bits & 0x000fffffffffffffULL) != 0;
}

/* --- platform layer ------------------------------------------------------ */

#if defined(__m68k__)

/* m68k: there is no operating system yet. LorraineOS will provide the console
 * write and the halt; until then the console hook is a no-op and the exit
 * spins, which still lets the linker prove the artifact is self-contained. */
static inline void cl_write_bytes(int fd, const void *data, unsigned long count)
{
    (void)fd;
    (void)data;
    (void)count;
    /* LorraineOS will provide the console write. */
}

static inline void cl_exit(int code)
{
    (void)code;
    /* LorraineOS will provide the halt. */
    for (;;) {
    }
}

#elif defined(__x86_64__)

static inline void cl_write_bytes(int fd, const void *data, unsigned long count)
{
    long result;
    __asm__ volatile("syscall"
                     : "=a"(result)
                     : "a"(1L), "D"((long)fd), "S"(data), "d"(count)
                     : "rcx", "r11", "memory");
    (void)result;
}

static inline void cl_exit(int code)
{
    __asm__ volatile("syscall"
                     :
                     : "a"(60L), "D"((long)code)
                     : "rcx", "r11", "memory");
    for (;;) {
    }
}

#elif defined(__aarch64__)

static inline void cl_write_bytes(int fd, const void *data, unsigned long count)
{
    register long x0 __asm__("x0") = (long)fd;
    register const void *x1 __asm__("x1") = data;
    register unsigned long x2 __asm__("x2") = count;
    register long x8 __asm__("x8") = 64; /* __NR_write */
    __asm__ volatile("svc #0"
                     : "+r"(x0)
                     : "r"(x1), "r"(x2), "r"(x8)
                     : "memory");
}

static inline void cl_exit(int code)
{
    register long x0 __asm__("x0") = (long)code;
    register long x8 __asm__("x8") = 93; /* __NR_exit */
    __asm__ volatile("svc #0"
                     :
                     : "r"(x0), "r"(x8)
                     : "memory");
    for (;;) {
    }
}

#else

/* Any other host: the driver drops `-nostdlib` for this leg, so the platform C
 * runtime supplies the raw console write and exit. */
extern long write(int file, const void *data, unsigned long count);
extern void _exit(int code);

static inline void cl_write_bytes(int fd, const void *data, unsigned long count)
{
    (void)write(fd, data, count);
}

static inline void cl_exit(int code)
{
    _exit(code);
}

#endif

/* --- reports, traps, and console ----------------------------------------- */

/* Copy a NUL-terminated piece into a bounded buffer. */
static inline unsigned long cl_copy_into(char *buffer, unsigned long at, unsigned long limit, const char *text)
{
    while (*text != '\0' && at < limit) {
        buffer[at] = *text;
        at++;
        text++;
    }
    return at;
}

/* Write one whole message (`prefix`, then `code`, then `: message`) in a single
 * call, so a captured stream never interleaves two reports. */
static inline void cl_report(int fd, const char *prefix, const char *code, const char *message)
{
    char buffer[256];
    unsigned long limit = (unsigned long)sizeof buffer;
    unsigned long at = 0;
    at = cl_copy_into(buffer, at, limit, prefix);
    at = cl_copy_into(buffer, at, limit, code);
    if (code[0] != '\0') {
        at = cl_copy_into(buffer, at, limit, ": ");
    }
    at = cl_copy_into(buffer, at, limit, message);
    at = cl_copy_into(buffer, at, limit, "\n");
    cl_write_bytes(fd, buffer, at);
}

/* Abort with a runtime fault. The first line is exactly `error CODE: message`,
 * which is the fault contract the run tests assert. */
static inline void cl_trap(const char *code, const char *message)
{
    cl_report(2, "error ", code, message);
    cl_exit(1);
}

/* The byte length of a NUL-terminated C string. The pointer forms of a panic
 * message keep printing their text through the length-based runtime. */
static inline unsigned long cl_cstring_len(const char *text)
{
    return strlen(text);
}

/* Abort on a user-level panic. The message is passed as pointer and length, so
 * a view or handle needs no NUL terminator. The bytes are copied into the
 * fixed report buffer with room reserved for the newline, so a message longer
 * than the buffer is truncated while the prefix and the newline are always
 * written. */
static inline void cl_panic(const unsigned char *message, unsigned long len)
{
    char buffer[256];
    unsigned long limit = (unsigned long)sizeof buffer;
    unsigned long at = 0;
    unsigned long index;
    at = cl_copy_into(buffer, at, limit, "panic: ");
    for (index = 0; index < len && at + 1 < limit; index++) {
        buffer[at] = (char)message[index];
        at++;
    }
    buffer[at] = '\n';
    at++;
    cl_write_bytes(2, buffer, at);
    cl_exit(1);
}

/* Write `count` bytes to standard output without a trailing newline. */
void cl_write(void *data, unsigned int count)
{
    if (count > 0) {
        cl_write_bytes(1, data, (unsigned long)count);
    }
}

/* Write `count` bytes to standard output followed by a newline. */
void cl_write_line(void *data, unsigned int count)
{
    if (count > 0) {
        cl_write_bytes(1, data, (unsigned long)count);
    }
    cl_write_bytes(1, "\n", 1);
}

/* --- file I/O contract ---------------------------------------------------- */

/* The freestanding profile has no filesystem. The `std.io` file functions
 * compile so programs type-check, but each traps with the profile feature
 * code when it runs. The parameter types match the `std.io` ffi declarations. */
void *Open(char *path, int mode)
{
    (void)path;
    (void)mode;
    cl_trap("E11214", "file I/O is not available on the freestanding profile");
    return 0;
}

void Close(void *file)
{
    (void)file;
    cl_trap("E11214", "file I/O is not available on the freestanding profile");
}

int Read(void *file, void *buffer, int count)
{
    (void)file;
    (void)buffer;
    (void)count;
    cl_trap("E11214", "file I/O is not available on the freestanding profile");
    return 0;
}

int Write(void *file, void *buffer, int count)
{
    (void)file;
    (void)buffer;
    (void)count;
    cl_trap("E11214", "file I/O is not available on the freestanding profile");
    return 0;
}

/* --- entry ---------------------------------------------------------------- */

int main(void);

/* The entry the host's no-libc link starts at. The host ABI expects `main` to
 * be entered with a 16-byte-aligned call stack, but a raw entry does not
 * guarantee it, so a 16-byte-aligned local store inside `main` faults. The
 * attribute realigns the frame on entry there; a target whose placement is
 * already aligned (m68k's 2-byte one) needs nothing, so the attribute is
 * selected for x86-64 only. */
#if defined(__x86_64__)
__attribute__((force_align_arg_pointer))
#endif
void _start(void)
{
    cl_exit(main());
}
