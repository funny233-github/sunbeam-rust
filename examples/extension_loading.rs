//! Demonstrates loading an extension, inspecting its manifest, and
//! navigating its commands.
//!
//! Uses the greet.sh example script which provides a filter command (greet)
//! and a hidden detail command (preview) with preferences.
//!
//! Usage: cargo run --example extension_loading

fn main() {
    let ext_path = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("examples/scripts/greet.sh");

    println!("=== 1. Loading Extension ===");
    println!("  Path: {}", ext_path.display());
    let ext = sunbeam_rust::extensions::load_extension(&ext_path.to_string_lossy())
        .expect("failed to load extension");
    println!("  Entrypoint: {}", ext.entrypoint.display());
    println!("  Loaded: OK\n");

    // ── 2. Manifest Inspection ─────────────────────────────────────────
    println!("=== 2. Manifest ===");
    println!("  Title: {}", ext.manifest.title);
    println!("  Description: {:?}", ext.manifest.description);
    println!(
        "  Preferences: {}",
        ext.manifest.preferences.as_ref().map(|p| p.len()).unwrap_or(0)
    );
    if let Some(prefs) = &ext.manifest.preferences {
        for pref in prefs {
            println!("    - {} ({}): {:?}, optional: {:?}, default: {:?}",
                pref.name, pref.title, pref.input_type, pref.optional, pref.default);
        }
    }
    println!("  Commands: {}", ext.manifest.commands.len());
    println!();

    // ── 3. Command Inspection ──────────────────────────────────────────
    println!("=== 3. Commands ===");
    for cmd in &ext.manifest.commands {
        println!("  {} — {} (mode: {:?}, hidden: {})",
            cmd.name,
            cmd.title,
            cmd.mode,
            cmd.hidden.unwrap_or(false));
        if let Some(params) = &cmd.params {
            for param in params {
                println!("    param: {} ({:?}), optional: {:?}", param.name, param.input_type, param.optional);
            }
        }
    }
    println!();

    // ── 4. Extension::root_commands() — filters out hidden commands ────
    println!("=== 4. Root Commands (non-hidden) ===");
    let root_cmds = ext.root_commands();
    println!("  Count: {}", root_cmds.len());
    for cmd in &root_cmds {
        println!("    {} — {}", cmd.name, cmd.title);
    }
    println!();

    // ── 5. Extension::command() — lookup by name ───────────────────────
    println!("=== 5. Command Lookup ===");
    let greet_cmd = ext.command("greet").expect("greet command should exist");
    println!("  greet command: {} (mode: {:?})", greet_cmd.title, greet_cmd.mode);

    let preview_cmd = ext.command("preview").expect("preview command should exist");
    println!("  preview command: {} (mode: {:?}, hidden: {})",
        preview_cmd.title, preview_cmd.mode, preview_cmd.hidden.unwrap_or(false));

    let nonexistent = ext.command("nonexistent");
    println!("  nonexistent command: {:?}", nonexistent);
    assert!(nonexistent.is_none(), "nonexistent command should return None");
    println!();

    // ── 6. Extension API utilities ─────────────────────────────────────
    println!("=== 6. Extension Utilities ===");
    let origin = &ext_path.to_string_lossy().to_string();
    println!("  is_remote('{origin}'): {}", sunbeam_rust::extensions::is_remote(origin));
    println!("  is_remote('https://...'): {}", sunbeam_rust::extensions::is_remote("https://example.com/ext.ts"));

    let hash = sunbeam_rust::extensions::hash_origin(origin).expect("hash origin");
    println!("  hash_origin('{origin}'): {hash}");

    let remote_hash = sunbeam_rust::extensions::hash_origin("https://example.com/ext.ts")
        .expect("hash remote origin");
    println!("  hash_origin('https://...'): {remote_hash}");

    println!("\n✅ Extension loading demo complete!");
}
