use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use pulldown_cmark::{Event, Parser, Tag, TagEnd, HeadingLevel};

/// Renders markdown text to Ratatui styled lines.
/// Handles: H1-H3, bold, italic, inline code, bullet lists, numbered lists, code blocks.
pub fn render_markdown(text: &str, max_width: usize) -> (Vec<Line<'static>>, usize) {
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut code_block = false;
    let mut list_indent: usize = 0;
    let mut total_lines = 0;

    let parser = Parser::new(text);

    for event in parser {
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
                        current_line
                            .iter()
                            .map(|s| {
                                let mut style = s.style.clone();
                                style = style.add_modifier(Modifier::BOLD);
                                Span::styled(s.content.clone(), style)
                            })
                            .collect::<Vec<_>>(),
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
            Event::Start(Tag::CodeBlock(_)) => {
                code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                code_block = false;
            }
            Event::Start(Tag::List(..)) => {
                list_indent = 2;
            }
            Event::End(TagEnd::List(..)) => {
                list_indent = 0;
            }
            Event::Start(Tag::Item) => {
                let indent = " ".repeat(list_indent);
                current_line.push(Span::raw(format!("{}• ", indent)));
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
                // Strip HTML tags for display
                let clean = text.replace('<', "&lt;").replace('>', "&gt;");
                current_line.push(Span::raw(clean));
            }
            _ => {}
        }
    }

    // Flush remaining content
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
        total_lines += 1;
    }

    (lines, total_lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown_empty() {
        let (lines, _) = render_markdown("", 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_render_markdown_plain_text() {
        let (lines, _) = render_markdown("Hello World", 80);
        assert_eq!(lines.len(), 1);
        let content = lines[0].to_string();
        assert!(content.contains("Hello World"));
    }

    #[test]
    fn test_render_markdown_heading() {
        let (lines, _) = render_markdown("# Title", 80);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_markdown_bold() {
        let (lines, _) = render_markdown("**bold text**", 80);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_markdown_code_block() {
        let (lines, _) = render_markdown("```\ncode here\n```", 80);
        assert!(lines.len() >= 1, "code block should produce lines");
    }

    #[test]
    fn test_render_markdown_list() {
        let (lines, _) = render_markdown("- item 1\n- item 2", 80);
        assert!(!lines.is_empty());
    }
}
