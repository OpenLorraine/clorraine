# Runtime Formatting: String Integer Formatting Throughput

> **Document**: 03-runtime-formatting.md
> **Parent**: [Index](00-index.md)

## Overview

This component owns the integer-to-decimal conversion used by:

- template placeholders (`$"{n}"`), through the compiler's `cl_text_*` call;
- explicit `text(n)` conversions;
- `std.fmt.toDecimal(n)`.

The change replaces the C library `snprintf("%d"/"%u")` path with a bounded
decimal writer that fills a freshly allocated `String` block in place. It
changes no signature visible to generated C: both converters keep
`String cl_text_i32(int)` / `String cl_text_u32(unsigned int)` on host and
`String cl_text_i32(LONG)` / `String cl_text_u32(ULONG)` on Amiga.

## Architecture

### Current

```
template / text / toDecimal
        │
        ▼
cl_text_u32(value)                  cl_text_i32(value)
        │                                   │
        ├─ snprintf(buf, 24, "%u")          ├─ snprintf(buf, 24, "%d")
        │                                   │
        └─ cl_string_from(buf, n)           └─ cl_string_from(buf, n)
                 │
                 ├─ cl_string_new(n)      one allocation
                 └─ cl_string_append(...) one copy
```

The library formatter (`vfprintf`, `putc`, libgcc division) dominates the
measured instruction count (AR-01).

### Proposed

```
cl_text_u32(value)                  cl_text_i32(value)
        │                                   │
        ├─ cl_string_new(10)                ├─ cl_string_new(11)
        ├─ cl_format_u32(payload, value)    ├─ cl_format_i32(payload, value)
        ├─ block->len = written             ├─ block->len = written
        └─ payload[written] = 0             └─ payload[written] = 0
```

One allocation, one write pass, no temporary buffer, no formatter call
(AR-02, AR-06).

## Implementation Details

### Width guard (both shims)

The width constants assume the modeled 32-bit `int`:

```c
#if INT_MAX != 2147483647 || UINT_MAX != 4294967295u
#error "the decimal formatter assumes a 32-bit int and unsigned int"
#endif
```

Both shims already include `<limits.h>`; a target where `int` is not 32 bits
fails the build instead of silently truncating (AR-09).

### Shared helpers (both shims)

```c
/* Write `value` as unsigned decimal digits at `dst`; return the count. */
static unsigned int cl_format_u32(unsigned char *dst, unsigned int value)
{
    unsigned char digits[10];
    unsigned int count = 0;
    do {
        digits[count++] = (unsigned char)('0' + (value % 10u));
        value /= 10u;
    } while (value != 0);
    unsigned int index;
    for (index = 0; index < count; index++) {
        dst[index] = digits[count - 1 - index];
    }
    return count;
}

/* Write a signed decimal at `dst`; return the count. */
static unsigned int cl_format_i32(unsigned char *dst, int value)
{
    unsigned int written = 0;
    unsigned int magnitude;
    if (value < 0) {
        dst[written++] = '-';
        magnitude = 0u - (unsigned int)value;
    } else {
        magnitude = (unsigned int)value;
    }
    return written + cl_format_u32(dst + written, magnitude);
}
```

**Why the unsigned magnitude:** converting `INT_MIN` to a positive `int` is
undefined. `0u - (unsigned int)value` produces the 32-bit two's-complement
magnitude `2147483648` without signed overflow, so `INT_MIN` prints
`-2147483648` exactly like `%d` (AR-03).

**Why the digit buffer:** decimal digits are produced least-significant first;
the short reverse copy places them in reading order. The buffer is at most 10
bytes, matching `UINT_MAX`.

### Host converters

```c
void *cl_text_i32(int value)
{
    cl_string_block *string = (cl_string_block *)cl_string_new(11);
    unsigned char *dst = cl_string_bytes(string);
    unsigned int written = cl_format_i32(dst, value);
    string->len = written;
    string->cap = written;
    dst[written] = '\0';
    return string;
}

void *cl_text_u32(unsigned int value)
{
    cl_string_block *string = (cl_string_block *)cl_string_new(10);
    unsigned char *dst = cl_string_bytes(string);
    unsigned int written = cl_format_u32(dst, value);
    string->len = written;
    string->cap = written;
    dst[written] = '\0';
    return string;
}
```

