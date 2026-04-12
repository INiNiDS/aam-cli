# AGENTS.md

## Project map
- `Cargo.toml` defines a Rust 2024 crate named `aam` that wraps `aam-rs` for AAM (Abstract Alias Mapping) files.
- `src/main.rs` is the CLI entrypoint: `check`, `format`, `get`, and `lsp`; no subcommand opens the TUI.
- `src/tui.rs` owns the interactive editor: multi-file tabs, input bar commands, help/error popups, and Ctrl-based hotkeys.
- `src/lsp.rs` implements the language server with `tower-lsp`; diagnostics are produced from `aam-rs` lexer/parser recovery.

## Command surface to preserve
- `aam check <file>` validates a file and prints counts; failures are reported on stderr and exit non-zero.
- `aam format <file> --dry-run` prints formatted output; without `--dry-run` it writes in place.
- `aam get <file> <key>` prints a value or suggests similar keys.
- `aam lsp` starts the LSP server; plain `aam [files...]` opens the TUI.

## TUI-specific behavior
- The input bar accepts `open <file>`, `check`/`c`, `format`/`f`, `save`/`w`, `get <key>`, `help`/`h`, `quit`/`q`, and `close`.
- Global hotkeys in `src/tui.rs` include `Ctrl+S`, `Ctrl+T`, `Ctrl+F`, `Ctrl+Q`, `Ctrl+H`, `Ctrl+W`, and `Ctrl+Left/Right` or `PgUp/PgDn` for tabs.
- `FileTab::new` preloads file contents into `tui-textarea` and applies a simple key-highlighting search pattern before rendering.
- Duplicate open paths are rejected in `App::new`; keep that guard when changing file-loading logic.

## LSP-specific behavior
- `initialize` advertises full-document sync and formatting.
- `did_open` and `did_change` revalidate text and publish diagnostics; `did_close` clears them.
- Diagnostic positions convert from `AamlError`’s 1-based line/column to LSP’s 0-based positions.

## Local workflow
- Use `cargo build`, `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings` for validation.
- `Makefile` mirrors those checks with `make fmt`, `make lint`, `make test`, `make check`, and `make all`.
- Regenerate third-party notices with `cargo about generate about.hbs -o CREDITS.html` when dependencies change.

## Repo conventions
- Keep `README.md`, `src/main.rs`, and the `.github/` docs aligned when commands, flags, or behavior change.
- Update `.github/CONTRIBUTING.md`, `.github/pull_request_template.md`, and `.github/SECURITY.md` rather than duplicating their guidance.
- Release/install scripts assume the binary name is `aam` and GitHub release assets use names like `aam-linux-amd64` and `aam-windows-amd64.exe`.
- `about.hbs` and `about.toml` drive `CREDITS.html`; avoid editing the generated HTML by hand unless the generator output itself changes.
