use std::path::PathBuf;
use std::time::Instant;

use crate::config::{Config, ExtensionConfig};
use crate::history::History;
use crate::types::*;

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

    // Error page
    pub err: Option<String>,

    // Pagination
    pub page: usize,
    pub page_size: usize,

    // Spinner animation (incremented each frame)
    pub tick: u64,

    // Auto-refresh tracking
    pub last_auto_refresh: Instant,
}

#[derive(Clone)]
pub struct PageDetail {
    pub markdown: String,
    pub actions: Vec<Action>,
    pub inner_selection: usize,
    pub action_mode: bool,
    pub scroll_offset: usize,
    pub rendered_lines: Vec<ratatui::text::Line<'static>>,
    pub action_query: String,
}

#[derive(Clone)]
pub struct FormState {
    pub title: String,
    pub fields: Vec<FormField>,
    pub selection: usize,
    pub ext_cfg: ExtensionConfig,
    pub alias: String,
}

#[derive(Clone)]
pub struct FormField {
    pub input: Input,
    pub value: String,
    pub checked: bool,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
pub enum Page {
    Root,
    Runner(RunnerPage),
}

#[derive(Clone)]
pub struct RunnerPage {
    pub extension: Option<crate::extensions::Extension>,
    pub command_name: String,
    pub preferences: Option<serde_json::Map<String, serde_json::Value>>,
    pub mode: CommandMode,
    pub items: Vec<FilterItem>,
    pub filtered_items: Vec<usize>,
    pub selection: usize,
    pub query: String,
    pub actions: Vec<Action>,
    pub is_loading: bool,
    pub page: usize,
    pub page_size: usize,
    pub show_detail: bool,
    pub auto_refresh_seconds: Option<i32>,
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
                if first_match.is_some_and(|prev| ti == prev + 1) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(text: &str) -> FilterItem {
        FilterItem {
            filter_text: text.to_lowercase(),
            item: ListItem {
                id: None, title: text.into(), subtitle: None, detail: None,
                accessories: None, actions: None,
            },
        }
    }

    #[test]
    fn test_fuzzy_score_empty_query() {
        assert_eq!(fuzzy_score("hello", ""), 1);
        assert_eq!(fuzzy_score("", ""), 1);
    }

    #[test]
    fn test_fuzzy_score_exact_match() {
        let score = fuzzy_score("hello", "hello");
        assert!(score > 0, "exact match should have positive score");
    }

    #[test]
    fn test_fuzzy_score_partial_match() {
        let score = fuzzy_score("hello world", "world");
        assert!(score > 0, "partial match should have positive score");
    }

    #[test]
    fn test_fuzzy_score_case_insensitive() {
        let score = fuzzy_score("Hello World", "world");
        assert!(score > 0, "case-insensitive match should have positive score");
    }

    #[test]
    fn test_fuzzy_score_case_sensitive() {
        let score_upper = fuzzy_score("Hello", "H");
        let score_lower = fuzzy_score("Hello", "h");
        assert!(score_upper > 0, "uppercase pattern should match uppercase char");
        assert!(score_lower > 0, "lowercase pattern should also match");
    }

    #[test]
    fn test_fuzzy_score_no_match() {
        assert_eq!(fuzzy_score("hello", "xyz"), 0);
    }

    #[test]
    fn test_filter_items_empty_query_returns_all() {
        let items = vec![make_item("apple"), make_item("banana")];
        let result = filter_items(&items, "");
        assert_eq!(result.len(), 2, "empty query should return all items");
    }

    #[test]
    fn test_filter_items_matching() {
        let items = vec![make_item("hello world"), make_item("goodbye world")];
        let result = filter_items(&items, "hello");
        assert_eq!(result.len(), 1, "only one item matches 'hello'");
        assert_eq!(items[result[0].0].item.title, "hello world");
    }

    #[test]
    fn test_filter_items_scores_descending() {
        let items = vec![make_item("aa"), make_item("ab"), make_item("ac")];
        let result = filter_items(&items, "a");
        assert_eq!(result.len(), 3, "all items match 'a'");
        for i in 1..result.len() {
            assert!(result[i-1].1 >= result[i].1, "scores should be descending");
        }
    }
}
