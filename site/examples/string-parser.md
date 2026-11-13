---
title: String parser
description: "A complete Clorraine program that classifies tokens, sums numbers, and formats hex without library help."
---

# String parser

This program classifies every space-separated token of a fixed input line as a
number or a word, sums the numbers, and formats the result as hex. It uses loops,
an enum, `switch`, string indexing, and `std.fmt`.

## Source

```clorraine
module tests.corpus.StringParser;

import { toHex } from std.fmt;
import { String } from std.text;
import { println } from std.io;

enum TokenKind : u8 {
    Word,
    Number,
}

const INPUT: static str = "MOV 12 34 ADD 5 7";

func classify(input: str, start: usize, end: usize): TokenKind {
    let allDigits: bool = true;
    let j: usize = start;
    while (j < end) {
        if (input[j] < '0' || input[j] > '9') {
            allDigits = false;
        }
        j = j + 1;
    }
    if (allDigits) {
        return TokenKind.Number;
    }
    return TokenKind.Word;
}

func parseNumber(input: str, start: usize, end: usize): i32 {
    let value: i32 = 0;
    let j: usize = start;
    while (j < end) {
        value = value * 10 + i32(input[j] - '0');
        j = j + 1;
    }
    return value;
}

func main(): i32 {
    let input: str = INPUT;
    println($"string-parser: input={input}");
    let total: usize = len(input);
    let tokens: u32 = 0;
    let numbers: u32 = 0;
    let words: u32 = 0;
    let sum: i32 = 0;

    let i: usize = 0;
    while (i < total) {
        while (i < total && input[i] == ' ') {
            i = i + 1;
        }
        let start: usize = i;
        while (i < total && input[i] != ' ') {
            i = i + 1;
        }
        if (i > start) {
            let kind: TokenKind = classify(input, start, i);
            tokens = tokens + 1;
            switch (kind) {
                case TokenKind.Number:
                    numbers = numbers + 1;
                    sum = sum + parseNumber(input, start, i);
                    break;
                case TokenKind.Word:
                    words = words + 1;
                    break;
                default:
                    break;
            }
        }
    }

    println($"string-parser: tokens={tokens} numbers={numbers} words={words}");
    println($"string-parser: sum={sum}");
    let hex: String = toHex(u32(sum));
    println($"string-parser: hex={hex}");
    println("string-parser: ok");
    return 0;
}
```

The file is
[`spec/tests/corpus/string-parser/program.clo`](https://github.com/OpenLorraine/clorraine/blob/main/spec/tests/corpus/string-parser/program.clo).

## Output

```text
string-parser: input=MOV 12 34 ADD 5 7
string-parser: tokens=6 numbers=4 words=2
string-parser: sum=58
string-parser: hex=0x3A
string-parser: ok
```

The same output is produced on the host profile and on `hosted-amiga`. The
program is part of the validation corpus, and its output is a committed golden.

## Run it

```sh
clorraine run spec/tests/corpus/string-parser/program.clo
```

On AmigaOS through `vamos`:

```sh
make run FILE=spec/tests/corpus/string-parser/program.clo PROFILE=hosted-amiga
```

## What it demonstrates

| Feature | Where |
|---|---|
| Enum with an integer backing type | `enum TokenKind : u8` |
| Scoped enum members | `TokenKind.Number` |
| Exhaustive `switch` | The `case` arms in `main` |
| String indexing and bounds checks | `input[i]` |
| String views versus owned strings | `str` parameters, `String` result |
| Explicit numeric conversion | `i32(input[j] - '0')` |
| Standard-library formatting | `toHex(u32(sum))` |
| Loops and early returns | `classify`, `parseNumber`, and the token walk |

## Why it is a good test

The program is deliberately unglamorous. It has nested loops, a helper that
returns an enum, a `switch` with a default arm, integer arithmetic, and a
formatted string. Those are the places where a compiler's semantics show up.

It is also deterministic: the input is a constant, so the output can be compared
byte for byte on the host and under `vamos`.
