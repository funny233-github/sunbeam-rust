# sunbeam-rust

A Rust port of [sunbeam](https://github.com/pomdtr/sunbeam) — a general-purpose command-line launcher.
Combine scripts written in any language into a TUI-driven workflow with live fuzzy search.

## Features

- TUI with live fuzzy filtering, detail panels, and action bars
- Extensions are any executable script (Python, Shell, Deno, …)
- Auto-registered CLI subcommands for every extension command
- Dynamic CLI flags derived from manifest params
- Clipboard, editor, and URL opening helpers
- JSON Schema validation at runtime

## Quick Start

```bash
cargo install --path .
sunbeam
```

First run auto-creates `~/.config/sunbeam/sunbeam.json` with default oneliners.

## Configuration

The config file is resolved from (in order): `$SUNBEAM_CONFIG`, a `sunbeam.json` found by walking up from the current directory, or `~/.config/sunbeam/sunbeam.json`.

```json
{
  "oneliners": [
    { "title": "Open Docs", "command": "sunbeam open https://sunbeam.pomdtr.me/docs", "exit": true }
  ],
  "extensions": {
    "gh": { "origin": "./gh.sh" }
  }
}
```

- **oneliners** — quick shell commands shown in the root list
- **extensions** — installed extensions keyed by alias

## Usage

### TUI

```
sunbeam                          Root list (oneliners + extensions)
sunbeam <alias>                  Extension command list
sunbeam <alias> <command>        Run a specific command
```

### Core Commands

```
sunbeam edit [file]              Open file in $VISUAL / $EDITOR
sunbeam edit --config            Edit the config file
sunbeam copy                     Copy stdin to clipboard
sunbeam paste                    Paste clipboard to stdout
sunbeam open <target>            Open URL or file in default app
```

### Validation

```
sunbeam validate list            Validate List JSON from stdin
sunbeam validate detail          Validate Detail JSON from stdin
sunbeam validate manifest        Validate Manifest JSON from stdin
sunbeam validate config [path]   Validate config file
```

### Extension Management

```
sunbeam extension install <origin> [--alias <name>]
sunbeam extension list
sunbeam extension remove <alias>...
sunbeam extension rename <old> <new>
sunbeam extension upgrade [<alias>] [--all]
sunbeam extension configure <alias>
sunbeam extension edit <alias>
```

## Extension System

An extension is any executable script. When invoked with no arguments it must print a JSON **manifest** to stdout. When invoked with a JSON payload argument it prints a **List** or **Detail** page.

### Manifest

```json
{
  "title": "GitHub",
  "description": "Interact with GitHub",
  "preferences": [{ "type": "string", "name": "token", "title": "API Token" }],
  "commands": [
    {
      "name": "pr-list",
      "title": "List Pull Requests",
      "mode": "filter",
      "params": [
        { "type": "string", "name": "state", "title": "PR state (open/closed)", "optional": true }
      ]
    }
  ]
}
```

### Command Modes

| Mode     | Behaviour                                      |
|----------|------------------------------------------------|
| `filter` | Items are filtered client-side by fuzzy match  |
| `search` | Each query change triggers a new invocation    |
| `detail` | Full-page markdown / plain text view           |
| `tty`    | Runs interactively in the parent terminal      |
| `silent` | Executes silently; result shown as notification|

## Environment Variables

| Variable | Description |
|---|---|
| `SUNBEAM_CONFIG` | Custom config file path |
| `SUNBEAM` | Set to `1` when running inside sunbeam |
| `<ALIAS>_<PREFERENCE>` | Override a preference per extension alias |

## Examples

See [examples/](examples/) for runnable code samples:

```bash
cargo run --example config
cargo run --example resolve_config_path
```
