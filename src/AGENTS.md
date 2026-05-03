# PROJECT KNOWLEDGE BASE

**Generated:** 2026-05-03
**Commit:** 7d54ece
**Branch:** master

## OVERVIEW

Rust source modules — entry point, types, config, extensions, history, schemas, TUI. Mirrors Go `internal/` layout.

## MODULES

| File | Purpose |
|------|---------|
| `main.rs` | CLI dispatch, extension invocation, root list builder |
| `lib.rs` | Module declarations |
| `types.rs` | Core types (Action, Payload, ListItem, Detail, Input, CommandSpec) |
| `config.rs` | Config struct, JSON loading, path resolution |
| `extensions.rs` | Extension loading from URL/file |
| `history.rs` | Usage history (JSON, timestamp sorted) |
| `schemas.rs` | JSON Schema validation (jsonschema crate) |
| `utils.rs` | Editor, URL open, path normalize, origin parse |
| `tui/mod.rs` | TUI module declarations |
| `tui/render.rs` | Ratatui rendering (list, detail, form) |
| `tui/runner.rs` | Extension command runner |
| `tui/key.rs` | Keyboard event mapping |
| `tui/types.rs` | TUI-specific types |

## WHERE TO LOOK

| Task | File |
|------|------|
| CLI dispatch | `main.rs` (load_config_or_default, dispatch_core) |
| Extension invocation | `main.rs` (run_extension_invocation) |
| All core types | `types.rs` (serde structs) |
| Extension runner | `tui/runner.rs` |
| TUI rendering | `tui/render.rs` |

## CONVENTIONS

- All functions return `Result<T>` (anyhow)
- Sync code only (no async)
- Module paths aligned with Go `internal/` for cross-reference
- Clap derive for subcommands
- stdin/stdout JSON protocol (same as Go version)

## ANTI-PATTERNS (THIS DIRECTORY)

- No async/await
- Keep module structure aligned with Go `internal/`
