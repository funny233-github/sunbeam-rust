//! Demonstrates the extension loading and caching system for local paths.
//!
//! Shows how entrypoints are resolved, cache directories are computed,
//! and manifests are stored/retrieved.
//!
//! Usage: cargo run --example extension_remote_simulation

fn main() {
    use sunbeam_rust::extensions;
    use std::path::Path;

    let ext_path = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("examples/scripts/greet.sh");
    let ext_str = ext_path.to_string_lossy().to_string();

    // ── 1. Remote detection ────────────────────────────────────────────
    println!("=== 1. Remote Detection ===");
    println!("  Local path '{}': is_remote = {}", ext_str, extensions::is_remote(&ext_str));
    println!("  URL 'https://example.com/ext.ts': is_remote = {}",
        extensions::is_remote("https://example.com/ext.ts"));
    println!("  URL 'http://example.com/ext.ts': is_remote = {}",
        extensions::is_remote("http://example.com/ext.ts"));
    println!("  Relative path './local.sh': is_remote = {}",
        extensions::is_remote("./local.sh"));
    println!();

    // ── 2. Origin hashing ──────────────────────────────────────────────
    println!("=== 2. Origin Hashing ===");
    let local_hash = extensions::hash_origin(&ext_str).expect("hash local");
    let remote_hash = extensions::hash_origin("https://example.com/ext.ts").expect("hash remote");
    println!("  Local origin hash:  {local_hash}");
    println!("  Remote origin hash: {remote_hash}");
    assert_eq!(local_hash.len(), 40, "SHA-1 should be 40 hex chars");
    assert_eq!(remote_hash.len(), 40, "SHA-1 should be 40 hex chars");
    println!();

    // ── 3. Path normalization ──────────────────────────────────────────
    println!("=== 3. Path Resolution ===");
    // For a remote URL, load_entrypoint downloads and caches
    // For a local path, it canonicalizes and returns
    let extension_dir = std::env::temp_dir().join("sunbeam-ext-test");
    let entrypoint = extensions::load_entrypoint(&ext_str, &extension_dir)
        .expect("load entrypoint");
    println!("  Entrypoint for local path: {}", entrypoint.display());
    assert!(entrypoint.is_absolute(), "entrypoint should be absolute");
    assert!(entrypoint.exists(), "entrypoint file should exist");
    println!();

    // ── 4. Cache directory structure ───────────────────────────────────
    println!("=== 4. Cache Directory ===");
    let cache_dir = sunbeam_rust::utils::cache_dir();
    println!("  Cache dir: {}", cache_dir.display());
    println!("  Config dir: {}", sunbeam_rust::utils::config_dir().display());
    assert!(cache_dir.to_string_lossy().contains("sunbeam"),
        "cache dir should contain 'sunbeam'");

    let ext_cache_dir = cache_dir.join("extensions").join(&local_hash);
    println!("  Extension cache: {}", ext_cache_dir.display());
    println!();

    // ── 5. Load and inspect cache behavior ─────────────────────────────
    println!("=== 5. Loading and Caching ===");
    // First load - no cache exists, manifest extracted from script
    let ext = extensions::load_extension(&ext_str).expect("load extension");
    println!("  Manifest title: {}", ext.manifest.title);
    println!("  Commands: {}", ext.manifest.commands.len());

    // The manifest should now be cached
    let manifest_cache = extension_dir.join("manifest.json");
    println!("  Manifest cache path: {}", manifest_cache.display());

    // Cleanup test directory
    if extension_dir.exists() {
        std::fs::remove_dir_all(&extension_dir).ok();
        println!("  Cleaned up test dir: {}", extension_dir.display());
    }

    println!("\n✅ Extension remote simulation demo complete!");
}
