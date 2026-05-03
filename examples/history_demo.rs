//! Demonstrates the History system: loading, recording item usage, sorting
//! items by frequency, and persisting to disk.
//!
//! Works entirely with a temporary file — no user history is touched.
//!
//! Usage: cargo run --example history_demo

fn main() {
    use sunbeam_rust::history::{History, history_path};
    use sunbeam_rust::types::ListItem;
    use std::path::PathBuf;

    // ── 1. Default path ────────────────────────────────────────────────
    println!("=== 1. History Path ===");
    let default = history_path();
    println!("  Default path: {}", default.display());
    assert!(
        default.to_string_lossy().contains("sunbeam"),
        "path should contain 'sunbeam'"
    );

    // ── 2. Load (create empty) ─────────────────────────────────────────
    println!("\n=== 2. Load Empty History ===");
    let tmp_path = std::env::temp_dir().join("sunbeam-history-test.json");
    let mut history = History::load(&tmp_path).expect("load empty history");
    println!("  Loaded: OK (empty)");

    // ── 3. Record usage ────────────────────────────────────────────────
    println!("\n=== 3. Record Usage ===");
    history.update("item-b");
    std::thread::sleep(std::time::Duration::from_millis(10));
    history.update("item-a");
    std::thread::sleep(std::time::Duration::from_millis(10));
    history.update("item-c");
    std::thread::sleep(std::time::Duration::from_millis(10));
    history.update("item-a"); // item-a used twice, most recent
    println!("  Recorded 4 usage events");

    // ── 4. Sort items ──────────────────────────────────────────────────
    println!("\n=== 4. Sort by Usage ===");
    let items = vec![
        ListItem {
            id: Some("item-a".into()),
            title: "Item A".into(),
            subtitle: None, detail: None, accessories: None, actions: None,
        },
        ListItem {
            id: Some("item-b".into()),
            title: "Item B".into(),
            subtitle: None, detail: None, accessories: None, actions: None,
        },
        ListItem {
            id: Some("item-c".into()),
            title: "Item C".into(),
            subtitle: None, detail: None, accessories: None, actions: None,
        },
        ListItem {
            id: Some("item-d".into()),
            title: "Item D (new)".into(),
            subtitle: None, detail: None, accessories: None, actions: None,
        },
    ];

    println!("  Before sort:");
    for item in &items {
        println!("    {}", item.title);
    }

    let mut sorted = items.clone();
    history.sort(&mut sorted);

    println!("  After sort (by MRU):");
    for item in &sorted {
        println!("    {}", item.title);
    }

    // item-a (used twice, most recent) should be first
    assert_eq!(sorted[0].title, "Item A", "most used item should be first");

    // ── 5. Save ────────────────────────────────────────────────────────
    println!("\n=== 5. Save ===");
    history.save().expect("save history");
    assert!(tmp_path.exists(), "history file should exist");
    println!("  Saved to: {}", tmp_path.display());

    // ── 6. Reload ──────────────────────────────────────────────────────
    println!("\n=== 6. Reload from File ===");
    let reloaded = History::load(&tmp_path).expect("reload history");
    let mut items_to_sort = items.clone();
    reloaded.sort(&mut items_to_sort);
    assert_eq!(items_to_sort[0].title, "Item A", "persisted sort should work");
    println!("  Reloaded and sorted: OK");

    // ── 7. Cleanup ─────────────────────────────────────────────────────
    println!("\n=== 7. Cleanup ===");
    std::fs::remove_file(&tmp_path).ok();
    assert!(!tmp_path.exists(), "temp file should be removed");
    println!("  Removed: {}", tmp_path.display());

    println!("\n✅ History demo complete!");
}
