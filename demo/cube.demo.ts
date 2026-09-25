import { defineDemo } from "../tools/demo/src/index.ts";

export default defineDemo({
  out: "demo/out/cube.mp4",
  poster: "demo/out/cube.png",
  scenes: [
    { id: "intro", kind: "card", title: "A 3D cube in Clorraine", subtitle: "Drawn line by line on a custom Amiga screen", duration: 6 },
    { id: "src", kind: "code", file: "examples/graphics/cube.clo", scroll: true, duration: 14 },
    { id: "run", kind: "terminal", commands: ["make -s build FILE=examples/graphics/cube.clo PROFILE=hosted-amiga"], duration: 14, caption: "Compiling to a native 68k binary" },
    { id: "emu", kind: "emulator", file: "examples/graphics/cube.clo", frame: true, duration: 20, caption: "A spinning 3D cube — under FS-UAE (Amiga emulator)" },
    { id: "outro", kind: "outro", title: "Clorraine", lines: ["github.com/OpenLorraine"], duration: 6 },
  ],
});
