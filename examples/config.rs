fn main() {
    // Resolve the config path using sunbeam's resolution strategy
    let config_path = sunbeam_rust::config::resolve_config_path();
    println!("Config path: {}", config_path.display());

    // Load and parse the config file
    let cfg = sunbeam_rust::config::load(&config_path)
        .expect("failed to load config");
    println!(
        "Loaded config with {} extensions",
        cfg.extensions.as_ref().map(|e| e.len()).unwrap_or(0)
    );

    // List all extension aliases
    #[allow(dead_code)]
    fn aliases(cfg: &sunbeam_rust::config::Config) -> Vec<String> {
        cfg.extensions
            .as_ref()
            .map(|exts| exts.keys().cloned().collect())
            .unwrap_or_default()
    }
    println!("Extension aliases: {:?}", aliases(&cfg));

    // Resolve a relative path against the config directory
    let resolved = cfg.resolve("some/relative/path");
    println!("Resolved relative path: {}", resolved.display());

    // Resolve a home-relative path
    let home_path = cfg.resolve("~/some/file.txt");
    println!("Resolved home path: {}", home_path.display());

    // Save the config to a temporary location
    let tmp = std::env::temp_dir().join("sunbeam-example-config.json");
    let mut save_cfg = cfg.clone();
    save_cfg.path = tmp.clone();
    save_cfg.save().expect("failed to save config");
    println!("Saved copy to: {}", tmp.display());
    assert!(tmp.exists());
    std::fs::remove_file(&tmp).ok();
}
