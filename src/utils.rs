use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

/// Returns the sunbeam config directory: `$XDG_CONFIG_HOME/sunbeam`
/// or `~/.config/sunbeam`.
pub fn config_dir() -> PathBuf {
    if let Ok(env) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(env).join("sunbeam");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".config")
        .join("sunbeam")
}

/// Returns the sunbeam cache directory: `$XDG_CACHE_HOME/sunbeam`
/// or `~/.cache/sunbeam`.
pub fn cache_dir() -> PathBuf {
    if let Ok(env) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(env).join("sunbeam");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/"))
        .join(".cache")
        .join("sunbeam")
}

/// Returns the user's preferred editor from `$VISUAL`, `$EDITOR`, or `"vi"`.
pub fn find_editor() -> String {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string())
}

/// Returns the user's shell from `$SHELL`, or `"/bin/sh"`.
#[allow(dead_code)]
pub fn find_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

/// Returns the user's pager from `$PAGER`, or `"less"`.
#[allow(dead_code)]
pub fn find_pager() -> String {
    std::env::var("PAGER").unwrap_or_else(|_| "less".to_string())
}

/// Opens a URL or file path using the system default application.
///
/// On macOS uses `open`, on Linux uses `xdg-open`. The command is detached
/// via `nohup` so sunbeam does not wait for the application to close.
///
/// # Errors
/// Returns an error if the underlying `xdg-open`/`open` command cannot be
/// spawned.
pub fn open_target(target: &str) -> Result<()> {
    let result = if cfg!(target_os = "macos") {
        Command::new("nohup").args(["open", target]).spawn()
    } else {
        Command::new("nohup").args(["xdg-open", target]).spawn()
    };

    match result {
        Ok(mut child) => {
            child.wait()?;
            Ok(())
        }
        Err(e) => anyhow::bail!("failed to open: {}", e),
    }
}

/// Strips ANSI escape sequences from a string.
#[allow(dead_code)]
pub fn strip_ansi(s: &str) -> String {
    strip_ansi_escapes::strip_str(s)
}

/// Validates and normalizes an extension origin string.
///
/// - Remote URLs are returned as-is.
/// - Local paths are resolved to their canonical absolute form.
///
/// # Errors
/// Returns an error if the origin is a local path that does not exist.
pub fn normalize_origin(origin: &str) -> Result<String> {
    if !origin.starts_with("http://") && !origin.starts_with("https://") {
        let path = PathBuf::from(origin);
        if !path.exists() {
            anyhow::bail!("failed to find origin: {}", origin);
        }
        if origin.starts_with('~') {
            return Ok(origin.to_string());
        }
        let abs = std::fs::canonicalize(&path)?;
        return Ok(abs.to_string_lossy().to_string());
    }
    Ok(origin.to_string())
}

/// Extracts a human-readable alias from a URL or file path origin.
///
/// For example, `https://example.com/my-extension.py` → `"my-extension"`.
///
/// # Errors
/// Returns an error if the URL cannot be parsed.
pub fn extract_alias(origin: &str) -> Result<String> {
    let url = url::Url::parse(origin)?;
    let path = PathBuf::from(url.path());
    let base = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("extension")
        .to_string();
    Ok(base)
}
