---
title: Hello, Amiga
description: "The smallest complete Clorraine program: source, host and Amiga commands, output, and a generated C excerpt."
---

# Hello, Amiga

This is the smallest complete Clorraine program. It prints one templated line
and exits with status 0.

## Source

```clorraine
module demo.Hello;

import { println } from std.io;

func main(): i32 {
    let who: str = "Amiga";
    println($"Hello, {who}, from Clorraine!");
    return 0;
}
```

The file is
[`examples/hello/hello.clo`](https://github.com/OpenLorraine/clorraine/blob/main/examples/hello/hello.clo).

## Run it

On the host profile:

```sh
clorraine run examples/hello/hello.clo
```

Output:

```text
Hello, Amiga, from Clorraine!
```

On AmigaOS through `vamos`, from a checkout with the SDK:

```sh
make run FILE=examples/hello/hello.clo PROFILE=hosted-amiga
```

On an A1200 with Kickstart 3.1 in FS-UAE:

```sh
make run FILE=examples/hello/hello.clo \
  PROFILE=hosted-amiga EMULATOR=1
```

On an A500 with Kickstart 1.3:

```sh
make run FILE=examples/hello/hello.clo \
  PROFILE=hosted-amiga-13 EMULATOR=1
```

The emulator paths need a licensed Kickstart ROM. The `vamos` path needs no
ROM.

## What Kickstart 3.1 sees

The screenshot below is from a real A1200 / Kickstart 3.1 boot. The A500 /
Kickstart 1.3 smoke gate still checks the same output line:

<DocFigure
  src="/screenshots/hello31.png"
  alt="An AmigaDOS shell on Kickstart 3.1 showing the line Hello, Amiga, from Clorraine!"
  caption="Native capture from an A1200 / Kickstart 3.1 run."
/>

## The generated C

The compiler emits one C99 translation unit. This is a trimmed excerpt of the
`main` body; line directives, temporary names, and the platform shim are
omitted for readability:

```c
static i32 cl_demo_Hello_main(void)
{
    ClStr who = {0};
    ClString __cl_owned0 = 0;
    ClString __cl_tmp0 = 0;
    i32 __cl_ret = 0;

    who = cl_str("Amiga");
    __cl_tmp0 = cl_text_str((who).ptr, (who).len);
    __cl_owned0 = cl_string_concat(
        (ClStrPart[]){
            cl_str_part_bytes((const u8 *)"Hello, ", 7u),
            cl_str_part_owned(__cl_tmp0),
            cl_str_part_bytes((const u8 *)", from Clorraine!", 17u)
        }, 3u);
    cl_std_io_println(cl_string_view(__cl_owned0));

    __cl_ret = 0;
    if (__cl_owned0) { FreeString(__cl_owned0); }
    return __cl_ret;
}
```

The owned string is released once, on the exit path. The call to
`cl_std_io_println` is a direct C call. There is no interpreter and no
ownership runtime in the generated program.

To read the whole file:

```sh
clorraine emit examples/hello/hello.clo -p host -o build/hello.c
```

## What it demonstrates

| Feature | In this program |
|---|---|
| Modules and imports | `module demo.Hello;` and `import { println } from std.io;` |
| A borrowed string view | `let who: str = "Amiga";` |
| String interpolation | `$"Hello, {who}, from Clorraine!"` |
| An owned temporary | The interpolated string, released at scope exit |
| The entry point | `func main(): i32` |

Next: [the string parser](/examples/string-parser) shows loops, enums, and
`switch` in a larger program.
