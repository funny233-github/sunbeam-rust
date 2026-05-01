use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::utils;

/// The user's sunbeam configuration, persisted as `sunbeam.json`.
///
/// Contains a list of one-liner commands and a map of installed extensions
/// keyed by user-chosen alias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Quick shell commands shown at the top of the root list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oneliners: Option<Vec<Oneliner>>,
    /// Installed extensions, keyed by alias.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, ExtensionConfig>>,
    /// Shortcut for a single oneliner (legacy field).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oneliner: Option<String>,

    /// Filesystem path this config was loaded from (not serialized).
    #[serde(skip)]
    pub path: PathBuf,
}

/// A quick shell command shown in the root list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oneliner {
    /// Display label.
    pub title: String,
    /// Shell command to execute.
    pub command: String,
    /// Whether to run interactively (TTY mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactive: Option<bool>,
    /// Working directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Whether to quit sunbeam after execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

/// Configuration for a single installed extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    /// Local path or remote URL to the extension script.
    pub origin: String,
    /// Saved preference values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<serde_json::Map<String, serde_json::Value>>,
    /// Pre-configured root items that bypass the command list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<Vec<RootItem>>,
}

/// A pre-configured command invocation shown directly in the root list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootItem {
    pub title: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Resolves the config file path by checking, in order:
///
/// 1. The `SUNBEAM_CONFIG` environment variable.
/// 2. A `sunbeam.json` file found by walking up from the current directory.
/// 3. The default location: `~/.config/sunbeam/sunbeam.json`.
pub fn resolve_config_path() -> PathBuf {
    if let Ok(env) = std::env::var("SUNBEAM_CONFIG") {
        return PathBuf::from(env);
    }

    let cwd = std::env::current_dir().ok();
    if let Some(cwd) = cwd {
        let mut dir = cwd.clone();
        loop {
            let candidate = dir.join("sunbeam.json");
            if candidate.exists() {
                return candidate;
            }
            if !dir.pop() {
                break;
            }
        }
    }

    utils::config_dir().join("sunbeam.json")
}

/// Loads and validates the config file at `config_path`.
///
/// # Example
/// ```
/// let cfg = config::load(&config::resolve_config_path())?;
/// ```
///
/// # Errors
/// Returns an error if the file cannot be read, fails JSON schema validation,
/// or cannot be deserialized.
pub fn load(config_path: &PathBuf) -> Result<Config> {
    let bytes = std::fs::read(config_path)
        .with_context(|| format!("failed to read config: {}", config_path.display()))?;

    crate::schemas::validate_config(&bytes)
        .context("invalid config")?;

    let mut config: Config = serde_json::from_slice(&bytes)?;
    config.path = config_path.clone();
    if config.extensions.is_none() {
        config.extensions = Some(HashMap::new());
    }
    Ok(config)
}

impl Config {
    /// Persists the config to disk as pretty-printed JSON.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(&self.path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    /// Returns the list of installed extension aliases.
    #[allow(dead_code)]
    pub fn aliases(&self) -> Vec<String> {
        self.extensions
            .as_ref()
            .map(|exts| exts.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Converts a user-supplied path (possibly relative or `~`-prefixed) to
    /// an absolute path resolved relative to the config file's directory.
    pub fn resolve(&self, path: &str) -> PathBuf {
        if let Some(rest) = path.strip_prefix("~/") {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            return home.join(rest);
        }
        let pb = PathBuf::from(path);
        if pb.is_relative() {
            if let Some(parent) = self.path.parent() {
                return parent.join(pb);
            }
        }
        pb
    }
}
