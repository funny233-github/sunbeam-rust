//! Demonstrates ExtensionConfig management: building configurations with
//! remote/local origins, preferences, root items, and serde round-tripping.
//!
//! Usage: cargo run --example extension_config_management

fn main() {
    use sunbeam_rust::config::{ExtensionConfig, RootItem};
    use std::collections::HashMap;

    // ── 1. Remote extension ────────────────────────────────────────────
    println!("=== 1. Remote Extension Config ===");
    let remote = ExtensionConfig {
        origin: "https://github.com/user/sunbeam-gh/raw/main/gh.ts".into(),
        preferences: Some({
            let mut m = serde_json::Map::new();
            m.insert("token".into(), serde_json::json!("ghp_xxxxxxxxxxxx"));
            m.insert("base_url".into(), serde_json::json!("https://api.github.com"));
            m
        }),
        root: Some(vec![
            RootItem {
                title: "My Issues".into(),
                command: "issue-list".into(),
                params: Some({
                    let mut m = serde_json::Map::new();
                    m.insert("state".into(), serde_json::json!("open"));
                    m
                }),
            },
        ]),
    };
    print_ext_cfg("Remote Extension", &remote);

    // ── 2. Local extension ─────────────────────────────────────────────
    println!("=== 2. Local Extension Config ===");
    let local = ExtensionConfig {
        origin: "~/scripts/my-extension.sh".into(),
        preferences: None,
        root: None,
    };
    print_ext_cfg("Local Extension", &local);

    // ── 3. Extension with only preferences ─────────────────────────────
    println!("=== 3. Preferences-Only Extension ===");
    let prefs_only = ExtensionConfig {
        origin: "~/tools/backup.py".into(),
        preferences: Some({
            let mut m = serde_json::Map::new();
            m.insert("backup_dir".into(), serde_json::json!("/mnt/backup"));
            m.insert("compress".into(), serde_json::json!(true));
            m.insert("retention_days".into(), serde_json::json!(30));
            m
        }),
        root: None,
    };
    print_ext_cfg("Preferences Only", &prefs_only);

    // ── 4. Extension with root items only ──────────────────────────────
    println!("=== 4. Root-Items-Only Extension ===");
    let root_only = ExtensionConfig {
        origin: "./tools/deploy.sh".into(),
        preferences: None,
        root: Some(vec![
            RootItem {
                title: "Deploy Staging".into(),
                command: "deploy".into(),
                params: Some({
                    let mut m = serde_json::Map::new();
                    m.insert("env".into(), serde_json::json!("staging"));
                    m
                }),
            },
            RootItem {
                title: "Deploy Production".into(),
                command: "deploy".into(),
                params: Some({
                    let mut m = serde_json::Map::new();
                    m.insert("env".into(), serde_json::json!("production"));
                    m.insert("confirm".into(), serde_json::json!(true));
                    m
                }),
            },
        ]),
    };
    print_ext_cfg("Root Items Only", &root_only);

    // ── 5. Serialize/Deserialize roundtrip ─────────────────────────────
    println!("=== 5. Serde Roundtrip ===");
    let json = serde_json::to_string_pretty(&remote).unwrap();
    let deserialized: ExtensionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.origin, remote.origin);
    assert_eq!(
        deserialized.preferences.as_ref().and_then(|p| p.get("token")),
        remote.preferences.as_ref().and_then(|p| p.get("token"))
    );
    assert_eq!(
        deserialized.root.as_ref().map(|r| r.len()),
        remote.root.as_ref().map(|r| r.len())
    );
    println!("  Roundtrip: OK");

    // ── 6. HashMap management ──────────────────────────────────────────
    println!("\n=== 6. Extension Config Map ===");
    let mut map: HashMap<String, ExtensionConfig> = HashMap::new();
    map.insert("gh".to_string(), remote);
    map.insert("backup".to_string(), prefs_only);
    map.insert("deploy".to_string(), root_only);
    map.insert("local".to_string(), local);

    println!("  Extensions: {}", map.len());
    for (alias, cfg) in &map {
        println!("    {} → {} ({} preferences)",
            alias,
            cfg.origin,
            cfg.preferences.as_ref().map(|p| p.len()).unwrap_or(0));
    }

    println!("\n✅ Extension config management demo complete!");
}

fn print_ext_cfg(label: &str, cfg: &sunbeam_rust::config::ExtensionConfig) {
    let json = serde_json::to_string_pretty(cfg).unwrap();
    println!("  {label}:");
    for line in json.lines() {
        println!("    {line}");
    }
    let has_prefs = cfg.preferences.is_some();
    let has_root = cfg.root.is_some();
    println!("  (preferences: {has_prefs}, root items: {has_root})\n");
}
