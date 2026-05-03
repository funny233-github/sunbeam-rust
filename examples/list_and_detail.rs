//! Builds multi-layered List and Detail structures with every optional field
//! populated, then serializes them to JSON and validates against the schemas.
//!
//! Shows how to assemble complex pages from the sunbeam type system,
//! including nested actions, item detail panels, and markdown content.
//!
//! Usage: cargo run --example list_and_detail

fn main() {
    println!("=== Building Manifest ===");
    let manifest = build_manifest();
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    println!("{}", json);
    println!();

    println!("=== Building List ===");
    let list = build_list();
    let list_json = serde_json::to_string_pretty(&list).unwrap();
    println!("{}", list_json);
    println!();

    println!("=== Building Detail ===");
    let detail = build_detail();
    let detail_json = serde_json::to_string_pretty(&detail).unwrap();
    println!("{}", detail_json);
    println!();

    println!("=== Validating against schemas ===");
    // These will pass if the JSON conforms to the embedded schemas
    let list_bytes = serde_json::to_vec(&list).unwrap();
    match sunbeam_rust::schemas::validate_list(&list_bytes) {
        Ok(_) => println!("  List schema: ✅ VALID"),
        Err(e) => println!("  List schema: ❌ {}", e),
    }

    let detail_bytes = serde_json::to_vec(&detail).unwrap();
    match sunbeam_rust::schemas::validate_detail(&detail_bytes) {
        Ok(_) => println!("  Detail schema: ✅ VALID"),
        Err(e) => println!("  Detail schema: ❌ {}", e),
    }

    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    match sunbeam_rust::schemas::validate_manifest(&manifest_bytes) {
        Ok(_) => println!("  Manifest schema: ✅ VALID"),
        Err(e) => println!("  Manifest schema: ❌ {}", e),
    }

    println!("\n✅ List and Detail construction complete!");
}

fn build_manifest() -> sunbeam_rust::types::Manifest {
    use sunbeam_rust::types::*;

    Manifest {
        title: "Dev Tools".into(),
        description: Some("A collection of developer utilities".into()),
        preferences: Some(vec![
            Input {
                input_type: InputType::String,
                name: "token".into(),
                title: "API Token".into(),
                optional: Some(true),
                default: None,
            },
            Input {
                input_type: InputType::Boolean,
                name: "verbose".into(),
                title: "Verbose Logging".into(),
                optional: None,
                default: Some(serde_json::json!(false)),
            },
        ]),
        commands: vec![
            CommandSpec {
                name: "list-repos".into(),
                title: "List Repositories".into(),
                hidden: None,
                params: Some(vec![
                    Input {
                        input_type: InputType::String,
                        name: "org".into(),
                        title: "Organization".into(),
                        optional: Some(true),
                        default: Some(serde_json::json!("personal")),
                    },
                ]),
                mode: Some(CommandMode::Filter),
            },
            CommandSpec {
                name: "repo-detail".into(),
                title: "Repository Details".into(),
                hidden: Some(true),
                params: None,
                mode: Some(CommandMode::Detail),
            },
            CommandSpec {
                name: "deploy".into(),
                title: "Deploy".into(),
                hidden: None,
                params: None,
                mode: Some(CommandMode::Tty),
            },
            CommandSpec {
                name: "health-check".into(),
                title: "Health Check".into(),
                hidden: None,
                params: None,
                mode: Some(CommandMode::Silent),
            },
        ],
    }
}

