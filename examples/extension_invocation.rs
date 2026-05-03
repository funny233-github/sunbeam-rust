//! Demonstrates a complete extension invocation workflow: loading a script,
//! invoking it in filter mode with params, parsing the List output, then
//! invoking in detail mode with another param set, parsing the Detail output.
//!
//! Uses the greet.sh example script.
//!
//! Usage: cargo run --example extension_invocation

fn main() {
    let ext_path = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("examples/scripts/greet.sh");

    // ── Phase 1: Load extension ─────────────────────────────────────────
    println!("=== Phase 1: Load Extension ===");
    let ext = sunbeam_rust::extensions::load_extension(&ext_path.to_string_lossy())
        .expect("failed to load extension");
    println!("  Title: {}", ext.manifest.title);
    println!("  Commands: {}", ext.manifest.commands.len());
    println!();

    // ── Phase 2: Filter mode invocation ─────────────────────────────────
    println!("=== Phase 2: Filter Mode (greet) ===");
    let greet_payload = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: Some({
            let mut m = serde_json::Map::new();
            m.insert("greeting".into(), serde_json::json!("Hello"));
            m
        }),
        params: Some({
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!("World"));
            m
        }),
        cwd: None,
        r#query: None,
    };

    let output = ext.run(&greet_payload).expect("run greet command");
    println!("  Raw stdout ({} bytes):", output.len());
    let output_str = String::from_utf8_lossy(&output);
    for line in output_str.lines() {
        println!("    {line}");
    }
    println!();

    // ── Phase 3: Parse List output ──────────────────────────────────────
    println!("=== Phase 3: Parse List ===");
    let list: sunbeam_rust::types::List =
        serde_json::from_slice(&output).expect("expected List JSON");
    if let Some(items) = &list.items {
        println!("  Items ({}):", items.len());
        for item in items {
            println!("    - {} ({})", item.title, item.subtitle.as_deref().unwrap_or(""));
            if let Some(accessories) = &item.accessories {
                println!("      accessories: {:?}", accessories);
            }
            if let Some(actions) = &item.actions {
                for action in actions {
                    println!("      action: [{}] {} ({:?})",
                        action.key.as_deref().unwrap_or(""),
                        action.title.as_deref().unwrap_or(""),
                        action.action_type);
                }
            }
            println!();
        }
    }
    println!("  empty_text: {:?}", list.empty_text);
    println!("  show_detail: {:?}", list.show_detail);
    println!();

    // ── Phase 4: Detail mode invocation ─────────────────────────────────
    println!("=== Phase 4: Detail Mode (preview) ===");
    let detail_payload = sunbeam_rust::types::Payload {
        command: "preview".into(),
        preferences: Some({
            let mut m = serde_json::Map::new();
            m.insert("greeting".into(), serde_json::json!("Howdy"));
            m
        }),
        params: Some({
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!("Partner"));
            m
        }),
        cwd: None,
        r#query: None,
    };

    let detail_output = ext.run(&detail_payload).expect("run preview command");
    let detail: sunbeam_rust::types::Detail =
        serde_json::from_slice(&detail_output).expect("expected Detail JSON");
    println!("  markdown: {}", detail.markdown.as_deref().unwrap_or("(none)"));
    println!("  text: {:?}", detail.text);
    if let Some(actions) = &detail.actions {
        println!("  actions: {}", actions.len());
        for action in actions {
            println!("    - {} ({:?})", action.title.as_deref().unwrap_or(""), action.action_type);
        }
    }
    println!();

    // ── Phase 5: Different preference value ─────────────────────────────
    println!("=== Phase 5: Custom Preference ===");
    let custom_prefs = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: Some({
            let mut m = serde_json::Map::new();
            m.insert("greeting".into(), serde_json::json!("Bonjour"));
            m
        }),
        params: Some({
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!("Monde"));
            m
        }),
        cwd: None,
        r#query: None,
    };
    let custom_output = ext.run(&custom_prefs).expect("run with custom prefs");
    let custom_list: sunbeam_rust::types::List =
        serde_json::from_slice(&custom_output).expect("expected List");
    if let Some(items) = &custom_list.items {
        for item in items {
            println!("  {}", item.title);
        }
    }

    println!("\n✅ Extension invocation demo complete!");
}
