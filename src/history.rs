use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

use crate::types::ListItem;
use crate::utils;

pub fn history_path() -> PathBuf {
    utils::cache_dir().join("history.json")
}

#[derive(Debug, Clone)]
pub struct History {
    entries: HashMap<String, i64>,
    path: PathBuf,
}

impl History {
    pub fn load(path: &PathBuf) -> Result<Self> {
        let entries = if path.exists() {
            let bytes = std::fs::read(path)?;
            serde_json::from_slice(&bytes)?
        } else {
            HashMap::new()
        };

        Ok(History {
            entries,
            path: path.clone(),
        })
    }

    pub fn sort(&self, items: &mut [ListItem]) {
        let entries = &self.entries;
        items.sort_by_key(|item| {
            let key = item.id.as_deref().unwrap_or(&item.title);
            std::cmp::Reverse(entries.get(key).copied().unwrap_or(0))
        });
    }

    pub fn update(&mut self, key: &str) {
        self.entries
            .insert(key.to_string(), chrono::Utc::now().timestamp());
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(&self.path)?;
        serde_json::to_writer_pretty(file, &self.entries)?;
        Ok(())
    }
}
