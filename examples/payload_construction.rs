//! Demonstrates building Payload objects for various extension command
//! scenarios and using the Extension::cmd / Extension::run methods.
//!
//! Uses the `greet.sh` example script to run real extension invocations.
//!
//! Usage: cargo run --example payload_construction

use std::collections::BTreeMap;

fn main() {
    let ext_path = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("examples/scripts/greet.sh");

    println!("Loading extension from: {}", ext_path.display());
    let ext = sunbeam_rust::extensions::load_extension(&ext_path.to_string_lossy())
        .expect("failed to load greet.sh extension");
    println!("Extension title: {}", ext.manifest.title);
    println!();

    // ── 1. Minimal Payload (command only) ───────────────────────────────
    println!("=== 1. Minimal Payload ===");
    let payload_minimal = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: None,
        params: None,
        cwd: None,
        r#query: None,
    };
    print_payload("minimal", &payload_minimal);
    println!();

    // ── 2. Payload with params ──────────────────────────────────────────
    println!("=== 2. Payload with params ===");
    let mut params = serde_json::Map::new();
    params.insert("name".into(), serde_json::json!("World"));
    let payload_with_params = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: None,
        params: Some(params),
        cwd: None,
        r#query: None,
    };
    print_payload("with params", &payload_with_params);
    println!();

    // ── 3. Payload with preferences ─────────────────────────────────────
    println!("=== 3. Payload with preferences ===");
    let mut prefs = serde_json::Map::new();
    prefs.insert("greeting".into(), serde_json::json!("Hey"));
    let payload_with_prefs = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: Some(prefs),
        params: Some({
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!("Alice"));
            m
        }),
        cwd: None,
        r#query: None,
    };
    print_payload("with preferences", &payload_with_prefs);
    println!();

    // ── 4. Payload with query (search mode) ─────────────────────────────
    println!("=== 4. Payload with query (search mode) ===");
    let payload_with_query = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: None,
        params: Some({
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!("Bob"));
            m
        }),
        cwd: Some("/tmp".into()),
        r#query: Some("bob".into()),
    };
    print_payload("with query", &payload_with_query);
    println!();

    // ── 5. Using Extension::cmd() to inspect the built command ──────────
    println!("=== 5. Using extension.cmd() ===");
    let built_cmd = ext
        .cmd(&payload_with_params)
        .expect("failed to build command");
    println!("  Program: {:?}", built_cmd.get_program());
    let cmd_str: String = built_cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    println!("  Args: {}", &cmd_str[..cmd_str.len().min(200)]);
    println!();

    // ── 6. Using Extension::run() to execute and capture output ─────────
    println!("=== 6. Using extension.run() ===");
    let output = ext
        .run(&payload_with_params)
        .expect("failed to run extension");
    let output_str = String::from_utf8_lossy(&output);
    println!("  stdout ({} bytes):", output.len());
    println!("  {}", output_str);
    println!();

    // ── 7. Parse output as List ─────────────────────────────────────────
    println!("=== 7. Parsing output as List ===");
    let list: sunbeam_rust::types::List =
        serde_json::from_slice(&output).expect("expected List JSON");
    if let Some(items) = &list.items {
        for item in items {
            println!("  - {} ({})", item.title, item.subtitle.as_deref().unwrap_or(""));
        }
    }
    println!();

    // ── 8. Detail mode (preview command) ────────────────────────────────
    println!("=== 8. Detail mode (preview command) ===");
    let detail_payload = sunbeam_rust::types::Payload {
        command: "preview".into(),
        preferences: None,
        params: Some({
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!("World"));
            m
        }),
        cwd: None,
        r#query: None,
    };
    let detail_output = ext.run(&detail_payload).expect("failed to run preview");
    let detail: sunbeam_rust::types::Detail =
        serde_json::from_slice(&detail_output).expect("expected Detail JSON");
    println!(
        "  markdown: {}",
        detail.markdown.as_deref().unwrap_or("(none)")
    );
    println!(
        "  actions: {}",
        detail
            .actions
            .as_ref()
            .map(|a| a.len().to_string())
            .unwrap_or_else(|| "0".into())
    );
    println!();

    // ── 9. Param types demonstration ────────────────────────────────────
    println!("=== 9. Various param types ===");
    let multi_params = build_typed_params();
    let payload_typed = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: None,
        params: Some(multi_params),
        cwd: None,
        r#query: None,
    };
    // Just show the serialization; the extension will ignore unexpected params
    let json = serde_json::to_string_pretty(&payload_typed).unwrap();
    println!("{}", json);

    println!("\n✅ Payload construction demo complete!");
}

fn print_payload(label: &str, payload: &sunbeam_rust::types::Payload) {
    let json = serde_json::to_string_pretty(payload).unwrap();
    println!("  {label}:");
    for line in json.lines() {
        println!("    {line}");
    }
}

fn build_typed_params() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    // string
    m.insert("name".into(), serde_json::json!("Rust"));
    // boolean
    m.insert("verbose".into(), serde_json::json!(true));
    // number
    m.insert("count".into(), serde_json::json!(42));
    m
}
