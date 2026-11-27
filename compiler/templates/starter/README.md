# My Amiga program

A Clorraine program for the Amiga, made with `clorraine new`.

## Run it

In VS Code, open `main.clo` and press the **Run in FS-UAE** button in the
editor title bar (or press **Ctrl+Shift+B**). An Amiga 1200 boots in FS-UAE
and runs your program. Click the left mouse button in the Amiga window to quit.
The button and the build task come from the Clorraine extension, which VS Code
recommends when you open the folder.

From a terminal:

```sh
clorraine run -p hosted-amiga --emulator main.clo
```

## Change it

The top of `main.clo` has a **Things to change** section: the message, the
sky colours, and the speed. Change something, save, and run it again.

## Learn more

- Clorraine: https://github.com/OpenLorraine/clorraine
- Examples: https://github.com/OpenLorraine/clorraine/tree/main/examples/amiga
