use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use sha1::{Digest, Sha1};

use crate::config;
use crate::types::{CommandSpec, Manifest, Payload};
use crate::utils;

#[derive(Debug, Clone)]
pub struct Extension {
    pub manifest: Manifest,
    pub entrypoint: PathBuf,
}

pub fn is_remote(origin: &str) -> bool {
    origin.starts_with("http://") || origin.starts_with("https://")
}

pub fn hash_origin(origin: &str) -> Result<String> {
    let origin = if !is_remote(origin) {
        let abs = std::fs::canonicalize(origin)
            .unwrap_or_else(|_| PathBuf::from(origin));
        abs.to_string_lossy().to_string()
    } else {
        origin.to_string()
    };

    let mut hasher = Sha1::new();
    hasher.update(origin.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn download_entrypoint(origin: &str, target: &Path) -> Result<()> {
    let resp = reqwest::blocking::get(origin)
        .context("failed to download extension")?;
    if !resp.status().is_success() {
        anyhow::bail!("failed to download extension: {}", resp.status());
    }
    let bytes = resp.bytes().context("failed to read response body")?;
    std::fs::write(target, &bytes)
        .context("failed to write entrypoint")?;
    set_executable(target)?;
    Ok(())
}

fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path)?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

pub fn load_entrypoint(origin: &str, extension_dir: &Path) -> Result<PathBuf> {
    if is_remote(origin) {
        let url = url::Url::parse(origin)?;
        let filename = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .unwrap_or("extension");
        let entrypoint = extension_dir.join(filename);

        if !entrypoint.exists() {
            std::fs::create_dir_all(extension_dir)?;
            download_entrypoint(origin, &entrypoint)?;
        }
        Ok(entrypoint)
    } else {
        let entrypoint = if origin.starts_with('~') {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            home.join(&origin[2..])
        } else {
            let pb = PathBuf::from(origin);
            if pb.is_relative() {
                let config_path = crate::config::resolve_config_path();
                if let Some(parent) = config_path.parent() {
                    parent.join(pb)
                } else {
                    pb
                }
            } else {
                pb
            }
        };
        Ok(std::fs::canonicalize(&entrypoint)?)
    }
}

fn extract_manifest(entrypoint: &Path) -> Result<Manifest> {
    set_executable(entrypoint)?;

    let dir = entrypoint.parent().unwrap_or(Path::new("."));
    let output = Command::new(entrypoint)
        .current_dir(dir)
        .env("SUNBEAM", "1")
        .output()
        .context("failed to run extension to extract manifest")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stripped = strip_ansi_escapes::strip_str(&stderr);
        anyhow::bail!("command failed: {}", stripped);
    }

    crate::schemas::validate_manifest(&output.stdout)
        .context("invalid manifest")?;

    let manifest: Manifest = serde_json::from_slice(&output.stdout)?;
    Ok(manifest)
}

fn cache_manifest(entrypoint: &Path, manifest_path: &Path) -> Result<Manifest> {
    let manifest = extract_manifest(entrypoint)?;
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(manifest_path)?;
    serde_json::to_writer(file, &manifest)?;
    Ok(manifest)
}

pub fn load_extension(origin: &str) -> Result<Extension> {
    let hash = hash_origin(origin)?;
    let extension_dir = utils::cache_dir().join("extensions").join(&hash);
    let entrypoint = load_entrypoint(origin, &extension_dir)?;

    let manifest_path = extension_dir.join("manifest.json");
    let manifest = if manifest_path.exists() {
        let entry_mtime = std::fs::metadata(&entrypoint)?.modified()?;
        let manifest_mtime = std::fs::metadata(&manifest_path)?.modified()?;
        if entry_mtime > manifest_mtime {
            cache_manifest(&entrypoint, &manifest_path)?
        } else {
            let bytes = std::fs::read(&manifest_path)?;
            serde_json::from_slice(&bytes)?
        }
    } else {
        cache_manifest(&entrypoint, &manifest_path)?
    };

    Ok(Extension { manifest, entrypoint })
}

pub fn upgrade(extension_config: &config::ExtensionConfig) -> Result<()> {
    let hash = hash_origin(&extension_config.origin)?;
    let extension_dir = utils::cache_dir().join("extensions").join(&hash);
    let manifest_path = extension_dir.join("manifest.json");

    if is_remote(&extension_config.origin) {
        let url = url::Url::parse(&extension_config.origin)?;
        let filename = url
            .path_segments()
            .and_then(|mut segments| segments.next_back())
            .unwrap_or("extension");
        let entrypoint = extension_dir.join(filename);
        std::fs::create_dir_all(&extension_dir)?;
        download_entrypoint(&extension_config.origin, &entrypoint)?;
        cache_manifest(&entrypoint, &manifest_path)?;
    } else {
        let origin = std::path::Path::new(&extension_config.origin);
        let entrypoint = if origin.is_relative() {
            let config_path = crate::config::resolve_config_path();
            config_path.parent().unwrap().join(origin)
        } else {
            origin.to_path_buf()
        };
        set_executable(&entrypoint)?;
        cache_manifest(&entrypoint, &manifest_path)?;
    }

    Ok(())
}

impl Extension {
    pub fn command(&self, name: &str) -> Option<&CommandSpec> {
        self.manifest.commands.iter().find(|c| c.name == name)
    }

    pub fn root_commands(&self) -> Vec<&CommandSpec> {
        self.manifest
            .commands
            .iter()
            .filter(|c| !c.hidden.unwrap_or(false))
            .collect()
    }

    pub fn cmd(&self, input: &Payload) -> Result<Command> {
        let mut prefs = input
            .preferences
            .clone()
            .unwrap_or_default();
        for pref in self.manifest.preferences.iter().flatten() {
            if !prefs.contains_key(&pref.name) {
                if pref.optional.unwrap_or(false) {
                    if let Some(ref default) = pref.default {
                        prefs.insert(pref.name.clone(), default.clone());
                    }
                } else {
                    anyhow::bail!("missing required preference: {}", pref.name);
                }
            }
        }

        let command_spec = self.command(&input.command)
            .ok_or_else(|| anyhow::anyhow!("command {} not found", input.command))?;

        let mut params = input
            .params
            .clone()
            .unwrap_or_default();
        for param in command_spec.params.iter().flatten() {
            if !params.contains_key(&param.name) {
                if param.optional.unwrap_or(false) {
                    if let Some(ref default) = param.default {
                        params.insert(param.name.clone(), default.clone());
                    }
                } else {
                    anyhow::bail!("missing required parameter: {}", param.name);
                }
            }
        }

        let payload = Payload {
            command: input.command.clone(),
            preferences: Some(prefs),
            params: Some(params),
            cwd: Some(std::env::current_dir()?.to_string_lossy().to_string()),
            r#query: input.r#query.clone(),
        };

        let payload_json = serde_json::to_string(&payload)?;
        let dir = self.entrypoint.parent().unwrap_or(Path::new("."));

        let mut cmd = Command::new(&self.entrypoint);
        cmd.arg(&payload_json)
            .current_dir(dir)
            .env("SUNBEAM", "1");
        Ok(cmd)
    }

    pub fn run(&self, input: &Payload) -> Result<Vec<u8>> {
        let mut cmd = self.cmd(input)?;
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stripped = strip_ansi_escapes::strip_str(&stderr);
            anyhow::bail!("command failed: {}", stripped);
        }
        Ok(output.stdout)
    }

    pub fn run_quiet(&self, input: &Payload) -> Result<()> {
        self.run(input)?;
        Ok(())
    }
}