fn build_list() -> sunbeam_rust::types::List {
    use sunbeam_rust::types::*;

    List {
        items: Some(vec![
            ListItem {
                id: Some("repo-1".into()),
                title: "sunbeam".into(),
                subtitle: Some("Command-line launcher".into()),
                detail: Some(ListItemDetail {
                    markdown: Some(
                        "# sunbeam\n\nA general-purpose **command-line launcher**.\n\n## Features\n\n- TUI with fuzzy search\n- Extension system\n- Clipboard integration".into(),
                    ),
                    text: Some("sunbeam: command-line launcher".into()),
                }),
                accessories: Some(vec!["Rust".into(), "Stars: 2.3k".into()]),
                actions: Some(vec![
                    Action {
                        title: Some("View Details".into()),
                        key: Some("v".into()),
                        action_type: ActionType::Run,
                        run: Some(RunAction {
                            extension: Some("gh".into()),
                            command: "repo-detail".into(),
                            params: None,
                            reload: None,
                            exit: None,
                        }),
                        open: None, copy: None, exec: None, edit: None,
                        config: None, reload: None,
                    },
                    Action {
                        title: Some("Open in Browser".into()),
                        key: Some("o".into()),
                        action_type: ActionType::Open,
                        open: Some(OpenAction {
                            url: Some("https://github.com/pomdtr/sunbeam".into()),
                            path: None,
                        }),
                        copy: None, run: None, exec: None, edit: None,
                        config: None, reload: None,
                    },
                    Action {
                        title: Some("Copy Clone URL".into()),
                        key: Some("c".into()),
                        action_type: ActionType::Copy,
                        copy: Some(CopyAction {
                            text: Some("git clone https://github.com/pomdtr/sunbeam.git".into()),
                            exit: Some(false),
                        }),
                        open: None, run: None, exec: None, edit: None,
                        config: None, reload: None,
                    },
                ]),
            },
            ListItem {
                id: Some("repo-2".into()),
                title: "ratatui".into(),
                subtitle: Some("Rust TUI library".into()),
                detail: Some(ListItemDetail {
                    markdown: Some("# ratatui\n\nA Rust library for **terminal UI**.\n\n- Ratatui is a fork of tui-rs\n- Backend-agnostic".into()),
                    text: Some("ratatui: Rust TUI library".into()),
                }),
                accessories: Some(vec!["Rust".into(), "Stars: 12k".into()]),
                actions: Some(vec![
                    Action {
                        title: Some("Open in Browser".into()),
                        key: Some("o".into()),
                        action_type: ActionType::Open,
                        open: Some(OpenAction {
                            url: Some("https://github.com/ratatui/ratatui".into()),
                            path: None,
                        }),
                        copy: None, run: None, exec: None, edit: None,
                        config: None, reload: None,
                    },
                ]),
            },
        ]),
        empty_text: Some("No repositories found".into()),
        show_detail: Some(true),
        auto_refresh_seconds: Some(60),
        actions: Some(vec![
            Action {
                title: Some("Refresh".into()),
                key: Some("r".into()),
                action_type: ActionType::Reload,
                reload: Some(ReloadAction { params: None }),
                open: None, copy: None, run: None, exec: None,
                edit: None, config: None,
            },
        ]),
    }
}

fn build_detail() -> sunbeam_rust::types::Detail {
    use sunbeam_rust::types::*;

    Detail {
        markdown: Some(
            "# sunbeam\n\n**Version:** 0.1.0\n\n## Overview\n\nSunbeam is a general-purpose command-line launcher that combines\nscripts written in any language into a TUI-driven workflow.\n\n## Key Features\n\n1. **TUI with live fuzzy filtering** — search items in real-time\n2. **Extensible** — write extensions in Python, Shell, Deno, or any language\n3. **Clipboard integration** — copy results with a keystroke\n4. **Auto-refresh** — live-updating lists\n\n## Quick Start\n\n```bash\nsunbeam\nsunbeam gh\nsunbeam gh search --query rust\n```\n\n## Configuration\n\nThe config file is at `~/.config/sunbeam/sunbeam.json`.".to_string()),
        text: Some("sunbeam: command-line launcher".into()),
        actions: Some(vec![
            Action {
                title: Some("Open Repository".into()),
                key: Some("o".into()),
                action_type: ActionType::Open,
                open: Some(OpenAction {
                    url: Some("https://github.com/pomdtr/sunbeam".into()),
                    path: None,
                }),
                copy: None, run: None, exec: None, edit: None,
                config: None, reload: None,
            },
            Action {
                title: Some("Copy Install Command".into()),
                key: Some("c".into()),
                action_type: ActionType::Copy,
                copy: Some(CopyAction {
                    text: Some("cargo install --path .".into()),
                    exit: Some(false),
                }),
                open: None, run: None, exec: None, edit: None,
                config: None, reload: None,
            },
        ]),
    }
}
