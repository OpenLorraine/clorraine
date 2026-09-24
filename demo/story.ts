import { defineDemo } from "../tools/demo/src/index.ts";

/**
 * The Clorraine story: the fifteen chapters of the project introduction, from the
 * opening card to the LorraineOS outro. Cards explain, code and terminal chapters
 * show the toolchain, and emulator chapters run the same programs under FS-UAE.
 */
export default defineDemo({
  out: "demo/demo-full.mp4",
  poster: "demo/poster.png",
  gif: {
    out: "demo/demo.gif",
    sceneIds: ["intro", "hello-src", "hello-emu", "hello-explain"],
  },
  scenes: [
    {
      id: "intro",
      kind: "card",
      title: "Clorraine",
      subtitle: "A safe, C-like language for the classic AmigaOS and the future LorraineOS",
      lines: ["Makes programming the Amiga easy", "Rust-style ownership · compiles to plain C99"],
    },
    {
      id: "hello-src",
      kind: "code",
      file: "examples/hello/hello.clo",
      caption: "The smallest Clorraine program",
    },
    {
      id: "hello-emu",
      kind: "emulator",
      file: "examples/hello/hello.clo",
      frame: true,
      caption: "Running under FS-UAE (Amiga emulator)",
    },
    {
      id: "hello-explain",
      kind: "card",
      title: "C-like syntax, checked like Rust",
      subtitle: "Ownership and borrows are checked at compile time",
      lines: ["The compiler emits plain C99 the Amiga toolchains accept"],
    },
    {
      id: "real",
      kind: "card",
      title: "Now a real program",
      subtitle: "A parser that sums numbers and formats the result",
    },
    {
      id: "source",
      kind: "code",
      file: "spec/tests/corpus/string-parser/program.clo",
      caption: "Real Clorraine source — C-like syntax, ownership checked at compile time",
    },
    {
      id: "own-explain",
      kind: "card",
      title: "Ownership, checked",
      subtitle: "Every value has one owner; using it after it moves is a compile error",
      lines: ["This catches memory bugs a C compiler cannot"],
    },
    {
      id: "ownership",
      kind: "terminal",
      commands: ["make -s run FILE=examples/ownership-fail.clo"],
      caption: "Use after move? The compiler rejects it (E10020) — C cannot do this",
    },
    {
      id: "build-explain",
      kind: "card",
      title: "One command",
      subtitle: "Clorraine emits plain C99, then builds and runs it",
      lines: ["Here for the hosted-amiga profile"],
    },
    {
      id: "run",
      kind: "terminal",
      commands: ["make -s run FILE=spec/tests/corpus/string-parser/program.clo PROFILE=hosted-amiga"],
      caption: "One command: build the m68k binary and run it (hosted-amiga profile)",
    },
    {
      id: "emulator",
      kind: "emulator",
      file: "spec/tests/corpus/string-parser/program.clo",
      frame: true,
      caption: "The same program under FS-UAE — 68k, Kickstart 3.1",
    },
    {
      id: "draw-explain",
      kind: "card",
      title: "Now: draw",
      subtitle: "A custom Amiga screen with colors, bars, and lines",
    },
    {
      id: "graphics",
      kind: "emulator",
      file: "spec/tests/corpus/graphics/program.clo",
      frame: true,
      caption: "Drawing on an Amiga screen — under FS-UAE (Amiga emulator)",
    },
    {
      id: "los",
      kind: "card",
      title: "LorraineOS",
      subtitle: "Planned: a clean-room implementation for the m68k Amiga",
      lines: ["Clorraine is part of that effort", "Very early in design"],
    },
    {
      id: "outro",
      kind: "outro",
      title: "Clorraine · early development",
      lines: [
        "spec frozen · compiler MVP working · corpus green",
        "github.com/OpenLorraine",
        "Contact: github.com/blendsdk",
      ],
    },
  ],
});
