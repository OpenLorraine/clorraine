# Clorraine for VS Code

This package adds Clorraine language support:

- compiler diagnostics from `clorraine-lsp`;
- hover, go to definition, completion, and document symbols;
- the canonical TextMate grammar from `editors/clorraine.tmLanguage.json`;
- Build, Run, and Run in FS-UAE commands for saved source files;
- one `clorraine.profile` setting for language features and actions
  (`hosted-amiga` by default).

## Install the extension

The extension needs compatible Clorraine tools; it does not contain a compiler
or install a toolchain. Build the local VSIX with `make vscode-package` and
install `build/vscode/clorraine-1.2.0.vsix`:

```sh
code --install-extension build/vscode/clorraine-1.2.0.vsix
```

You can also use **Extensions: Install from VSIX...**. The CLI installer and
candidate archive do not bundle or place the VSIX. Installing or updating the
extension is an explicit action: the installer does not change VS Code or your
shell configuration.

Restart or reload the editor after installation. Grant workspace trust only
for source folders you trust. In Restricted Mode, the extension does not start
the language server, discover workspace tools, save files, or launch tasks.

### Tool discovery

The language server is selected in this order:

1. the `clorraine.lsp.path` setting;
2. the `CLORRAINE_LSP` environment variable;
3. `~/.local/bin/clorraine-lsp`;
4. `target/debug/clorraine-lsp`, then `target/release/clorraine-lsp`, in trusted
   workspace folders;
5. executable files in absolute `PATH` directories.

Driver discovery uses `clorraine.driver.path`, then
`~/.local/bin/clorraine`, trusted workspace development builds, and `PATH`.
The default installation therefore works when VS Code starts from a desktop
launcher without Clorraine in its inherited `PATH`.

All five path settings are machine-level settings. Set them in **User
Settings**, not workspace or language-specific settings:

| Setting | Use |
|---|---|
| `clorraine.lsp.path` | Language server executable; empty uses discovery above |
| `clorraine.driver.path` | Build/run driver executable; empty uses discovery above |
| `clorraine.toolchain.path` | Upstream Amiga toolchain root containing `bin/m68k-amigaos-gcc`; empty uses `CLORRAINE_TOOLCHAIN` or the CLI config |
| `clorraine.fsUae.path` | Native Linux FS-UAE executable |
| `clorraine.kickstart.path` | Locally licensed Kickstart ROM for the native emulator action |

An explicit invalid executable setting is an error: it never falls back to
another installation. Correct or clear the setting. Workspace and
language-specific path overrides are ignored. For a custom installation root,
set both tool paths to the corresponding executables under that root's `bin`
directory; you need not edit `PATH`.

The removed `clorraine.sdk.path` is read once when `toolchain.path` is empty.
The action passes it to the CLI as `--toolchain` and shows a warning that the
old SDK layout is no longer used; a second action does not warn again. Select
the upstream root in `clorraine.toolchain.path` to clear the warning path.
Keep paths literal, without surrounding shell quotes or `~` expansion.

Development builds remain available with
`make -C docker exec CMD="cargo build --workspace"` from the repository root.

## Build and run saved sources

Open a saved `.clo` entry file, then use the Command Palette:

| Command | Result |
|---|---|
| **Clorraine: Build** | Compile the active entry and its saved dependencies |
| **Clorraine: Run** | Build and run on the host, or through `vamos` for an Amiga profile (`vamos` must be on `PATH`) |
| **Clorraine: Run in FS-UAE** | Build an Amiga program and start native Linux FS-UAE |

Select `host`, `hosted-amiga`, or `hosted-amiga-13` using `clorraine.profile`.
The same profile controls diagnostics and tasks. `freestanding` is the
provisional LorraineOS target, not a native Linux run target. Run in FS-UAE
accepts only the two Amiga profiles.

The extension saves a dirty active entry before selecting sources. A cancelled
or failed save stops the action. Save any dirty selected dependencies yourself
and retry; the extension never saves unrelated documents. Switching or changing
the entry during selection also stops the launch.

Editing or saving a `.clo` file under the selected root while selection is
pending also stops the action, even if that file is clean when selection ends.
Save the sources and retry. This conservative check can include unrelated
sources in the root; it does not freeze the filesystem against external edits.

Source selection reads saved disk files under the entry's containing workspace
folder, or its parent directory for a standalone file. It includes transitive
imports, qualified user-module references and all files in selected modules.
It excludes unrelated programs, hidden/generated directories and symlinked
files/directories. Embedded standard-library sources stay compiler-owned.
Live diagnostics still use unsaved editor contents; task compilation does not.

The index permits at most 2,000 source files and the selected program at most
256 files. Exact limits are allowed; overflow, unreadable sources, missing
explicit imports and paths outside the selected root stop the action rather
than return a partial program. Use a smaller containing source folder when the
index limit is reached.

Tasks run the selected executable directly with a literal argument array,
not through a shell. The task terminal shows output and process failures.
VS Code still interprets task variables in process tasks. Paths containing the
literal introducer `${` are therefore rejected in the executable, arguments
and working directory. Rename the file/folder or change the tool path in User
Settings and retry; adding quotes or backslashes does not escape this syntax.
Other punctuation, spaces, Unicode and ordinary dollar signs remain literal.
The working directory is the entry's containing root, including in a
multi-folder workspace. The CLI still requires explicit source arguments; it
does not discover dependencies on its own.

### Compatible language server

Actions require the server's saved-source selection capability version **1**:
`capabilities.experimental.clorraineSourceFilesVersion`. The read-only
`clorraine/sourceFiles` request accepts `{entryUri, rootUri}` file URIs and
returns `{files: string[]}` with canonical absolute paths, entry first.

An older or incompatible server may provide language features but cannot
provide these tasks. Install the matching compiler/editor distribution,
correct `clorraine.lsp.path` or `CLORRAINE_LSP` if needed, and reload VS Code.
The extension does not silently compile only the entry.

### Native Linux FS-UAE

Install FS-UAE separately and supply your own licensed media. Configure
`clorraine.fsUae.path` and `clorraine.kickstart.path` in User Settings.
If setting the toolchain explicitly, select the upstream root containing
`bin/m68k-amigaos-gcc`, not an old `sdk/amiga` directory. These values are
passed only to applicable actions.

The native action uses the verified runner: it boots an A1200 with your
licensed 512 KB Kickstart 3.1 A1200 ROM, runs the program through a compiled
wrapper, and accepts the run only for rc 0 plus status `ok`. A failed or
timed-out guest prints its output and retains the private run; the driver
prints the retained path. FS-UAE 3.2.0 or newer is required. The A500 profile
binary also runs on this recipe, but that is not a Kickstart 1.3 proof; the
A500/KS1.3 gates remain in the Docker SDK.

## Platform scope

v1.2.0 supports the Linux desktop extension host only. WSL2, Remote-WSL and
Windows extension hosts are outside this release. The extension makes no
Windows tool-execution, path-sharing or automatic Windows emulator-launch
claim.

## Packaging

From the repository root:

```text
make vscode-check
```

This builds the server in the pinned SDK container, type-checks the extension,
runs the headless tests, packages `build/vscode/clorraine-<version>.vsix`, and
audits the archive.

## License

Clorraine is licensed under the Apache License 2.0, with the Clorraine Output
Exception. The packaged extension includes `LICENSE.txt`,
`LICENSE-EXCEPTION.md`, `NOTICE`, and `TRADEMARKS.md`.
