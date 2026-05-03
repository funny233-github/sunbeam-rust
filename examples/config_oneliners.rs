//! Demonstrates building and operating on oneliner configurations with
//! serde round-tripping and manual ListItem construction.
//!
//! The `sunbeam_rust::builder` module is part of the binary crate and not
//! exposed through the library, so we construct the ListItems directly here.
//!
//! Usage: cargo run --example config_oneliners

fn main() {
    use sunbeam_rust::config::{Config, Oneliner};
    use sunbeam_rust::types::*;
    use std::path::PathBuf;

    // ── 1. Build oneliners with all variants ────────────────────────────
    println!("=== 1. Oneliner Variants ===");
    let oneliners = vec![
        Oneliner {
            title: "Hello World".into(),
            command: "echo hello".into(),
            interactive: None,
            cwd: None,
            exit: None,
        },
        Oneliner {
            title: "Interactive Editor".into(),
            command: "vim ~/notes.txt".into(),
            interactive: Some(true),
            cwd: None,
            exit: Some(false),
        },
        Oneliner {
            title: "Run Script".into(),
            command: "./deploy.sh --env production".into(),
            interactive: Some(false),
            cwd: Some("/home/user/project".into()),
            exit: Some(true),
        },
        Oneliner {
            title: "System Update".into(),
            command: "sudo pacman -Syu".into(),
            interactive: Some(true),
            cwd: None,
            exit: Some(false),
        },
    ];

    for o in &oneliners {
        let interactive = if o.interactive.unwrap_or(false) { "interactive" } else { "silent" };
        let exit_str = if o.exit.unwrap_or(false) { "exit" } else { "stay" };
        println!("  {} — {} ({}, {})", o.title, o.command, interactive, exit_str);
    }

    // ── 2. Manually construct ListItems from oneliners ──────────────────
    println!("\n=== 2. ListItems from Oneliners ===");
    let items: Vec<ListItem> = oneliners.iter().map(|o| {
        ListItem {
            id: Some(format!("oneliner - {}", o.title)),
            title: o.title.clone(),
            subtitle: None,
            detail: None,
            accessories: Some(vec!["Oneliner".into()]),
            actions: Some(vec![
                Action {
                    title: Some("Run".into()),
                    key: None,
                    action_type: ActionType::Exec,
                    exec: Some(ExecAction {
                        command: o.command.clone(),
                        interactive: o.interactive,
                        dir: o.cwd.clone(),
                        exit: o.exit,
                    }),
                    open: None, copy: None, run: None, edit: None, config: None, reload: None,
                },
                Action {
                    title: Some("Copy Command".into()),
                    key: Some("c".into()),
                    action_type: ActionType::Copy,
                    copy: Some(CopyAction {
                        text: Some(o.command.clone()),
                        exit: None,
                    }),
                    open: None, run: None, exec: None, edit: None, config: None, reload: None,
                },
            ]),
        }
    }).collect();

    println!("  Generated {} ListItems:", items.len());
    for item in &items {
        println!("    {} — accessories: {:?}",
            item.title,
            item.accessories.as_ref().unwrap_or(&vec![]));
        if let Some(actions) = &item.actions {
            for action in actions {
                println!("      action: {} ({:?})",
                    action.title.as_deref().unwrap_or(""),
                    action.action_type);
            }
        }
    }

    // ── 3. Oneliner serde roundtrip ─────────────────────────────────────
    println!("\n=== 3. Oneliner Serde Roundtrip ===");
    let json = serde_json::to_string_pretty(&oneliners).unwrap();
    println!("{}", json);
    let deserialized: Vec<Oneliner> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), oneliners.len());
    assert_eq!(deserialized[0].title, "Hello World");
    println!("  Roundtrip: OK");

    println!("\n✅ Oneliner config demo complete!");
}
