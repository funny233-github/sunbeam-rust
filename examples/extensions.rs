fn main() {
    let ext_path = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("examples/scripts/modrinth.sh");

    // ── Phase 1: load extension, inspect manifest ──────────────────────
    println!("=== load_extension ===");
    let ext = sunbeam_rust::extensions::load_extension(&ext_path.to_string_lossy())
        .expect("failed to load extension");
    println!("manifest: {:#?}", ext.manifest);
    println!("entrypoint: {:?}", ext.entrypoint);

    println!("\n=== root_commands ===");
    for cmd in ext.root_commands() {
        println!("  {} — {} (mode: {:?})", cmd.name, cmd.title, cmd.mode);
    }

    println!("\n=== command (lookup) ===");
    let cmd = ext.command("search-project").expect("command not found");
    println!("command spec: {:#?}", cmd);

    // ── Phase 2: build the command ─────────────────────────────────────
    println!("\n=== cmd (build) ===");
    let payload = sunbeam_rust::types::Payload {
        command: "search-project".into(),
        preferences: None,
        params: Some(
            [("name".into(), serde_json::json!("World"))]
                .into_iter()
                .collect(),
        ),
        cwd: None,
        r#query: Some("fabric-api".into()),
    };
    let built = ext.cmd(&payload).expect("failed to build command");
    println!("  program: {:?}", built.get_program());
    println!("  args[0]: {:#?}", built.get_args().collect::<Vec<_>>()[0]);

    // ── Phase 3: run the greet command, parse List response ────────────
    println!("\n=== run (greet → filter → List) ===");
    let output = ext.run(&payload).expect("failed to run greet command");
    let list: sunbeam_rust::types::List =
        serde_json::from_slice(&output).expect("expected a List JSON");

    let items = list.items.as_deref().unwrap_or_default();
    println!("  item count: {}", items.len());
    for item in items {
        println!(
            "  - {} ({})",
            item.title,
            item.subtitle.as_deref().unwrap_or("")
        );
        println!("    accessories: {:?}", item.accessories);
        if let Some(actions) = &item.actions {
            for action in actions {
                println!(
                    "    action: [{}] {} ({:?})",
                    action.key.as_deref().unwrap_or(""),
                    action.title.as_deref().unwrap_or(""),
                    action.action_type
                );
            }
        }
    }
}