`cap` is set back to `written` after the in-place write, so the block's
logical capacity is exact and `cl_string_append` keeps its old
capacity-exceeded behavior. The physical allocation may have up to 7 spare
payload bytes; that slack is internal and never observable through the block
API (AR-10).

### Amiga converters

The same bodies with `LONG`/`ULONG` parameters and `ULONG` written counts.
The block fields are `ULONG` in that shim, so the assignment converts without
changing the value:

```c
void *cl_text_i32(LONG value)
{
    cl_string_block *string = (cl_string_block *)cl_string_new(11);
    unsigned char *dst = cl_string_bytes(string);
    ULONG written = (ULONG)cl_format_i32(dst, (int)value);
    string->len = written;
    string->cap = written;
    dst[written] = '\0';
    return string;
}

void *cl_text_u32(ULONG value)
{
    cl_string_block *string = (cl_string_block *)cl_string_new(10);
    unsigned char *dst = cl_string_bytes(string);
    ULONG written = (ULONG)cl_format_u32(dst, (unsigned int)value);
    string->len = written;
    string->cap = written;
    dst[written] = '\0';
    return string;
}
```

`LONG` and `int` are both 32-bit on the target, so the cast preserves the
value; the compiler already passes `(i32)`/`(u32)` spellings (AR-09).

### Capacity proof

| Converter | Maximum text | Capacity | Payload allocation |
|---|---:|---:|---:|
| `cl_text_i32` | `-2147483648` = 11 | 11 | `11 + 1` (NUL) |
| `cl_text_u32` | `4294967295` = 10 | 10 | `10 + 1` (NUL) |

`cl_string_new(cap)` allocates `sizeof(block) + cap + 1` and writes the
initial NUL (`shim_host.c:242`, `shim_hosted_amiga.c:863`). The converter
writes at most `cap` payload bytes at offset 0, sets `len` and `cap` to the
written count, and writes the NUL at `payload[len]`; both stay inside the
allocation. The physical block has up to 7 spare payload bytes for `i32` and 0
for `u32`; the logical `cap` is exact, so the block API behaves as before
(AR-10).

### Integration points

| Caller | Spelling | Effect |
|---|---|---|
| Template `{n}` | `cl_text_i32((i32)(n))` / `cl_text_u32((u32)(n))` | Same signature, faster body |
| `text(n)` | compiler-selected `cl_text_*` | Same |
| `std.fmt.toDecimal(n)` | `cl_text_i32(v)` | Same |
| `std.fmt.toHex(n)` | `cl_text_hex_u32(v)` | Unchanged |
| `$"{f}"` | `cl_text_f64(f)` | Unchanged (`%g`) |
| `$"{b}"` | `cl_text_bool(b)` | Unchanged |

The template builder still owns the temporary `String` and frees it after the
copy; that path is unchanged (AR-01).

## Code Examples

### Boundary output

```clorraine
import { println } from std.io;
import { toDecimal } from std.fmt;

func main(): i32 {
    let min: i32 = -2147483647 - 1;
    let max: u32 = 0xFFFFFFFF;
    println($"min={min} max={max} zero={0}");
    println(toDecimal(-42));
    return 0;
}
```

Expected:

```
min=-2147483648 max=4294967295 zero=0
-42
```

### Adjacent placeholders

```clorraine
println($"{a}{b}{c}");   // digits only, no separators inserted
```

## Error Handling

| Error case | Handling | AR Ref |
|---|---|---|
| Allocation failure | `cl_string_new` traps `error E1: out of memory`; no new branch | AR-04 |
| Capacity too large for the modeled size | `cl_string_new` traps `error E1: string too large` | AR-04 |
| `INT_MIN` magnitude | unsigned subtraction, no signed overflow | AR-03 |
| Block write bound | fixed maximum width, proven above | AR-10 |

There is no parsing, no user-controlled format string, and no new input
surface (AR-11).

## Testing Requirements

- Behavior suite: every ST case in `07-testing-strategy.md`.
- Implementation suite: the base fails the "no `snprintf` in the integer
  converter" pin; the head passes.
- Sanitizers: ASan with leak detection and UBSan with
  `-fno-sanitize-recover=all`.
- Cross-profile: host, hosted-amiga, hosted-amiga-13, vbcc, and freestanding
  build legs.
- Benchmark: `make bench-compare` with identical output hashes and the 1%
  guard.
