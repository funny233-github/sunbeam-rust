//! Demonstrates the markdown-to-Ratatui-line rendering pipeline.
//!
//! Uses `pulldown_cmark` directly (same library as `tui::render_md`) to
//! convert CommonMark content into styled Ratatui `Line` values.
//!
//! Usage: cargo run --example markdown_rendering

use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Renders markdown text to Ratatui styled lines (inline implementation).
fn render_markdown(text: &str, max_width: usize) -> (Vec<Line<'static>>, usize) {
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut code_block = false;
    let mut list_indent = 0;
    let mut total_lines = 0;

    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_line.push(Span::styled(
                    match level {
                        HeadingLevel::H1 => "# ",
                        HeadingLevel::H2 => "## ",
                        HeadingLevel::H3 => "### ",
                        _ => "",
                    },
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
            Event::End(TagEnd::Heading(_)) => {
                if !current_line.is_empty() {
                    let line = Line::from(
                        current_line.iter().map(|s| {
                            let mut style = s.style;
                            style = style.add_modifier(Modifier::BOLD);
                            Span::styled(s.content.clone(), style)
                        }).collect::<Vec<_>>(),
                    );
                    lines.push(line);
                    current_line.clear();
                    total_lines += 1;
                }
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                    total_lines += 1;
                }
            }
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::CodeBlock(_)) => code_block = true,
            Event::End(TagEnd::CodeBlock) => code_block = false,
            Event::Start(Tag::List(..)) => list_indent = 2,
            Event::End(TagEnd::List(..)) => list_indent = 0,
            Event::Start(Tag::Item) => {
                current_line.push(Span::raw("  • "));
            }
            Event::End(TagEnd::Item) => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                    total_lines += 1;
                }
            }
            Event::Code(text) => {
                current_line.push(Span::styled(
                    format!("`{}`", text),
                    Style::default().fg(Color::Yellow),
                ));
            }
            Event::Text(text) => {
                if code_block {
                    lines.push(Line::from(Span::styled(
                        text.to_string(),
                        Style::default().fg(Color::Cyan),
                    )));
                    total_lines += 1;
                } else {
                    let mut style = Style::default();
                    if bold { style = style.add_modifier(Modifier::BOLD); }
                    if italic { style = style.add_modifier(Modifier::ITALIC); }
                    current_line.push(Span::styled(text.to_string(), style));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                    total_lines += 1;
                }
            }
            Event::Rule => {
                lines.push(Line::from(Span::raw("─".repeat(max_width.min(40)))));
                total_lines += 1;
            }
            Event::Html(text) => {
                let clean = text.replace('<', "&lt;").replace('>', "&gt;");
                current_line.push(Span::raw(clean));
            }
            _ => {}
        }
    }
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
        total_lines += 1;
    }
    (lines, total_lines)
}

fn main() {
    // ── 1. Plain text ──────────────────────────────────────────────────
    println!("=== 1. Plain Text ===");
    render_and_print("Hello World");
    println!();

    // ── 2. Headings ────────────────────────────────────────────────────
    println!("=== 2. Headings ===");
    render_and_print("# Heading 1\n\n## Heading 2\n\n### Heading 3");
    println!();

    // ── 3. Bold and Italic ─────────────────────────────────────────────
    println!("=== 3. Bold and Italic ===");
    render_and_print("This is **bold** and this is *italic* and ***both***.");
    println!();

    // ── 4. Inline code ─────────────────────────────────────────────────
    println!("=== 4. Inline Code ===");
    render_and_print("Use the `sunbeam` command to launch.");
    println!();

    // ── 5. Code blocks ─────────────────────────────────────────────────
    println!("=== 5. Code Blocks ===");
    render_and_print("Example:\n\n```\nfn main() {\n    println!(\"hello\");\n}\n```");
    println!();

    // ── 6. Lists ───────────────────────────────────────────────────────
    println!("=== 6. Lists ===");
    render_and_print("Shopping list:\n\n- Apples\n- Bananas\n- Cherries");
    println!();

    // ── 7. Horizontal rule ─────────────────────────────────────────────
    println!("=== 7. Horizontal Rule ===");
    render_and_print("Above\n\n---\n\nBelow");
    println!();

    // ── 8. Complex document ────────────────────────────────────────────
    println!("=== 8. Complex Document ===");
    render_and_print("# Sunbeam User Guide\n\n## Installation\n\nInstall using **cargo**:\n\n```bash\ncargo install --path .\n```\n\n## Usage\n\nRun `sunbeam` to launch the TUI.\n\n### Commands\n\n- **filter** — client-side fuzzy matching\n- **search** — server-side query\n- **detail** — full-page markdown view\n\n> Note: Extensions must be executable.\n\n---\n\n*Happy coding!*");
    println!();

    // ── 9. Empty input ─────────────────────────────────────────────────
    println!("=== 9. Empty Input ===");
    let (lines, count) = render_with_stats("");
    println!("  Lines: {lines:?}, count: {count}");
    println!();

    // ── 10. HTML content ───────────────────────────────────────────────
    println!("=== 10. HTML Escaping ===");
    render_and_print("HTML: <script>alert('xss')</script>");
    println!();

    println!("✅ Markdown rendering demo complete!");
}

fn render_and_print(markdown: &str) {
    let (lines, count) = render_with_stats(markdown);
    println!("  Rendered lines: {count}");
    for line in &lines {
        let content = line_to_plain_text(line);
        let style_info = line_style_summary(line);
        println!("    [{style_info}] {content}");
    }
}

fn render_with_stats(markdown: &str) -> (Vec<Line<'static>>, usize) {
    render_markdown(markdown, 60)
}

fn line_to_plain_text(line: &Line<'static>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn line_style_summary(line: &Line<'static>) -> String {
    let mut parts = Vec::new();
    for span in &line.spans {
        let style = span.style;
        let mut mods = Vec::new();
        if style.add_modifier.bits() != 0 {
            if style.add_modifier.contains(Modifier::BOLD) {
                mods.push("bold");
            }
            if style.add_modifier.contains(Modifier::ITALIC) {
                mods.push("italic");
            }
        }
        let fg = if let Some(color) = style.fg {
            format!("{:?}", color)
        } else {
            "default".into()
        };
        let mod_str = if mods.is_empty() { String::new() } else { format!("+{}", mods.join(",")) };
        parts.push(format!("{fg}{mod_str}"));
    }
    parts.join(" | ")
}
