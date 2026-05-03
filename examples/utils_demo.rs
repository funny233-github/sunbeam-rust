//! Demonstrates the utility functions: config/cache directory resolution,
//! editor detection, path normalization, and alias extraction.
//!
//! Usage: cargo run --example utils_demo

fn main() {
    use sunbeam_rust::utils;

    // ── 1. Directory resolution ────────────────────────────────────────
    println!("=== 1. Directory Resolution ===");
    let config_dir = utils::config_dir();
    println!("  Config dir: {}", config_dir.display());
    assert!(
        config_dir.to_string_lossy().contains("sunbeam"),
        "config dir should contain 'sunbeam'"
    );

    let cache_dir = utils::cache_dir();
    println!("  Cache dir: {}", cache_dir.display());
    assert!(
        cache_dir.to_string_lossy().contains("sunbeam"),
        "cache dir should contain 'sunbeam'"
    );

    // XDG overrides
    std::env::set_var("XDG_CONFIG_HOME", "/custom/config");
    std::env::set_var("XDG_CACHE_HOME", "/custom/cache");
    let xdg_config = utils::config_dir();
    let xdg_cache = utils::cache_dir();
    println!("  XDG_CONFIG_HOME: config dir = {}", xdg_config.display());
    println!("  XDG_CACHE_HOME:  cache dir = {}", xdg_cache.display());
    assert_eq!(xdg_config, std::path::PathBuf::from("/custom/config/sunbeam"));
    assert_eq!(xdg_cache, std::path::PathBuf::from("/custom/cache/sunbeam"));
    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::remove_var("XDG_CACHE_HOME");
    println!();

    // ── 2. Editor detection ────────────────────────────────────────────
    println!("=== 2. Editor Detection ===");
    std::env::set_var("VISUAL", "nvim");
    assert_eq!(utils::find_editor(), "nvim");
    println!("  VISUAL=nvim → editor = {}", utils::find_editor());
    std::env::remove_var("VISUAL");

    std::env::set_var("EDITOR", "code --wait");
    assert_eq!(utils::find_editor(), "code --wait");
    println!("  EDITOR=code --wait → editor = {}", utils::find_editor());
    std::env::remove_var("EDITOR");

    // Fallback to vi
    let editor = utils::find_editor();
    println!("  No env → editor = {}", editor);
    assert_eq!(editor, "vi");
    println!();

    // ── 3. Origin normalization ────────────────────────────────────────
    println!("=== 3. Origin Normalization ===");
    assert_eq!(
        utils::normalize_origin("https://example.com/ext.ts").unwrap(),
        "https://example.com/ext.ts"
    );
    println!("  URL origin preserved: https://example.com/ext.ts");

    let nonexistent = utils::normalize_origin("/nonexistent/path.sh");
    match nonexistent {
        Ok(_) => println!("  ❌ Should have failed for nonexistent path"),
        Err(e) => println!("  ✅ Nonexistent path error: {e}"),
    }
    println!();

    // ── 4. Alias extraction ────────────────────────────────────────────
    println!("=== 4. Alias Extraction ===");
    let test_cases = vec![
        ("https://example.com/my-extension.py", "my-extension"),
        ("https://raw.githubusercontent.com/user/repo/main/gh.ts", "gh"),
        ("https://example.com/tools/deploy.sh", "deploy"),
        ("file:///home/user/extensions/my-tool.sh", "my-tool"),
    ];

    for (url, expected) in &test_cases {
        let alias = utils::extract_alias(url).unwrap_or_else(|_| "?".into());
        let status = if alias == *expected { "✅" } else { "❌" };
        println!("  {status} {url}");
        println!("      alias: {alias} (expected: {expected})");
    }

    println!("\n✅ Utils demo complete!");
}
