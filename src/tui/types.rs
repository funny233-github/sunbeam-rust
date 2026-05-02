use std::path::PathBuf;
use std::time::Instant;

use crate::config::{Config, ExtensionConfig};
use crate::history::History;
use crate::types::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct FilterItem {
    pub item: ListItem,
    pub filter_text: String,
}

/// Central application state for the TUI event loop.
pub struct AppState {
    pub items: Vec<FilterItem>,
    pub filtered_items: Vec<usize>,
    pub selection: usize,
    pub query: String,

    pub actions: Vec<Action>,
    pub action_selection: usize,
    pub action_mode: bool,

    pub page_stack: Vec<Page>,

    pub config_path: PathBuf,
    pub config: Config,
    pub history: History,

    pub form: Option<FormState>,

    pub detail: Option<PageDetail>,

    pub notification: String,
    pub notification_until: Option<Instant>,

    pub width: u16,
    pub height: u16,
    pub should_quit: bool,
}

#[derive(Clone)]
pub struct PageDetail {
    pub markdown: String,
    pub actions: Vec<Action>,
    pub inner_selection: usize,
    pub action_mode: bool,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct FormState {
    pub title: String,
    pub fields: Vec<FormField>,
    pub selection: usize,
    pub config: Config,
    pub ext_cfg: ExtensionConfig,
    pub alias: String,
}

#[derive(Clone)]
pub struct FormField {
    pub input: Input,
    pub value: String,
    pub checked: bool,
}

#[allow(dead_code, clippy::large_enum_variant)]
#[derive(Clone)]
pub enum Page {
    Root,
    Detail(PageDetail),
    Runner(RunnerPage),
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct RunnerPage {
    pub extension_origin: String,
    pub command_name: String,
    pub entrypoint: std::path::PathBuf,
    pub preferences: Option<serde_json::Map<String, serde_json::Value>>,
    pub mode: CommandMode,
    pub items: Vec<FilterItem>,
    pub filtered_items: Vec<usize>,
    pub selection: usize,
    pub query: String,
    pub actions: Vec<Action>,
    pub action_selection: usize,
    pub action_mode: bool,
    pub is_detail: bool,
    pub detail_text: String,
    pub detail_actions: Vec<Action>,
    pub has_loaded: bool,
    pub is_loading: bool,
    pub auto_refresh: Option<i32>,
}

/// Computes a fuzzy match score between `text` and `pattern`.
pub fn fuzzy_score(text: &str, pattern: &str) -> i32 {
    if pattern.is_empty() {
        return 1;
    }
    let lower_text = text.to_lowercase();
    let lower_pattern = pattern.to_lowercase();
    let case_sensitive = pattern.chars().any(|c| c.is_uppercase());

    let search_text = if case_sensitive { text } else { &lower_text };
    let search_pattern = if case_sensitive { pattern } else { &lower_pattern };

    let mut pi = 0;
    let chars: Vec<char> = search_pattern.chars().collect();
    let text_chars: Vec<char> = search_text.chars().collect();

    let mut first_match = None;
    let mut consecutive = 0;
    let mut score = 0;

    for (ti, tc) in text_chars.iter().enumerate() {
        if pi < chars.len() && *tc == chars[pi] {
            if pi > 0 {
                if first_match.is_none_or(|prev| ti == prev + 1) {
                    consecutive += 1;
                    score += 10 * consecutive;
                } else {
                    consecutive = 1;
                    score += 5;
                }
            } else {
                first_match = Some(ti);
                consecutive = 1;
                score += 1;
            }
            pi += 1;
        }
    }

    if pi == chars.len() {
        score.max(1)
    } else {
        0
    }
}

/// Filters and scores items against a query string.
pub fn filter_items(items: &[FilterItem], query: &str) -> Vec<(usize, i32)> {
    if query.is_empty() {
        return items.iter().enumerate().map(|(i, _)| (i, i32::MAX)).collect();
    }
    let mut scored: Vec<(usize, i32)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let score = fuzzy_score(&item.filter_text, query);
            if score > 0 { Some((i, score)) } else { None }
        })
        .collect();
    scored.sort_by_key(|k| std::cmp::Reverse(k.1));
    scored
}
