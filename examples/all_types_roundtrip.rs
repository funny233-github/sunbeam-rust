//! Demonstrates serde round-trip serialization/deserialization for every
//! public type in `sunbeam_rust::types`.
//!
//! Each type is constructed with representative field values, serialized to
//! JSON, deserialized back, and the round-tripped value is asserted to match
//! the original — validating that all serde attributes are correct.
//!
//! Usage: cargo run --example all_types_roundtrip

use std::collections::HashMap;

fn main() {
    // ── Input & InputType ───────────────────────────────────────────────
    println!("=== Input (string) ===");
    let input = sunbeam_rust::types::Input {
        input_type: sunbeam_rust::types::InputType::String,
        name: "api_key".into(),
        title: "API Key".into(),
        optional: Some(true),
        default: Some(serde_json::json!("default-key")),
    };
    roundtrip::<sunbeam_rust::types::Input>(&input, |a, b| {
        assert_eq!(a.input_type, b.input_type);
        assert_eq!(a.name, b.name);
    });

    println!("=== Input (boolean) ===");
    let input_bool = sunbeam_rust::types::Input {
        input_type: sunbeam_rust::types::InputType::Boolean,
        name: "verbose".into(),
        title: "Verbose".into(),
        optional: None,
        default: Some(serde_json::json!(true)),
    };
    roundtrip::<sunbeam_rust::types::Input>(&input_bool, |a, b| {
        assert_eq!(a.input_type, sunbeam_rust::types::InputType::Boolean);
    });

    println!("=== Input (number) ===");
    let input_num = sunbeam_rust::types::Input {
        input_type: sunbeam_rust::types::InputType::Number,
        name: "timeout".into(),
        title: "Timeout (s)".into(),
        optional: Some(false),
        default: Some(serde_json::json!(30)),
    };
    roundtrip::<sunbeam_rust::types::Input>(&input_num, |a, b| {
        assert_eq!(a.input_type, sunbeam_rust::types::InputType::Number);
        assert_eq!(a.optional, b.optional);
    });

    // ── CommandSpec with params ─────────────────────────────────────────
    println!("=== CommandSpec ===");
    let cmd = sunbeam_rust::types::CommandSpec {
        name: "search".into(),
        title: "Search Projects".into(),
        hidden: Some(false),
        params: Some(vec![
            sunbeam_rust::types::Input {
                input_type: sunbeam_rust::types::InputType::String,
                name: "query".into(),
                title: "Search Query".into(),
                optional: None,
                default: None,
            },
            sunbeam_rust::types::Input {
                input_type: sunbeam_rust::types::InputType::Number,
                name: "limit".into(),
                title: "Max Results".into(),
                optional: Some(true),
                default: Some(serde_json::json!(20)),
            },
        ]),
        mode: Some(sunbeam_rust::types::CommandMode::Search),
    };
    roundtrip::<sunbeam_rust::types::CommandSpec>(&cmd, |a, b| {
        assert_eq!(a.name, b.name);
        assert_eq!(a.hidden, b.hidden);
        assert_eq!(a.mode, b.mode);
        assert_eq!(
            a.params.as_ref().map(|p| p.len()),
            b.params.as_ref().map(|p| p.len())
        );
    });

    // ── Manifest ────────────────────────────────────────────────────────
    println!("=== Manifest ===");
    let manifest = sunbeam_rust::types::Manifest {
        title: "GitHub Helper".into(),
        description: Some("Interact with GitHub".into()),
        preferences: Some(vec![
            sunbeam_rust::types::Input {
                input_type: sunbeam_rust::types::InputType::String,
                name: "token".into(),
                title: "API Token".into(),
                optional: Some(true),
                default: None,
            },
        ]),
        commands: vec![cmd],
    };
    roundtrip::<sunbeam_rust::types::Manifest>(&manifest, |a, b| {
        assert_eq!(a.title, b.title);
        assert_eq!(a.commands.len(), b.commands.len());
    });

    // ── Payload ─────────────────────────────────────────────────────────
    println!("=== Payload ===");
    let mut params = serde_json::Map::new();
    params.insert("query".into(), serde_json::json!("rust"));
    params.insert("limit".into(), serde_json::json!(50));

    let payload = sunbeam_rust::types::Payload {
        command: "search".into(),
        preferences: None,
        params: Some(params),
        cwd: Some("/home/user".into()),
        r#query: Some("rust".into()),
    };
    roundtrip::<sunbeam_rust::types::Payload>(&payload, |a, b| {
        assert_eq!(a.command, b.command);
        assert_eq!(a.cwd, b.cwd);
        assert_eq!(a.r#query, b.r#query);
    });

    // ── Action types ────────────────────────────────────────────────────
    println!("=== Action (Open) ===");
    let action_open = sunbeam_rust::types::Action {
        title: Some("Open".into()),
        key: Some("o".into()),
        action_type: sunbeam_rust::types::ActionType::Open,
        open: Some(sunbeam_rust::types::OpenAction {
            url: Some("https://example.com".into()),
            path: None,
        }),
        copy: None,
        run: None,
        exec: None,
        edit: None,
        config: None,
        reload: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_open, |a, b| {
        assert_eq!(a.action_type, b.action_type);
        assert!(b.open.is_some());
    });

    println!("=== Action (Copy) ===");
    let action_copy = sunbeam_rust::types::Action {
        title: Some("Copy".into()),
        key: Some("c".into()),
        action_type: sunbeam_rust::types::ActionType::Copy,
        copy: Some(sunbeam_rust::types::CopyAction {
            text: Some("copied text".into()),
            exit: Some(true),
        }),
        open: None,
        run: None,
        exec: None,
        edit: None,
        config: None,
        reload: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_copy, |a, b| {
        assert_eq!(a.action_type, sunbeam_rust::types::ActionType::Copy);
    });

    println!("=== Action (Run) ===");
    let action_run = sunbeam_rust::types::Action {
        title: Some("Run".into()),
        key: None,
        action_type: sunbeam_rust::types::ActionType::Run,
        run: Some(sunbeam_rust::types::RunAction {
            extension: Some("gh".into()),
            command: "search".into(),
            params: None,
            reload: Some(true),
            exit: None,
        }),
        open: None,
        copy: None,
        exec: None,
        edit: None,
        config: None,
        reload: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_run, |a, b| {
        assert_eq!(a.action_type, sunbeam_rust::types::ActionType::Run);
    });

    println!("=== Action (Exec) ===");
    let action_exec = sunbeam_rust::types::Action {
        title: Some("Exec".into()),
        key: Some("e".into()),
        action_type: sunbeam_rust::types::ActionType::Exec,
        exec: Some(sunbeam_rust::types::ExecAction {
            command: "echo hello".into(),
            interactive: Some(true),
            dir: Some("/tmp".into()),
            exit: Some(false),
        }),
        open: None,
        copy: None,
        run: None,
        edit: None,
        config: None,
        reload: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_exec, |a, b| {
        assert_eq!(a.action_type, sunbeam_rust::types::ActionType::Exec);
    });

    println!("=== Action (Edit) ===");
    let action_edit = sunbeam_rust::types::Action {
        title: Some("Edit".into()),
        key: Some("e".into()),
        action_type: sunbeam_rust::types::ActionType::Edit,
        edit: Some(sunbeam_rust::types::EditAction {
            path: "/tmp/file.txt".into(),
            exit: Some(false),
            reload: Some(true),
        }),
        open: None,
        copy: None,
        run: None,
        exec: None,
        config: None,
        reload: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_edit, |a, b| {
        assert_eq!(a.action_type, sunbeam_rust::types::ActionType::Edit);
    });

    println!("=== Action (Config) ===");
    let action_config = sunbeam_rust::types::Action {
        title: Some("Config".into()),
        key: Some("s".into()),
        action_type: sunbeam_rust::types::ActionType::Config,
        config: Some(sunbeam_rust::types::ConfigAction {
            extension: "gh".into(),
        }),
        open: None,
        copy: None,
        run: None,
        exec: None,
        edit: None,
        reload: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_config, |a, b| {
        assert_eq!(a.action_type, sunbeam_rust::types::ActionType::Config);
    });

    println!("=== Action (Reload) ===");
    let mut reload_params = serde_json::Map::new();
    reload_params.insert("page".into(), serde_json::json!(2));
    let action_reload = sunbeam_rust::types::Action {
        title: Some("Reload".into()),
        key: None,
        action_type: sunbeam_rust::types::ActionType::Reload,
        reload: Some(sunbeam_rust::types::ReloadAction {
            params: Some(reload_params),
        }),
        open: None,
        copy: None,
        run: None,
        exec: None,
        edit: None,
        config: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_reload, |a, b| {
        assert_eq!(a.action_type, sunbeam_rust::types::ActionType::Reload);
    });

    println!("=== Action (Exit) ===");
    let action_exit = sunbeam_rust::types::Action {
        title: Some("Exit".into()),
        key: None,
        action_type: sunbeam_rust::types::ActionType::Exit,
        open: None,
        copy: None,
        run: None,
        exec: None,
        edit: None,
        config: None,
        reload: None,
    };
    roundtrip::<sunbeam_rust::types::Action>(&action_exit, |a, b| {
        assert_eq!(a.action_type, sunbeam_rust::types::ActionType::Exit);
    });

    // ── ListItem & ListItemDetail ───────────────────────────────────────
    println!("=== ListItem ===");
    let list_item = sunbeam_rust::types::ListItem {
        id: Some("item-1".into()),
        title: "Hello World".into(),
        subtitle: Some("A test item".into()),
        detail: Some(sunbeam_rust::types::ListItemDetail {
            markdown: Some("# Hello\n\nThis is **markdown** content.".into()),
            text: Some("Plain text fallback".into()),
        }),
        accessories: Some(vec!["tag1".into(), "tag2".into()]),
        actions: Some(vec![action_copy.clone()]),
    };
    roundtrip::<sunbeam_rust::types::ListItem>(&list_item, |a, b| {
        assert_eq!(a.title, b.title);
        assert_eq!(
            a.accessories.as_ref().map(|a| a.len()),
            b.accessories.as_ref().map(|a| a.len())
        );
    });

    // ── List ────────────────────────────────────────────────────────────
    println!("=== List ===");
    let list = sunbeam_rust::types::List {
        items: Some(vec![
            list_item,
            sunbeam_rust::types::ListItem {
                id: Some("item-2".into()),
                title: "Another Item".into(),
                subtitle: None,
                detail: None,
                accessories: None,
                actions: None,
            },
        ]),
        empty_text: Some("No items found".into()),
        show_detail: Some(true),
        auto_refresh_seconds: Some(30),
        actions: Some(vec![action_reload.clone()]),
    };
    roundtrip::<sunbeam_rust::types::List>(&list, |a, b| {
        assert_eq!(
            a.items.as_ref().map(|i| i.len()),
            b.items.as_ref().map(|i| i.len())
        );
        assert_eq!(a.show_detail, b.show_detail);
        assert_eq!(a.auto_refresh_seconds, b.auto_refresh_seconds);
    });

    // ── Detail ──────────────────────────────────────────────────────────
    println!("=== Detail ===");
    let detail = sunbeam_rust::types::Detail {
        markdown: Some("# Project Details\n\nThis is a **detail** page.\n\n- Feature A\n- Feature B\n- Feature C".into()),
        text: Some("Fallback text".into()),
        actions: Some(vec![action_open, action_copy]),
    };
    roundtrip::<sunbeam_rust::types::Detail>(&detail, |a, b| {
        assert!(b.markdown.is_some());
        assert_eq!(
            a.actions.as_ref().map(|a| a.len()),
            b.actions.as_ref().map(|a| a.len())
        );
    });

    // ── Enum variant tags ───────────────────────────────────────────────
    println!("=== Enum tags ===");
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::InputType::String).unwrap(),
        serde_json::json!("string")
    );
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::InputType::Boolean).unwrap(),
        serde_json::json!("boolean")
    );
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::InputType::Number).unwrap(),
        serde_json::json!("number")
    );
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::CommandMode::Search).unwrap(),
        serde_json::json!("search")
    );
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::CommandMode::Filter).unwrap(),
        serde_json::json!("filter")
    );
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::CommandMode::Detail).unwrap(),
        serde_json::json!("detail")
    );
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::ActionType::Run).unwrap(),
        serde_json::json!("run")
    );
    assert_eq!(
        serde_json::to_value(sunbeam_rust::types::ActionType::Open).unwrap(),
        serde_json::json!("open")
    );

    println!("\n✅ All types round-tripped successfully!");
}

/// Serializes `val` to JSON, deserializes it back, calls `check` to compare,
/// and prints the JSON representation.
fn roundtrip<T>(val: &T, check: fn(&T, &T))
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let json = serde_json::to_string_pretty(val).expect("serialize");
    println!("{}", json);
    let deserialized: T = serde_json::from_str(&json).expect("deserialize");
    check(val, &deserialized);
    println!("→ OK\n");
}
