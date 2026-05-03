//! Demonstrates JSON Schema validation for List, Detail, Manifest, and Config
//! payloads, showing both valid and invalid examples.
//!
//! Usage: cargo run --example schema_validation

fn main() {
    use sunbeam_rust::schemas;

    // ── 1. List Validation ─────────────────────────────────────────────
    println!("=== List Validation ===");
    let valid_list = serde_json::json!({
        "items": [
            {
                "title": "Test Item",
                "subtitle": "A test",
                "accessories": ["tag"],
                "actions": [
                    {"title": "Run", "type": "run", "run": {"command": "test"}}
                ]
            }
        ],
        "empty_text": "No items"
    });
    let valid_bytes = serde_json::to_vec(&valid_list).unwrap();
    match schemas::validate_list(&valid_bytes) {
        Ok(_) => println!("  ✅ Valid list passes"),
        Err(e) => println!("  ❌ Valid list failed: {e}"),
    }

    let invalid_list = br#"{"items": [{"title": 123}]}"#;
    match schemas::validate_list(invalid_list) {
        Ok(_) => println!("  ❌ Invalid list passed!"),
        Err(e) => println!("  ✅ Invalid list rejected: {e}"),
    }
    println!();

    // ── 2. Detail Validation ───────────────────────────────────────────
    println!("=== Detail Validation ===");
    let valid_detail = serde_json::json!({
        "markdown": "# Hello\nThis is detail content.",
        "actions": []
    });
    let valid_bytes = serde_json::to_vec(&valid_detail).unwrap();
    match schemas::validate_detail(&valid_bytes) {
        Ok(_) => println!("  ✅ Valid detail passes"),
        Err(e) => println!("  ❌ Valid detail failed: {e}"),
    }

    let invalid_detail = br#"{"markdown": 42}"#;
    match schemas::validate_detail(invalid_detail) {
        Ok(_) => println!("  ❌ Invalid detail passed!"),
        Err(e) => println!("  ✅ Invalid detail rejected: {e}"),
    }
    println!();

    // ── 3. Manifest Validation ─────────────────────────────────────────
    println!("=== Manifest Validation ===");
    let valid_manifest = serde_json::json!({
        "title": "Test Extension",
        "description": "A test",
        "commands": [
            {"name": "cmd1", "title": "Command 1", "mode": "filter"}
        ]
    });
    let valid_bytes = serde_json::to_vec(&valid_manifest).unwrap();
    match schemas::validate_manifest(&valid_bytes) {
        Ok(_) => println!("  ✅ Valid manifest passes"),
        Err(e) => println!("  ❌ Valid manifest failed: {e}"),
    }

    let invalid_manifest = br#"{"title": 42}"#;
    match schemas::validate_manifest(invalid_manifest) {
        Ok(_) => println!("  ❌ Invalid manifest passed!"),
        Err(e) => println!("  ✅ Invalid manifest rejected: {e}"),
    }
    println!();

    // ── 4. Config Validation ───────────────────────────────────────────
    println!("=== Config Validation ===");
    let valid_config = serde_json::json!({
        "oneliners": [
            {"title": "Test", "command": "echo hello"}
        ]
    });
    let valid_bytes = serde_json::to_vec(&valid_config).unwrap();
    match schemas::validate_config(&valid_bytes) {
        Ok(_) => println!("  ✅ Valid config passes"),
        Err(e) => println!("  ❌ Valid config failed: {e}"),
    }

    let invalid_config = br#"{"oneliners": "not an array"}"#;
    match schemas::validate_config(invalid_config) {
        Ok(_) => println!("  ❌ Invalid config passed!"),
        Err(e) => println!("  ✅ Invalid config rejected: {e}"),
    }
    println!();

    // ── 5. Show validation error details ───────────────────────────────
    println!("=== Detailed Error Messages ===");
    let bad_action = br#"{"actions": [{"type": "run"}]}"#;
    match schemas::validate_list(bad_action) {
        Ok(_) => println!("  ✅ (unexpected pass)"),
        Err(e) => println!("  Run action missing 'run.command':\n    {e}"),
    }

    let bad_title = br#"{"items": [{"title": "", "subtitle": "empty"}]}"#;
    match schemas::validate_list(bad_title) {
        Ok(_) => println!("  ✅ Empty title allowed"),
        Err(e) => println!("  Empty title: {e}"),
    }

    println!("\n✅ Schema validation demo complete!");
}
