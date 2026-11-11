# Template Lowering: Direct Template Formatting

> **Document**: 03-template-lowering.md
> **Parent**: [Index](00-index.md)

## Overview

The template keeps its shape: one `cl_string_concat` call over a compound
literal of `ClStrPart` pieces, one owned result. The change is what each piece
carries and what the builder does with it.

## Part Representation

Both shims replace the 12-byte `{ptr, len, owned}` part with a 12-byte union
part:

```c
/* Kind codes for a scalar piece: `len` holds the code and `ptr` is NULL.
 * The codes start at 2 so a zero/NULL view (ptr == NULL, len == 0) is never
 * read as a scalar. */
#define CL_STR_KIND_I32 2u
#define CL_STR_KIND_U32 3u

typedef struct cl_str_part {
    const unsigned char *ptr;
    unsigned int len;
    union {
        void *owned;
        int i;
        unsigned int u;
    } value;
} ClStrPart;
```

| Piece | `ptr` | `len` | `value` |
|---|---|---|---|
| Literal | static bytes | byte count | `.owned = NULL` |
| `str`/`static str` | copy payload | copy length | `.owned = handle` |
| `String` | copy payload | copy length | `.owned = handle` |
| `i8`…`isize` | `NULL` | `CL_STR_KIND_I32` (2) | `.i` |
| `u8`…`usize` | `NULL` | `CL_STR_KIND_U32` (3) | `.u` |
| `bool` | static `"true"`/`"false"` | 4/5 | `.owned = NULL` |
| `f32`/`f64` | `%g` payload | byte count | `.owned = handle` |

The size stays 12 bytes, so the compound literal's stack footprint does not
grow (AR-02).

## Helpers (both shims)

```c
static ClStrPart cl_str_part_owned(void *string)
{
    ClStrPart part;
    part.ptr = (const unsigned char *)cl_string_ptr(string);
    part.len = cl_string_len(string);
    part.value.owned = string;
    return part;
}

static ClStrPart cl_str_part_bytes(const unsigned char *ptr, unsigned int len)
{
    ClStrPart part;
    part.ptr = ptr;
    part.len = len;
    part.value.owned = NULL;
    return part;
}

static ClStrPart cl_str_part_i32(int value)
{
    ClStrPart part;
    part.ptr = NULL;
    part.len = CL_STR_KIND_I32;
    part.value.i = value;
    return part;
}

static ClStrPart cl_str_part_u32(unsigned int value)
{
    ClStrPart part;
    part.ptr = NULL;
    part.len = CL_STR_KIND_U32;
    part.value.u = value;
    return part;
}

static ClStrPart cl_str_part_bool(int value)
{
    static const unsigned char truth[5] = "true";
    static const unsigned char falsehood[6] = "false";
    ClStrPart part;
    part.ptr = value ? truth : falsehood;
    part.len = value ? 4u : 5u;
    part.value.owned = NULL;
    return part;
}

void *cl_text_f64(double value);   /* defined below */

static ClStrPart cl_str_part_f64(double value)
{
    return cl_str_part_owned(cl_text_f64(value));
}
```

`str` and `String` placeholders are copied at part construction
(`cl_str_part_owned(cl_text_str(...))` / `cl_str_part_owned(cl_string_copy(...))`),
so a sibling placeholder cannot release the source before the builder reads it
(AR-03, AR-13). `cl_str_part_f64`
keeps the documented `%g` text and its one temporary (AR-04).

## Builder

The decimal helpers move above the builder so it can call them. The first pass
computes the maximum capacity; the second writes and frees owned pieces:

