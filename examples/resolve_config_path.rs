fn main() {
    // Resolves the config path using three strategies in order:
    //
    // 1. $SUNBEAM_CONFIG environment variable
    // 2. Walk up from the current directory looking for sunbeam.json
    // 3. Fallback to ~/.config/sunbeam/sunbeam.json
    let path = sunbeam_rust::config::resolve_config_path();
    println!("Resolved config path: {}", path.display());

    // The path is always absolute
    assert!(path.is_absolute());

    // When SUNBEAM_CONFIG is set, that path is used directly
    std::env::set_var("SUNBEAM_CONFIG", "/tmp/custom-sunbeam.json");
    let forced = sunbeam_rust::config::resolve_config_path();
    assert_eq!(forced, std::path::PathBuf::from("/tmp/custom-sunbeam.json"));
    println!("Forced config path:  {}", forced.display());
}
