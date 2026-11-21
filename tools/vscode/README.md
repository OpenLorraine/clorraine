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
or install an SDK. After a native Linux installation, run the exact
`code --install-extension ...` command printed by the installer. You can also
use **Extensions: Install from VSIX...** and select that installation's
`clorraine.vsix`. Installing or updating the extension is an explicit action:
the installer does not change VS Code or your shell configuration.

Restart or reload the editor after installation. Grant workspace trust only
for source folders you trust. In Restricted Mode, the extension does not start
the language server, discover workspace tools, save files, or launch tasks.

### Tool discovery

The language server is selected in this order:

1. the `clorraine.lsp.path` setting;
2. the `CLORRAINE_LSP` environment variable;
3. `~/.clorraine/bin/clorraine-lsp`;
4. `target/debug/clorraine-lsp`, then `target/release/clorraine-lsp`, in trusted
   workspace folders;
5. executable files in absolute `PATH` directories.

Driver discovery uses `clorraine.driver.path`, then
`~/.clorraine/bin/clorraine`, trusted workspace development builds, and `PATH`.
The default installation therefore works when VS Code starts from a desktop
launcher without Clorraine in its inherited `PATH`.

All five path settings are machine-level settings. Set them in **User
Settings**, not workspace or language-specific settings:

| Setting | Use |
|---|---|
| `clorraine.lsp.path` | Language server executable; empty uses discovery above |
| `clorraine.driver.path` | Build/run driver executable; empty uses discovery above |
| `clorraine.sdk.path` | Native SDK's `sdk/amiga` directory; empty lets the driver discover its SDK |
| `clorraine.fsUae.path` | Native Linux FS-UAE executable |
| `clorraine.kickstart.path` | Locally licensed Kickstart ROM for the native emulator action |

An explicit invalid executable setting is an error: it never falls back to
another installation. Correct or clear the setting. Workspace and
language-specific path overrides are ignored. For a custom installation root,
set both tool paths to the corresponding executables under that root's `bin`
directory; you need not edit `PATH`.
Keep paths literal, without surrounding shell quotes or `~` expansion.

Development builds remain available with
`make -C docker exec CMD="cargo build --workspace"` from the repository root.

## Build and run saved sources

Open a saved `.clo` entry file, then use the Command Palette:

| Command | Result |
|---|---|
| **Clorraine: Build** | Compile the active entry and its saved dependencies |
| **Clorraine: Run** | Build and run on the host, or through vamos for an Amiga profile |
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
If setting the SDK explicitly, select its **`sdk/amiga`** directory, not `sdk`.
These values are passed only to applicable actions.

The native action uses an isolated A1200 configuration with your licensed
Kickstart 3.1 ROM for either Amiga compilation profile. It stages only the
program and startup script in a private guest drive. Close FS-UAE normally
to let the driver remove its private run directory. Emulator process success
does **not** establish the guest program's exit status.

## WSL2 and Remote-WSL

Install Clorraine inside a supported Ubuntu 22.04 or 24.04 x86_64 WSL2
distribution. Open the source folder in a real **Remote-WSL** window. Install
the VSIX into **WSL: your distribution**, not only into local Windows VS Code.
Use the Extensions view or **Developer: Show Running Extensions** to check
which host runs Clorraine.

Tool discovery and all paths then refer to Linux HOME, Linux executables and
Linux files. Set any custom paths in that remote host's User Settings.
Build/Run use the Linux compiler, SDK and vamos. They do not execute Windows
tools or depend on exporting Clorraine paths in an integrated terminal.

### Use a Windows FS-UAE installation manually

Automatic Windows emulator launch is not provided. The Linux Run in FS-UAE
action is not a bridge to a Windows executable.

1. Build with **Clorraine: Build** using an Amiga profile. For an entry named
   `main.clo`, the executable is
   `build/clorraine/main/hosted-amiga/main` or
   `build/clorraine/main/hosted-amiga-13/main`, relative to the task's source
   root. The task output reports the actual path.
2. In Linux, calculate its SHA-256, for example:

   ```sh
   sha256sum build/clorraine/main/hosted-amiga/main
   ```

3. Copy that executable through the WSL share into a Windows directory you
   choose, such as `C:\Amiga\Clorraine`. In Windows Explorer, the source is
   available under `\\wsl.localhost\<distribution>\<Linux path>`.
   Replace the placeholders with your distribution and source path, using
   backslashes for the Windows share path.
4. Compare the copied bytes in PowerShell:

   ```powershell
   Get-FileHash -Algorithm SHA256 -LiteralPath 'C:\Amiga\Clorraine\main'
   ```

   The digest must match the Linux result; hexadecimal letter case does not
   matter. Do not proceed with a truncated or changed copy.
5. In your existing Windows FS-UAE configuration, mount only that chosen
   Windows directory as a guest volume, for example `CLORRAINE`. Use your own
   licensed Kickstart/Workbench media and a compatible Amiga configuration.
   Boot it, open an Amiga shell, and run `CLORRAINE:main`.

This is a manual transfer/run workflow. Linux desktop tests do not establish
that a particular Windows or Remote-WSL setup has been verified.

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
