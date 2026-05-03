//! Demonstrates the full Config lifecycle: path resolution, loading, saving,
//! extension management, and path resolution utilities.
//!
//! Works entirely with temporary files — no user config is touched.
//!
//! Usage: cargo run --example config_lifecycle

use std::path::PathBuf;

fn main() {
    use sunbeam_rust::config::{Config, ExtensionConfig, Oneliner, resolve_config_path};

    // ── 1. Path resolution strategies ───────────────────────────────────
    println!("=== 1. Config Path Resolution ===");

    // Default: ~/.config/sunbeam/sunbeam.json
    let default_path = resolve_config_path();
    println!("  Default: {}", default_path.display());

    // Override with SUNBEAM_CONFIG env var
    std::env::set_var("SUNBEAM_CONFIG", "/tmp/custom-sunbeam.json");
    let forced_path = resolve_config_path();
    assert_eq!(forced_path, PathBuf::from("/tmp/custom-sunbeam.json"));
    println!("  Forced (SUNBEAM_CONFIG): {}", forced_path.display());
    std::env::remove_var("SUNBEAM_CONFIG");

    // ── 2. Build and save a config ──────────────────────────────────────
    println!("\n=== 2. Building and Saving Config ===");
    let tmp_path = std::env::temp_dir().join("sunbeam-example-config.json");
    let mut cfg = Config {
        oneliners: Some(vec![
            Oneliner {
                title: "Hello".into(),
                command: "echo hello".into(),
                interactive: Some(false),
                cwd: None,
                exit: None,
            },
            Oneliner {
                title: "Disk Usage".into(),
                command: "df -h".into(),
                interactive: Some(true),
                cwd: Some("/".into()),
                exit: Some(false),
            },
        ]),
        extensions: Some(
            vec![
                ("gh".to_string(), ExtensionConfig {
                    origin: "https://example.com/gh.ts".into(),
                    preferences: None,
                    root: None,
                }),
                ("notes".to_string(), ExtensionConfig {
                    origin: "~/scripts/notes.sh".into(),
                    preferences: Some({
                        let mut m = serde_json::Map::new();
                        m.insert("notes_dir".into(), serde_json::json!("/home/user/notes"));
                        m
                    }),
                    root: None,
                }),
            ].into_iter().collect()
        ),
        oneliner: None,
        path: tmp_path.clone(),
    };

    cfg.save().expect("save config");
    println!("  Saved to: {}", tmp_path.display());
    assert!(tmp_path.exists(), "file should exist after save");

    // ── 3. Load and verify ──────────────────────────────────────────────
    println!("\n=== 3. Loading Config ===");
    let loaded = sunbeam_rust::config::load(&tmp_path).expect("load config");
    assert_eq!(loaded.oneliners.as_ref().map(|o| o.len()), Some(2));
    assert_eq!(loaded.extensions.as_ref().map(|e| e.len()), Some(2));
    println!("  Loaded: {} oneliners, {} extensions",
        loaded.oneliners.as_ref().map(|o| o.len()).unwrap_or(0),
        loaded.extensions.as_ref().map(|e| e.len()).unwrap_or(0));

    // ── 4. Modify and re-save ────────────────────────────────────────────
    println!("\n=== 4. Modify and Re-save ===");
    let exts = loaded.extensions.as_ref().unwrap();
    println!("  Before: {:?}", exts.keys().collect::<Vec<_>>());

    // We need to rebuild a mutable copy
    let mut modified = loaded.clone();
    let mut exts_map = modified.extensions.unwrap_or_default();
    exts_map.insert("todo".to_string(), ExtensionConfig {
        origin: "https://example.com/todo.sh".into(),
        preferences: None,
        root: None,
    });
    exts_map.remove("notes");
    modified.extensions = Some(exts_map);
    modified.save().expect("re-save config");

    let reloaded = sunbeam_rust::config::load(&tmp_path).expect("reload config");
    let ext_keys: Vec<&String> = reloaded.extensions.as_ref().map(|e| e.keys().collect()).unwrap_or_default();
    println!("  After:  {:?}", ext_keys);
    assert!(ext_keys.contains(&&"todo".to_string()), "todo should have been added");
    assert!(!ext_keys.contains(&&"notes".to_string()), "notes should have been removed");

    // ── 5. Path resolution ──────────────────────────────────────────────
    println!("\n=== 5. Path Resolution ===");
    let home_path = modified.resolve("~/some/file.txt");
    println!("  ~/some/file.txt → {}", home_path.display());
    assert!(home_path.is_absolute());

    let rel_path = modified.resolve("relative/path.txt");
    println!("  relative/path.txt → {}", rel_path.display());
    // Should be resolved relative to config parent dir
    println!("  (config parent: {:?})", tmp_path.parent());

    // ── 6. Cleanup ──────────────────────────────────────────────────────
    println!("\n=== 6. Cleanup ===");
    std::fs::remove_file(&tmp_path).ok();
    println!("  Removed: {}", tmp_path.display());
    assert!(!tmp_path.exists(), "temp file should be removed");

    println!("\n✅ Config lifecycle demo complete!");
}
