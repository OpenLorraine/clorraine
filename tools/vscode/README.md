# Clorraine for VS Code

This package adds Clorraine language support:

- compiler diagnostics from `clorraine-lsp`;
- hover, go to definition, completion, and document symbols;
- the canonical TextMate grammar from `editors/clorraine.tmLanguage.json`;
- the `hosted-amiga` profile by default, configurable as `clorraine.profile`.

## Server discovery

The extension looks for `clorraine-lsp` in this order:

1. the `clorraine.lsp.path` setting;
2. the `CLORRAINE_LSP` environment variable;
3. `<workspace>/target/debug/clorraine-lsp` and `target/release/clorraine-lsp`;
4. `PATH`.

Build it with `make -C docker exec CMD="cargo build --workspace"`.

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
