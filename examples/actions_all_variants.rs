//! Creates all 8 Action variants with complete payload sub-structures,
//! serializes each to JSON, and verifies the correct `type` discriminator.
//!
//! The Action type is sunbeam's universal "do something" unit — it can open
//! URLs, copy text, run commands, edit files, configure extensions, and more.
//!
//! Usage: cargo run --example actions_all_variants

fn main() {
    // ── 1. Open: open a URL or file ─────────────────────────────────────
    println!("=== 1. Open (URL) ===");
    let open_url = action(
        "Open URL",
        Some("o"),
        sunbeam_rust::types::ActionType::Open,
        Some(sunbeam_rust::types::OpenAction {
            url: Some("https://github.com".into()),
            path: None,
        }),
        None, None, None, None, None, None,
    );
    print_action(&open_url);

    println!("=== 1b. Open (File) ===");
    let open_file = action(
        "Open Config",
        Some("O"),
        sunbeam_rust::types::ActionType::Open,
        Some(sunbeam_rust::types::OpenAction {
            url: None,
            path: Some("~/.config/sunbeam/sunbeam.json".into()),
        }),
        None, None, None, None, None, None,
    );
    print_action(&open_file);

    // ── 2. Copy: copy text to clipboard ─────────────────────────────────
    println!("=== 2. Copy ===");
    let copy_action = action(
        "Copy Link",
        Some("c"),
        sunbeam_rust::types::ActionType::Copy,
        None,
        Some(sunbeam_rust::types::CopyAction {
            text: Some("https://example.com".into()),
            exit: Some(false),
        }),
        None, None, None, None, None,
    );
    print_action(&copy_action);

    // ── 3. Run: invoke another extension command ────────────────────────
    println!("=== 3. Run (same extension) ===");
    let run_same = action(
        "View Details",
        Some("v"),
        sunbeam_rust::types::ActionType::Run,
        None, None,
        Some(sunbeam_rust::types::RunAction {
            extension: None, // same extension
            command: "repo-detail".into(),
            params: None,
            reload: None,
            exit: None,
        }),
        None, None, None, None,
    );
    print_action(&run_same);

    println!("=== 3b. Run (cross extension) ===");
    let mut params = serde_json::Map::new();
    params.insert("name".into(), serde_json::json!("sunbeam"));
    let run_cross = action(
        "Search Repository",
        Some("s"),
        sunbeam_rust::types::ActionType::Run,
        None, None,
        Some(sunbeam_rust::types::RunAction {
            extension: Some("gh".into()),
            command: "search".into(),
            params: Some(params),
            reload: Some(true),
            exit: Some(false),
        }),
        None, None, None, None,
    );
    print_action(&run_cross);

    // ── 4. Exec: run a shell command ────────────────────────────────────
    println!("=== 4. Exec (interactive) ===");
    let exec_interactive = action(
        "Open Editor",
        Some("e"),
        sunbeam_rust::types::ActionType::Exec,
        None, None, None,
        Some(sunbeam_rust::types::ExecAction {
            command: "vim ~/.config/sunbeam/sunbeam.json".into(),
            interactive: Some(true),
            dir: None,
            exit: Some(false),
        }),
        None, None, None,
    );
    print_action(&exec_interactive);

    println!("=== 4b. Exec (silent) ===");
    let exec_silent = action(
        "Disk Usage",
        None,
        sunbeam_rust::types::ActionType::Exec,
        None, None, None,
        Some(sunbeam_rust::types::ExecAction {
            command: "df -h".into(),
            interactive: Some(false),
            dir: Some("/".into()),
            exit: None,
        }),
        None, None, None,
    );
    print_action(&exec_silent);

    // ── 5. Edit: open a file in the editor ──────────────────────────────
    println!("=== 5. Edit ===");
    let edit_action = action(
        "Edit Script",
        Some("e"),
        sunbeam_rust::types::ActionType::Edit,
        None, None, None, None,
        Some(sunbeam_rust::types::EditAction {
            path: "/home/user/extensions/my-script.sh".into(),
            exit: Some(false),
            reload: Some(true),
        }),
        None, None,
    );
    print_action(&edit_action);

    // ── 6. Config: open extension preferences ───────────────────────────
    println!("=== 6. Config ===");
    let config_action = action(
        "Configure",
        Some("s"),
        sunbeam_rust::types::ActionType::Config,
        None, None, None, None, None,
        Some(sunbeam_rust::types::ConfigAction {
            extension: "gh".into(),
        }),
        None,
    );
    print_action(&config_action);

    // ── 7. Reload: re-fetch the current page ────────────────────────────
    println!("=== 7. Reload ===");
    let mut reload_params = serde_json::Map::new();
    reload_params.insert("page".into(), serde_json::json!(2));
    let reload_action = action(
        "Next Page",
        Some("n"),
        sunbeam_rust::types::ActionType::Reload,
        None, None, None, None, None, None,
        Some(sunbeam_rust::types::ReloadAction {
            params: Some(reload_params),
        }),
    );
    print_action(&reload_action);

    // ── 8. Exit: quit sunbeam ──────────────────────────────────────────
    println!("=== 8. Exit ===");
    let exit_action = action(
        "Quit",
        Some("q"),
        sunbeam_rust::types::ActionType::Exit,
        None, None, None, None, None, None, None,
    );
    print_action(&exit_action);

    // ── Verify all type discriminator tags ──────────────────────────────
    println!("=== Verifying type tags ===");
    let checks: [(&str, &sunbeam_rust::types::Action, &str); 10] = [
        ("open_url", &open_url, "open"),
        ("open_file", &open_file, "open"),
        ("copy", &copy_action, "copy"),
        ("run_same", &run_same, "run"),
        ("run_cross", &run_cross, "run"),
        ("exec_interactive", &exec_interactive, "exec"),
        ("edit", &edit_action, "edit"),
        ("config", &config_action, "config"),
        ("reload", &reload_action, "reload"),
        ("exit", &exit_action, "exit"),
    ];

    for (name, action, expected_type) in &checks {
        let json: serde_json::Value =
            serde_json::to_value(action).expect("serialize action");
        let actual = json["type"].as_str().unwrap_or("");
        assert_eq!(
            actual, *expected_type,
            "{name}: expected type={expected_type}, got={actual}"
        );
        println!("  {name}: type={actual} ✅");
    }

    println!("\n✅ All 8 Action variants created and verified!");
}

/// Helper to construct an Action without typing `None` on every field.
#[allow(clippy::too_many_arguments)]
fn action(
    title: &str,
    key: Option<&str>,
    action_type: sunbeam_rust::types::ActionType,
    open: Option<sunbeam_rust::types::OpenAction>,
    copy: Option<sunbeam_rust::types::CopyAction>,
    run: Option<sunbeam_rust::types::RunAction>,
    exec: Option<sunbeam_rust::types::ExecAction>,
    edit: Option<sunbeam_rust::types::EditAction>,
    config: Option<sunbeam_rust::types::ConfigAction>,
    reload: Option<sunbeam_rust::types::ReloadAction>,
) -> sunbeam_rust::types::Action {
    sunbeam_rust::types::Action {
        title: Some(title.into()),
        key: key.map(std::string::ToString::to_string),
        action_type,
        open,
        copy,
        run,
        exec,
        edit,
        config,
        reload,
    }
}

fn print_action(action: &sunbeam_rust::types::Action) {
    let json = serde_json::to_string_pretty(action).unwrap();
    println!("{}", json);
    println!();
}
