//! Demonstrates error handling in the extension system: missing required
//! preferences/params, nonexistent commands, and invalid output.
//!
//! Uses the greet.sh extension which requires a "name" param for the greet
//! command and has an optional "greeting" preference.
//!
//! Usage: cargo run --example extension_error_handling

fn main() {
    let ext_path = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("examples/scripts/greet.sh");
    let ext = sunbeam_rust::extensions::load_extension(&ext_path.to_string_lossy())
        .expect("load extension");

    println!("=== 1. Nonexistent command ===");
    let payload = sunbeam_rust::types::Payload {
        command: "nonexistent".into(),
        preferences: None,
        params: None,
        cwd: None,
        r#query: None,
    };
    match ext.cmd(&payload) {
        Ok(_) => println!("  ❌ Should have failed!"),
        Err(e) => println!("  ✅ Expected error: {e}"),
    }

    println!("\n=== 2. Missing required preference ===");
    // Manifest has no required preferences, so this should work
    let payload = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: None,
        params: Some({
            let mut m = serde_json::Map::new();
            m.insert("name".into(), serde_json::json!("World"));
            m
        }),
        cwd: None,
        r#query: None,
    };
    match ext.cmd(&payload) {
        Ok(_) => println!("  ✅ No required prefs — OK"),
        Err(e) => println!("  ❌ Unexpected error: {e}"),
    }

    println!("\n=== 3. Missing required command param ===");
    // "greet" requires "name" param with no default
    let payload_no_name = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: None,
        params: None,
        cwd: None,
        r#query: None,
    };
    match ext.cmd(&payload_no_name) {
        Ok(_) => println!("  ❌ Should have failed (name is required)"),
        Err(e) => println!("  ✅ Expected error: {e}"),
    }

    println!("\n=== 4. Required param with default value ===");
    // greet.sh has "name" as required with no default — simulate by providing it
    let params = {
        let mut m = serde_json::Map::new();
        m.insert("name".into(), serde_json::json!("Test"));
        m
    };
    let payload_with_name = sunbeam_rust::types::Payload {
        command: "greet".into(),
        preferences: None,
        params: Some(params),
        cwd: None,
        r#query: None,
    };
    match ext.cmd(&payload_with_name) {
        Ok(cmd) => println!("  ✅ Command built successfully: {:?}", cmd.get_program()),
        Err(e) => println!("  ❌ Unexpected error: {e}"),
    }

    println!("\n=== 5. Invalid output (not valid List JSON) ===");
    match ext.run(&payload_no_name) {
        Ok(output) => {
            // The extension will print an error to stderr but might still exit 0
            println!("  Output ({} bytes): {}", output.len(), String::from_utf8_lossy(&output).lines().next().unwrap_or(""));
        }
        Err(e) => println!("  ✅ Expected extension error: {e}"),
    }

    println!("\n=== 6. Extension with invalid origin ===");
    match sunbeam_rust::extensions::load_extension("/nonexistent/path/to/script.sh") {
        Ok(_) => println!("  ❌ Should have failed!"),
        Err(e) => println!("  ✅ Expected error: {e}"),
    }

    println!("\n=== 7. Command lookup returns None for missing command ===");
    let cmd = ext.command("does_not_exist");
    match cmd {
        Some(_) => println!("  ❌ Should not have found command"),
        None => println!("  ✅ command() returned None as expected"),
    }

    println!("\n=== 8. hash_origin for nonexistent local path ===");
    let result = sunbeam_rust::extensions::hash_origin("/nonexistent/path");
    match result {
        Ok(hash) => println!("  Hash: {hash} (canonicalization failed but hash still computed)"),
        Err(e) => println!("  Error: {e}"),
    }

    println!("\n✅ Extension error handling demo complete!");
}
