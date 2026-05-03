//! Demonstrates the fuzzy matching algorithm used by sunbeam's TUI filtering.
//!
//! The `fuzzy_score` function ranks items by how well they match a search
//! pattern, with bonuses for consecutive character matches.
//!
//! The `tui::types` module is not public, so we inline the algorithm here.
//!
//! Usage: cargo run --example fuzzy_scoring_demo

/// Computes a fuzzy match score between `text` and `pattern`.
/// Score 0 = no match, positive = match quality (higher = better).
fn fuzzy_score(text: &str, pattern: &str) -> i32 {
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

    let mut consecutive = 0;
    let mut score = 0;
    let mut prev_match: Option<usize> = None;

    for (ti, tc) in text_chars.iter().enumerate() {
        if pi < chars.len() && *tc == chars[pi] {
            if let Some(prev) = prev_match {
                if ti == prev + 1 {
                    consecutive += 1;
                    score += 10 * consecutive;
                } else {
                    consecutive = 1;
                    score += 5;
                }
            } else {
                consecutive = 1;
                score += 1;
            }
            prev_match = Some(ti);
            pi += 1;
        }
    }

    if pi == chars.len() { score.max(1) } else { 0 }
}

/// Filters items by fuzzy matching against a query, returning (index, score) pairs.
fn filter_items(texts: &[&str], query: &str) -> Vec<(usize, i32)> {
    if query.is_empty() {
        return texts.iter().enumerate().map(|(i, _)| (i, i32::MAX)).collect();
    }
    let mut scored: Vec<(usize, i32)> = texts.iter().enumerate()
        .filter_map(|(i, text)| {
            let score = fuzzy_score(text, query);
            if score > 0 { Some((i, score)) } else { None }
        })
        .collect();
    scored.sort_by_key(|k| std::cmp::Reverse(k.1));
    scored
}

fn main() {
    // ── 1. Basic scoring behavior ──────────────────────────────────────
    println!("=== 1. Basic Scoring ===");
    let cases = vec![
        ("hello", "", "empty pattern"),
        ("hello", "hello", "exact match"),
        ("hello world", "world", "substring match"),
        ("Hello World", "world", "case insensitive"),
        ("hello", "xyz", "no match"),
        ("rust programming", "rust", "start match"),
        ("programming rust", "rust", "end match"),
    ];

    for (text, pattern, label) in &cases {
        let score = fuzzy_score(text, pattern);
        println!("  {label:25} score({text:20}, {pattern:10}) = {score}");
    }
    println!();

    // ── 2. Consecutive character bonus ─────────────────────────────────
    println!("=== 2. Consecutive Match Bonus ===");
    println!("  fuzzy_score(\"abcde\", \"abcde\") = {} (exact, max bonus)", fuzzy_score("abcde", "abcde"));
    println!("  fuzzy_score(\"abcde\", \"ace\")  = {} (scattered)", fuzzy_score("abcde", "ace"));
    println!("  fuzzy_score(\"abcde\", \"ab\")   = {} (consecutive start)", fuzzy_score("abcde", "ab"));
    println!("  fuzzy_score(\"abcde\", \"cde\")  = {} (consecutive end)", fuzzy_score("abcde", "cde"));
    println!("  fuzzy_score(\"abcde\", \"a\")    = {} (single char)", fuzzy_score("abcde", "a"));
    println!();

    // ── 3. Case sensitivity ────────────────────────────────────────────
    println!("=== 3. Case Sensitivity ===");
    // Pattern with uppercase chars triggers case-sensitive mode
    println!("  fuzzy_score(\"HelloWorld\", \"HW\")  = {} (uppercase pattern)", fuzzy_score("HelloWorld", "HW"));
    println!("  fuzzy_score(\"HelloWorld\", \"hw\")  = {} (lowercase pattern)", fuzzy_score("HelloWorld", "hw"));
    println!("  fuzzy_score(\"helloworld\", \"HW\") = {} (uppercase pattern, all-lowercase text)", fuzzy_score("helloworld", "HW"));
    println!();

    // ── 4. Filter items demo ──────────────────────────────────────────
    println!("=== 4. Filter Items ===");
    let items: Vec<&str> = vec![
        "rust programming",
        "python programming",
        "rust tools",
        "javascript",
        "typescript",
        "rust-analyzer",
    ];

    let queries = vec!["rust", "python", "script", "tools", "xyz"];
    for query in &queries {
        let results = filter_items(&items, query);
        println!("  Query '{}': {} match(es)", query, results.len());
        for (idx, score) in &results {
            println!("    score={score:3} {}", items[*idx]);
        }
    }
    println!();

    // ── 5. Empty and edge cases ────────────────────────────────────────
    println!("=== 5. Edge Cases ===");
    assert_eq!(fuzzy_score("", ""), 1, "empty text + empty pattern = 1");
    assert_eq!(fuzzy_score("anything", ""), 1, "any text + empty pattern = 1");
    assert_eq!(fuzzy_score("", "a"), 0, "empty text + non-empty pattern = 0");
    println!("  ✅ All edge cases pass");

    println!("\n✅ Fuzzy scoring demo complete!");
}