```c
void *cl_string_concat(const ClStrPart *parts, unsigned int count)
{
    unsigned int index;
    unsigned int total = 0;
    for (index = 0; index < count; index++) {
        unsigned int extra = parts[index].len;
        if (parts[index].ptr == NULL && parts[index].len >= CL_STR_KIND_I32) {
            extra = parts[index].len == CL_STR_KIND_I32 ? 11u : 10u;
        }
        unsigned int next = total + extra;
        if (next < total) {
            cl_trap("E1", "string too large");
        }
        total = next;
    }
    cl_string_block *string = (cl_string_block *)cl_string_new(total);
    unsigned char *dst = cl_string_bytes(string);
    unsigned int len = 0;
    for (index = 0; index < count; index++) {
        const ClStrPart *part = &parts[index];
        if (part->ptr == NULL && part->len >= CL_STR_KIND_I32) {
            if (part->len == CL_STR_KIND_I32) {
                len += cl_format_i32(dst + len, part->value.i);
                continue;
            }
            len += cl_format_u32(dst + len, part->value.u);
            continue;
        }
        if (part->len != 0) {
            memcpy(dst + len, part->ptr, (size_t)part->len);
        }
        len += part->len;
        if (part->value.owned != NULL) {
            cl_free(part->value.owned);
        }
    }
    string->len = len;
    string->cap = len;
    dst[len] = '\0';
    return string;
}
```

**Capacity proof:** a scalar i32 reserves 11, a u32 reserves 10, a byte piece
reserves its exact length. The writer never exceeds the reserved range; the
NUL is at `dst[len]` with `len <= total`, inside the `total + 1` payload
allocation. After the write, `cap == len`, preserving the append trap
(AR-05).

**No `} else {` or `do { `:** the loops are `while` and the scalar branch uses
`continue`. This is ordinary style, and it keeps two whole-translation-unit
emission assertions (which search for those spellings) scoped to the program
body.

## Emitted Shape

```c
s = cl_string_concat((ClStrPart[]){
    cl_str_part_bytes((const u8 *)"token-", 6u),
    cl_str_part_u32((u32)(i)),
    cl_str_part_bytes((const u8 *)"-", 1u),
    cl_str_part_u32((u32)((i * (u32)(3))))
}, 4u);
```

A `String` placeholder: `cl_str_part_owned(cl_string_copy(s))`.
A `str` placeholder: `cl_str_part_owned(cl_text_str((v).ptr, (v).len))`.
A bool placeholder: `cl_str_part_bool((int)(b))`.
A float placeholder: `cl_str_part_f64((double)(f))`.

No program-body designated initializer is emitted; the union and kind are set
by the shim helpers (AR-13, RV-002).
A literal-only template stays `cl_str("...")` with no builder call.

## Codegen Change

`ExprEmitter::template` (`compiler/src/codegen/expr.rs:1616`) keeps the
no-placeholder path. For placeholders it matches the substituted type:

| Type | Emitted part |
|---|---|
| `Str`, `StaticStr` | `cl_str_part_owned(cl_text_str(...))` |
| `Handle("String")` | `cl_str_part_owned(cl_string_copy(<value>))` |
| scalar i8/i16/i32/isize | `cl_str_part_i32((i32)(<value>))` |
| scalar u8/u16/u32/usize | `cl_str_part_u32((u32)(<value>))` |
| scalar bool | `cl_str_part_bool((int)(<value>))` |
| scalar f32/f64 | `cl_str_part_f64((double)(<value>))` |
| any other handled type | `cl_str_part_owned(<text_call>)` |
| literal | `cl_str_part_bytes((const u8 *)"...", Nu)` |

`text_call` and the explicit `text`/`toDecimal` converters are unchanged.

## Error Handling

| Case | Handling | AR Ref |
|---|---|---|
| Capacity sum overflow | `cl_trap("E1", "string too large")` in the first pass | AR-05 |
| Allocation failure | `cl_string_new` traps `E1` | AR-05 |
| Wrong union member | kind code selects the member; tests cover both | AR-02, AR-07 |
| Borrowed source lifetime | full-expression lifetime; ASan/ownership probes | AR-03 |

## Testing Requirements

- Output equality for every formattable type and boundary.
- Allocation delta: two integer placeholders = 1 result allocation; base = 3.
- `String`/`str` copy at part construction; owned source reusable.
- Float = 2 allocations (temp + result).
- ASan/UBSan.
- Emission pins and the base/head C diff.
