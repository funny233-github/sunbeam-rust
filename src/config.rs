use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oneliners: Option<Vec<Oneliner>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, ExtensionConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oneliner: Option<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oneliner {
    pub title: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    pub origin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<Vec<RootItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootItem {
    pub title: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
}

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
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(&self.path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn aliases(&self) -> Vec<String> {
        self.extensions
            .as_ref()
            .map(|exts| exts.keys().cloned().collect())
            .unwrap_or_default()
    }

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
