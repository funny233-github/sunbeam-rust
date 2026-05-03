//! Demonstrates a complete sunbeam workflow end-to-end:
//!
//! 1. Create a temporary config with oneliners and an extension reference
//! 2. Resolve and load the config
//! 3. Load the greet.sh extension from examples/scripts
//! 4. Build root items from the config
//! 5. Run a filter-mode extension command, parse List output
//! 6. Run a detail-mode extension command, parse Detail output
//! 7. Use history to sort items
//! 8. Validate all outputs against JSON schemas
//! 9. Clean up temporary files
//!
//! Usage: cargo run --example full_workflow

use std::path::PathBuf;

fn main() {
    let greet_path = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("examples/scripts/greet.sh");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 1: Create a temporary config
    // ═══════════════════════════════════════════════════════════════════
    println!("=== STEP 1: Create Config ===");
    let tmp_config = std::env::temp_dir().join("sunbeam-workflow-config.json");
    let cfg = sunbeam_rust::config::Config {
        oneliners: Some(vec![
            sunbeam_rust::config::Oneliner {
                title: "Hello".into(),
                command: "echo hello".into(),
                interactive: None, cwd: None, exit: None,
            },
        ]),
        extensions: Some(
            vec![("greet".to_string(), sunbeam_rust::config::ExtensionConfig {
                origin: greet_path.to_string_lossy().to_string(),
                preferences: None,
                root: None,
            })].into_iter().collect()
        ),
        oneliner: None,
        path: tmp_config.clone(),
    };
    cfg.save().expect("save config");
    println!("  Config saved: {}", tmp_config.display());

    // ═══════════════════════════════════════════════════════════════════
    // STEP 2: Load config
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 2: Load Config ===");
    let loaded = sunbeam_rust::config::load(&tmp_config).expect("load config");
    let oneliner_count = loaded.oneliners.as_ref().map(|o| o.len()).unwrap_or(0);
    let ext_count = loaded.extensions.as_ref().map(|e| e.len()).unwrap_or(0);
    println!("  Loaded: {oneliner_count} oneliners, {ext_count} extensions");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 3: Load extension
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 3: Load Extension ===");
    let ext_alias = "greet";
    let ext_cfg = loaded.extensions.as_ref()
        .and_then(|e| e.get(ext_alias))
        .expect("extension config found");
    let ext = sunbeam_rust::extensions::load_extension(&ext_cfg.origin)
        .expect("load extension");
    println!("  Extension: {} — {}", ext_alias, ext.manifest.title);
    println!("  Commands: {}", ext.manifest.commands.len());

    // ═══════════════════════════════════════════════════════════════════
    // STEP 4: Run filter mode command
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 4: Run Filter Command ===");
    let filter_payload = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: Some({
            let mut m = serde_json::Map::new();
            m.insert("greeting".into(), serde_json::json!("Hi"));
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

    let filter_output = ext.run(&filter_payload).expect("run filter command");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 5: Validate and parse List
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 5: Validate + Parse List ===");
    sunbeam_rust::schemas::validate_list(&filter_output)
        .expect("list validation passed");
    let list: sunbeam_rust::types::List =
        serde_json::from_slice(&filter_output).expect("parse List");
    println!("  Items: {}", list.items.as_ref().map(|i| i.len()).unwrap_or(0));
    for item in list.items.as_ref().unwrap_or(&vec![]) {
        println!("    - {} ({})", item.title, item.subtitle.as_deref().unwrap_or(""));
    }

    // ═══════════════════════════════════════════════════════════════════
    // STEP 6: Run detail mode command
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 6: Run Detail Command ===");
    let detail_payload = sunbeam_rust::types::Payload {
        command: "preview".into(),
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

    let detail_output = ext.run(&detail_payload).expect("run detail command");

    // ═══════════════════════════════════════════════════════════════════
    // STEP 7: Validate and parse Detail
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 7: Validate + Parse Detail ===");
    sunbeam_rust::schemas::validate_detail(&detail_output)
        .expect("detail validation passed");
    let detail: sunbeam_rust::types::Detail =
        serde_json::from_slice(&detail_output).expect("parse Detail");
    println!("  Markdown: {} chars", detail.markdown.as_ref().map(|m| m.len()).unwrap_or(0));
    println!("  Actions: {}", detail.actions.as_ref().map(|a| a.len()).unwrap_or(0));

    // ═══════════════════════════════════════════════════════════════════
    // STEP 8: History sort demo
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 8: History Sort ===");
    let tmp_history = std::env::temp_dir().join("sunbeam-workflow-history.json");
    let mut history = sunbeam_rust::history::History::load(&tmp_history)
        .expect("load history");
    history.update("greet - greet");
    history.update("oneliner - Hello");
    history.save().expect("save history");

    let items = vec![
        sunbeam_rust::types::ListItem {
            id: Some("oneliner - Hello".into()),
            title: "Hello".into(),
            subtitle: None, detail: None, accessories: None, actions: None,
        },
        sunbeam_rust::types::ListItem {
            id: Some("greet - greet".into()),
            title: "Greet".into(),
            subtitle: None, detail: None, accessories: None, actions: None,
        },
    ];
    let mut sorted = items.clone();
    history.sort(&mut sorted);
    println!("  Sorted items: {:?}", sorted.iter().map(|i| i.title.as_str()).collect::<Vec<_>>());

    // ═══════════════════════════════════════════════════════════════════
    // STEP 9: Cleanup
    // ═══════════════════════════════════════════════════════════════════
    println!("\n=== STEP 9: Cleanup ===");
    std::fs::remove_file(&tmp_config).ok();
    std::fs::remove_file(&tmp_history).ok();
    println!("  Temp files cleaned up");

    println!("\n✅ Full workflow demo complete!");
}
