use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;

use crate::types::ListItem;
use crate::utils;

/// Returns the default path for the history file: `~/.cache/sunbeam/history.json`.
pub fn history_path() -> PathBuf {
    utils::cache_dir().join("history.json")
}

/// Tracks item selection frequency to sort frequently-used items to the top.
///
/// Persisted as a JSON map of item ID → Unix timestamp.
#[derive(Debug, Clone)]
pub struct History {
    entries: HashMap<String, i64>,
    path: PathBuf,
}

impl History {
    /// Loads history from a JSON file, or returns an empty history if the file
    /// does not exist.
    ///
    /// # Errors
    /// Returns an error if the file exists but cannot be parsed.
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

    /// Sorts items in-place by most-recently-used descending.
    ///
    /// Items without a history entry remain in their original relative order
    /// at the end of the list.
    pub fn sort(&self, items: &mut [ListItem]) {
        let entries = &self.entries;
        items.sort_by_key(|item| {
            let key = item.id.as_deref().unwrap_or(&item.title);
            std::cmp::Reverse(entries.get(key).copied().unwrap_or(0))
        });
    }

    /// Records that an item was used at the current moment.
    pub fn update(&mut self, key: &str) {
        self.entries
            .insert(key.to_string(), chrono::Utc::now().timestamp());
    }

    /// Persists the history to disk as pretty-printed JSON.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created or written.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(&self.path)?;
        serde_json::to_writer_pretty(file, &self.entries)?;
        Ok(())
    }
}
