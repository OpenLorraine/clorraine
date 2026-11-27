# Developing the Clorraine VS Code extension

Maintainer notes for `tools/vscode/`. The user-facing listing is `README.md`;
this file is not packaged.

## Build, test, and package

From the repository root:

```text
make vscode-check
```

This builds the language server in the pinned SDK container, type-checks the
extension, runs the headless tests, packages
`build/vscode/clorraine-<version>.vsix`, audits the archive with
`scripts/audit_vsix.py`, and runs the packaged client against the real server.
`make vscode-package` only packages.

`scripts/stage.mjs` stages the package under `build/vscode/extension`. It
copies the canonical grammar from `editors/`, the listing files, the icon, and
the licence files, and sets the version from `compiler/Cargo.toml`, the
version of record. `tools/vscode/package.json` must carry the same version;
`make dist` and the publish workflow check it.

The icon `images/icon.png` is a 256×256 rendering of `site/public/logo.svg`
(the Marketplace rejects SVG icons). Re-render it with headless Chrome when the
logo changes; ImageMagick's built-in SVG renderer drops the arc.

## Publishing

`.github/workflows/publish-vscode.yml` publishes to the Visual Studio
Marketplace with Trusted Publishing; `RELEASE.md` owns the procedure. Dispatch
it from `main` with `dry_run` ticked to build and audit only; publishing needs
a dispatch from the release tag with `dry_run` unticked, and
`scripts/release-version.mjs` refuses any tag that differs from the compiler
and manifest versions.

## Tool discovery

The language server is selected in this order:

1. the `clorraine.lsp.path` setting;
2. the `CLORRAINE_LSP` environment variable;
3. the installed CLI: `~/.local/bin/clorraine-lsp` on Linux,
   `%LOCALAPPDATA%\Programs\Clorraine\current\bin\clorraine-lsp.exe` on Windows;
4. `target/debug/clorraine-lsp`, then `target/release/clorraine-lsp`, in trusted
   workspace folders;
5. executable files in absolute `PATH` directories.

Driver discovery uses `clorraine.driver.path`, then the installed CLI, trusted
workspace development builds, and `PATH`, so the default installation works
when VS Code starts from a desktop launcher without Clorraine on `PATH`. The
`clorraine.driverPath` command returns the same driver; the starter project's
tasks call it as `${command:clorraine.driverPath}`.

Path settings are machine settings: workspace and language-specific overrides
are ignored, and an invalid explicit setting is an error rather than a
fallback. The removed `clorraine.sdk.path` is read once when `toolchain.path`
is empty and passed as `--toolchain` with a one-time warning.

## Compatible language server

Build and Run require the server's saved-source selection capability, version
1: `capabilities.experimental.clorraineSourceFilesVersion`. The read-only
`clorraine/sourceFiles` request accepts `{entryUri, rootUri}` file URIs and
returns `{files: string[]}` with canonical absolute paths, entry first. An
older server may still provide language features. The version notice compares
`serverInfo.version` with the extension version and only informs.

## Source selection for Build and Run

Source selection reads saved files under the entry's workspace folder (or its
parent directory for a standalone file): transitive imports, qualified
user-module references, and all files in selected modules. It excludes
unrelated programs, hidden and generated directories, and symbolic links. The
index allows at most 2,000 source files and a program at most 256; overflow,
unreadable sources, missing imports, and paths outside the root stop the
action rather than build a partial program. Editing a `.clo` file under the
root while selection runs also stops it.

Tasks run the selected executable directly with a literal argument array, not
through a shell. VS Code still interprets `${…}` in process tasks, so paths
containing `${` are rejected.

## FS-UAE

Run in FS-UAE boots an A1200 with the user's licensed Kickstart 3.1 ROM, runs
the program through a compiled wrapper, and accepts the run only for rc 0 and
status `ok`. A failed or timed-out guest prints its output and keeps the
private run directory. FS-UAE 3.2.0 or newer is required.
