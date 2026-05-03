use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};

const H1_COLOR: Color = Color::Magenta;
const H2_COLOR: Color = Color::Cyan;
const H3_COLOR: Color = Color::Yellow;
const H4_COLOR: Color = Color::Blue;
const H5_COLOR: Color = Color::Blue;
const H6_COLOR: Color = Color::Blue;
const CODE_INLINE_FG: Color = Color::Yellow;
const CODE_INLINE_BG: Color = Color::DarkGray;
const CODE_BLOCK_FG: Color = Color::Cyan;
const BLOCKQUOTE_FG: Color = Color::Green;
const LINK_URL_FG: Color = Color::DarkGray;
const HR_FG: Color = Color::DarkGray;
const TASK_DONE_FG: Color = Color::Green;
const TASK_PENDING_FG: Color = Color::Red;
const BULLET_FG: Color = Color::Magenta;

/// Renders markdown text to Ratatui styled lines with a glamour-inspired theme.
///
/// Supports: H1–H6, blockquotes, code blocks (fenced with language label),
/// tables, bold/italic/strikethrough, inline code, links, task lists,
/// ordered/unordered lists (nested), horizontal rules, and word wrapping.
pub fn render_markdown(text: &str, max_width: usize) -> (Vec<Line<'static>>, usize) {
    let effective_width = if max_width < 20 { 20 } else { max_width };
    let mut ctx = Ctx::new(effective_width);
    for event in Parser::new(text) {
        ctx.handle(event);
    }
    ctx.flush_current_line();
    ctx.finalize_table();
    (ctx.lines, ctx.total_lines)
}

struct ListFrame {
    ordered: bool,
    counter: u64,
}

#[allow(dead_code)]
enum Align {
    None,
    Left,
    Center,
    Right,
}

struct Ctx {
    lines: Vec<Line<'static>>,
    current_line: Vec<Span<'static>>,
    current_width: usize,
    max_width: usize,
    total_lines: usize,
    pending_space: bool,
    bold: bool,
    italic: bool,
    strike: bool,
    link_href: String,
    image_alt: Option<String>,
    in_code_block: bool,
    code_block_lang: String,
    quote_depth: usize,
    in_table: bool,
    in_table_head: bool,
    table_col: usize,
    table_alignments: Vec<Align>,
    table_header_row: Vec<Vec<Span<'static>>>,
    table_body_rows: Vec<Vec<Vec<Span<'static>>>>,
    list_stack: Vec<ListFrame>,
    in_list_item: bool,
    saved_line: Vec<Span<'static>>,
    saved_width: usize,
}

impl Ctx {
    fn new(max_width: usize) -> Self {
        Self {
            lines: Vec::new(),
            current_line: Vec::new(),
            current_width: 0,
            max_width,
            total_lines: 0,
            pending_space: false,
            bold: false,
            italic: false,
            strike: false,
            link_href: String::new(),
            image_alt: None,
            in_code_block: false,
            code_block_lang: String::new(),
            quote_depth: 0,
            in_table: false,
            in_table_head: false,
            table_col: 0,
            table_alignments: Vec::new(),
            table_header_row: Vec::new(),
            table_body_rows: Vec::new(),
            list_stack: Vec::new(),
            in_list_item: false,
            saved_line: Vec::new(),
            saved_width: 0,
        }
    }

    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.handle_start(tag),
            Event::End(tag) => self.handle_end(tag),
            Event::Text(text) => self.handle_text(&text),
            Event::Code(text) => self.handle_code(text),
            Event::SoftBreak | Event::HardBreak => self.handle_break(),
            Event::Rule => self.handle_rule(),
            Event::Html(text) => self.handle_html(&text),
            Event::TaskListMarker(checked) => self.handle_task_marker(checked),
            Event::InlineMath(content) => {
                self.push_word(&format!("${}$", content), Style::default().fg(Color::Cyan));
            }
            Event::DisplayMath(content) => {
                self.push_word(&format!("$${}$$", content), Style::default().fg(Color::Cyan));
            }
            Event::InlineHtml(content) => {
                self.push_word(&content, Style::default().fg(Color::DarkGray));
            }
            Event::FootnoteReference(_) => {}
        }
    }

    fn handle_start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.flush_current_line();
                self.current_line.push(match level {
                    HeadingLevel::H1 => {
                        Span::styled(" #", Style::default().fg(H1_COLOR).add_modifier(Modifier::BOLD))
                    }
                    HeadingLevel::H2 => {
                        Span::styled(" ##", Style::default().fg(H2_COLOR).add_modifier(Modifier::BOLD))
                    }
                    HeadingLevel::H3 => {
                        Span::styled(" ###", Style::default().fg(H3_COLOR).add_modifier(Modifier::BOLD))
                    }
                    HeadingLevel::H4 => {
                        Span::styled(" ####", Style::default().fg(H4_COLOR).add_modifier(Modifier::BOLD))
                    }
                    HeadingLevel::H5 => {
                        Span::styled(
                            " #####",
                            Style::default().fg(H5_COLOR).add_modifier(Modifier::BOLD),
                        )
                    }
                    HeadingLevel::H6 => {
                        Span::styled(
                            " ######",
                            Style::default().fg(H6_COLOR).add_modifier(Modifier::BOLD),
                        )
                    }
                });
                self.current_line.push(Span::raw(" "));
            }
            Tag::Paragraph => {}
            Tag::Emphasis => self.italic = true,
            Tag::Strong => self.bold = true,
            Tag::Strikethrough => self.strike = true,
            Tag::CodeBlock(kind) => {
                self.flush_current_line();
                self.in_code_block = true;
                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                if !self.code_block_lang.is_empty() {
                    self.lines.push(Line::from(Span::raw("")));
                    self.total_lines += 1;
                    let badge = format!(" {} ", self.code_block_lang);
                    self.lines.push(Line::from(Span::styled(
                        badge,
                        Style::default().fg(CODE_BLOCK_FG).add_modifier(Modifier::BOLD),
                    )));
                    self.total_lines += 1;
                }
            }
            Tag::BlockQuote(..) => {
                self.flush_current_line();
                self.quote_depth += 1;
            }
            Tag::List(info) => {
                let ordered = info.is_some();
                let start = info.unwrap_or(1);
                self.list_stack.push(ListFrame { ordered, counter: start });
            }
            Tag::Item => {
                self.flush_current_line();
                self.in_list_item = true;
                let depth = self.list_stack.len();
                let indent = "  ".repeat(depth.saturating_sub(1));

                if let Some(frame) = self.list_stack.last_mut() {
                    if frame.ordered {
                        let num = frame.counter;
                        frame.counter += 1;
                        let marker = format!("{}.", num);
                        self.current_line.push(Span::styled(
                            format!("{}{} ", indent, marker),
                            Style::default().fg(BULLET_FG).add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        let bullet = match depth {
                            1 => "•",
                            2 => "◦",
                            _ => "▪",
                        };
                        self.current_line.push(Span::styled(
                            format!("{}{} ", indent, bullet),
                            Style::default().fg(BULLET_FG),
                        ));
                    }
                }
                self.current_width = self.current_line.iter().map(|s| s.content.len()).sum();
            }
            Tag::Link { dest_url, .. } => {
                self.link_href = dest_url.to_string();
            }
            Tag::Image { dest_url, title, .. } => {
                self.image_alt = if title.is_empty() {
                    Some(format!("[image: {}]", dest_url))
                } else {
                    Some(format!("[image: {}]({})", title, dest_url))
                };
            }
            Tag::Table(alignments) => {
                self.flush_current_line();
                self.in_table = true;
                self.table_alignments = alignments
                    .into_iter()
                    .map(|a| match a {
                        pulldown_cmark::Alignment::None => Align::None,
                        pulldown_cmark::Alignment::Left => Align::Left,
                        pulldown_cmark::Alignment::Center => Align::Center,
                        pulldown_cmark::Alignment::Right => Align::Right,
                    })
                    .collect();
                self.table_col = 0;
                self.table_header_row.clear();
                self.table_body_rows.clear();
            }
            Tag::TableHead => {
                self.in_table_head = true;
            }
            Tag::TableRow => {
                self.table_col = 0;
            }
            Tag::TableCell => {
                self.saved_line = std::mem::take(&mut self.current_line);
                self.saved_width = self.current_width;
                self.current_width = 0;
            }
            Tag::HtmlBlock => {
                self.flush_current_line();
            }
            Tag::MetadataBlock(_) => {
                self.flush_current_line();
            }
            Tag::FootnoteDefinition(_) => {}
        }
    }

    fn handle_end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(level) => {
                let heading_color = match level {
                    HeadingLevel::H1 => H1_COLOR,
                    HeadingLevel::H2 => H2_COLOR,
                    HeadingLevel::H3 => H3_COLOR,
                    HeadingLevel::H4 => H4_COLOR,
                    HeadingLevel::H5 => H5_COLOR,
                    HeadingLevel::H6 => H6_COLOR,
                };
                let styled: Vec<Span> = self
                    .current_line
                    .iter()
                    .map(|s| {
                        let mut style = s.style;
                        style = style.fg(heading_color).add_modifier(Modifier::BOLD);
                        Span::styled(s.content.clone(), style)
                    })
                    .collect();
                if !styled.is_empty() {
                    self.lines.push(Line::from(styled));
                    self.total_lines += 1;
                }
                self.current_line.clear();
                self.current_width = 0;
                self.pending_space = false;
            }
            TagEnd::Paragraph => {
                self.flush_current_line();
            }
            TagEnd::Emphasis => self.italic = false,
            TagEnd::Strong => self.bold = false,
            TagEnd::Strikethrough => self.strike = false,
            TagEnd::CodeBlock => {
                self.flush_current_line();
                self.lines.push(Line::from(Span::raw("")));
                self.total_lines += 1;
                self.in_code_block = false;
                self.code_block_lang.clear();
            }
            TagEnd::BlockQuote => {
                self.flush_current_line();
                self.quote_depth = self.quote_depth.saturating_sub(1);
            }
            TagEnd::List(ordered) => {
                self.flush_current_line();
                if self.list_stack.last().map(|f| f.ordered) == Some(ordered) {
                    self.list_stack.pop();
                }
                self.lines.push(Line::from(Span::raw("")));
                self.total_lines += 1;
            }
            TagEnd::Item => {
                self.flush_current_line();
                self.in_list_item = false;
            }
            TagEnd::Link => {
                if !self.link_href.is_empty() {
                    let url_style = Style::default().fg(LINK_URL_FG).add_modifier(Modifier::ITALIC);
                    self.current_line
                        .push(Span::styled(format!(" ({})", self.link_href), url_style));
                }
                self.link_href.clear();
            }
            TagEnd::Image => {
                if let Some(alt) = self.image_alt.take() {
                    self.current_line.push(Span::styled(
                        alt,
                        Style::default().fg(LINK_URL_FG).add_modifier(Modifier::ITALIC),
                    ));
                }
            }
            TagEnd::Table => {
                self.flush_current_line();
                self.render_table();
                self.in_table = false;
                self.in_table_head = false;
                self.table_header_row.clear();
                self.table_body_rows.clear();
            }
            TagEnd::TableHead => {
                self.in_table_head = false;
            }
            TagEnd::TableRow => {}
            TagEnd::TableCell => {
                let cell = std::mem::take(&mut self.current_line);
                self.current_line = std::mem::take(&mut self.saved_line);
                self.current_width = self.saved_width;

                if self.in_table_head {
                    if self.table_col < self.table_header_row.len() {
                        self.table_header_row[self.table_col] = cell;
                    } else {
                        self.table_header_row.push(cell);
                    }
                } else {
                    while self.table_body_rows.len() <= self.table_col {
                        self.table_body_rows.push(Vec::new());
                    }
                    self.table_body_rows[self.table_col].push(cell);
                }
                self.table_col += 1;
            }
            TagEnd::HtmlBlock => {}
            TagEnd::MetadataBlock(_) => {}
            TagEnd::FootnoteDefinition => {}
        }
    }

    fn handle_text(&mut self, text: &str) {
        if self.in_code_block {
            let style = Style::default().fg(CODE_BLOCK_FG);
            for line in text.split('\n') {
                if !self.current_line.is_empty() {
                    self.flush_current_line();
                }
                self.lines
                    .push(Line::from(Span::styled(line.to_string(), style)));
                self.total_lines += 1;
                self.current_width = 0;
            }
            return;
        }

        let style = self.inline_style();

        let text = text.replace('\n', " ");
        let words: Vec<&str> = text.split_ascii_whitespace().collect();
        if words.is_empty() {
            if text.contains(' ') {
                self.pending_space = true;
            }
            return;
        }

        for word in words {
            self.push_word(word, style);
        }
    }

    fn handle_code(&mut self, text: pulldown_cmark::CowStr) {
        let style = Style::default()
            .fg(CODE_INLINE_FG)
            .bg(CODE_INLINE_BG)
            .add_modifier(Modifier::BOLD);
        let display = format!("`{}`", text);
        self.push_word(&display, style);
    }

    fn handle_break(&mut self) {
        self.flush_current_line();
    }

    fn handle_rule(&mut self) {
        self.flush_current_line();
        let width = self.max_width.min(40);
        self.lines.push(Line::from(Span::styled(
            "─".repeat(width),
            Style::default().fg(HR_FG),
        )));
        self.total_lines += 1;
        self.lines.push(Line::from(Span::raw("")));
        self.total_lines += 1;
    }

    fn handle_html(&mut self, text: &str) {
        let clean = text.replace('<', "&lt;").replace('>', "&gt;");
        self.push_word(&clean, Style::default().fg(Color::DarkGray));
    }

    fn handle_task_marker(&mut self, checked: bool) {
        let (marker, color) = if checked {
            ("[x]", TASK_DONE_FG)
        } else {
            ("[ ]", TASK_PENDING_FG)
        };
        let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        self.current_line.push(Span::styled(marker, style));
        self.current_line.push(Span::raw(" "));
        self.current_width += 4;
    }

    fn inline_style(&self) -> Style {
        let mut style = Style::default();
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.italic {
            style = style.add_modifier(Modifier::ITALIC);
        }
        if self.strike {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }
        if self.quote_depth > 0 {
            style = style.fg(BLOCKQUOTE_FG);
        }
        style
    }

    fn push_word(&mut self, word: &str, style: Style) {
        let word_w = word.len();
        let sep_w = if self.pending_space && !self.current_line.is_empty() {
            1
        } else {
            0
        };

        if self.current_width + sep_w + word_w > self.max_width && !self.current_line.is_empty() {
            self.flush_current_line();
            self.add_line_prefix();
            self.current_line
                .push(Span::styled(word.to_string(), style));
            self.current_width = word_w;
            self.pending_space = false;
        } else {
            if sep_w > 0 {
                self.current_line.push(Span::raw(" "));
                self.current_width += 1;
            }
            self.current_line
                .push(Span::styled(word.to_string(), style));
            self.current_width += word_w;
            self.pending_space = true;
        }
    }

    fn add_line_prefix(&mut self) {
        if self.quote_depth > 0 {
            let bar = "▍ ".repeat(self.quote_depth);
            self.current_line
                .push(Span::styled(bar, Style::default().fg(BLOCKQUOTE_FG)));
        }
        if self.in_list_item {
            let depth = self.list_stack.len();
            let indent = "  ".repeat(depth.saturating_sub(1));
            if let Some(frame) = self.list_stack.last() {
                if frame.ordered {
                    let pad = "   ";
                    self.current_line
                        .push(Span::styled(format!("{}{}", indent, pad), Style::default()));
                } else {
                    self.current_line.push(Span::styled(
                        format!("{}  ", indent),
                        Style::default(),
                    ));
                }
            }
        }
    }

    fn flush_current_line(&mut self) {
        if self.current_line.is_empty() {
            return;
        }
        let mut line = Vec::new();
        if self.quote_depth > 0 {
            let bar = "▍ ".repeat(self.quote_depth);
            line.push(Span::styled(bar, Style::default().fg(BLOCKQUOTE_FG)));
        }
        line.append(&mut self.current_line);
        self.lines.push(Line::from(line));
        self.total_lines += 1;
        self.current_width = 0;
        self.pending_space = false;
    }

    fn finalize_table(&mut self) {}

    fn render_table(&mut self) {
        let col_count = self.table_alignments.len().max(self.table_header_row.len());
        if col_count == 0 {
            return;
        }

        let body_row_count = self
            .table_body_rows
            .iter()
            .map(|col| col.len())
            .max()
            .unwrap_or(0);

        let mut col_widths = vec![0usize; col_count];
        for (ci, hcell) in self.table_header_row.iter().enumerate() {
            let w = hcell.iter().map(|s| s.content.len()).sum::<usize>();
            col_widths[ci] = col_widths[ci].max(w);
        }
        for (ci, col) in self.table_body_rows.iter().enumerate() {
            for row in col {
                let w = row.iter().map(|s| s.content.len()).sum::<usize>();
                col_widths[ci] = col_widths[ci].max(w);
            }
        }
        for w in &mut col_widths {
            *w = (*w).min(self.max_width.saturating_sub(col_count + 1) / col_count.max(1));
        }

        let total_table_width: usize = col_widths.iter().sum::<usize>() + col_count + 1;
        if total_table_width > self.max_width {
            if !self.table_header_row.is_empty() {
                let header_text: String = self
                    .table_header_row
                    .iter()
                    .map(|cell| {
                        cell.iter()
                            .map(|s| s.content.as_ref())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                self.lines.push(Line::from(Span::styled(
                    header_text,
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                self.total_lines += 1;
            }
            for ri in 0..body_row_count {
                let parts: Vec<String> = (0..col_count)
                    .map(|ci| {
                        self.table_body_rows
                            .get(ci)
                            .and_then(|col| col.get(ri))
                            .map(|cell| cell.iter().map(|s| s.content.as_ref()).collect::<String>())
                            .unwrap_or_default()
                    })
                    .collect();
                let line = parts.join(" | ");
                self.lines.push(Line::from(Span::raw(line)));
                self.total_lines += 1;
            }
            self.lines.push(Line::from(Span::raw("")));
            self.total_lines += 1;
            return;
        }

        let top = self.render_table_border(&col_widths, "┌", "┬", "┐");
        self.lines
            .push(Line::from(Span::styled(top, Style::default().fg(Color::DarkGray))));
        self.total_lines += 1;

        let header_spans = self.render_table_row(&self.table_header_row, &col_widths, false);
        self.lines.push(Line::from(header_spans));
        self.total_lines += 1;

        let sep = self.render_table_border(&col_widths, "├", "┼", "┤");
        self.lines
            .push(Line::from(Span::styled(sep, Style::default().fg(Color::DarkGray))));
        self.total_lines += 1;

        for ri in 0..body_row_count {
            let mut row_cells = Vec::new();
            for ci in 0..col_count {
                let cell = self
                    .table_body_rows
                    .get(ci)
                    .and_then(|col| col.get(ri))
                    .cloned()
                    .unwrap_or_default();
                row_cells.push(cell);
            }
            let row_spans = self.render_table_row(&row_cells, &col_widths, false);
            self.lines.push(Line::from(row_spans));
            self.total_lines += 1;
        }

        let bottom = self.render_table_border(&col_widths, "└", "┴", "┘");
        self.lines
            .push(Line::from(Span::styled(bottom, Style::default().fg(Color::DarkGray))));
        self.total_lines += 1;

        self.lines.push(Line::from(Span::raw("")));
        self.total_lines += 1;
    }

    fn render_table_border(&self, col_widths: &[usize], left: &str, sep: &str, right: &str) -> String {
        let mut s = String::from(left);
        for (i, w) in col_widths.iter().enumerate() {
            s.push_str(&"─".repeat(w + 2));
            if i < col_widths.len() - 1 {
                s.push_str(sep);
            }
        }
        s.push_str(right);
        s
    }

    fn render_table_row(
        &self,
        cells: &[Vec<Span<'static>>],
        col_widths: &[usize],
        _is_header: bool,
    ) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        for (ci, cell) in cells.iter().enumerate() {
            let total_w: usize = cell.iter().map(|s| s.content.len()).sum();
            let pad = col_widths
                .get(ci)
                .copied()
                .unwrap_or(total_w)
                .saturating_sub(total_w);
            let align = self.table_alignments.get(ci).unwrap_or(&Align::None);
            let (left_pad, right_pad) = match align {
                Align::Right => (pad, 0),
                Align::Center => (pad / 2, pad - pad / 2),
                _ => (0, pad),
            };

            spans.push(Span::raw(" "));
            if left_pad > 0 {
                spans.push(Span::raw(" ".repeat(left_pad)));
            }
            spans.extend(cell.iter().cloned());
            if right_pad > 0 {
                spans.push(Span::raw(" ".repeat(right_pad)));
            }
            spans.push(Span::raw(" "));
            spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        }
        spans
    }
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
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("Hello") || joined.contains("World"));
    }

    #[test]
    fn test_render_markdown_heading_h1() {
        let (lines, _) = render_markdown("# Title", 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("Title"));
    }

    #[test]
    fn test_render_markdown_heading_h2() {
        let (lines, _) = render_markdown("## Section", 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("Section"));
    }

    #[test]
    fn test_render_markdown_bold() {
        let (lines, _) = render_markdown("**bold text**", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_italic() {
        let (lines, _) = render_markdown("*italic text*", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_strikethrough() {
        let (lines, _) = render_markdown("~~strikethrough~~", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_code_block() {
        let (lines, _) = render_markdown("```\ncode here\n```", 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("code here"));
    }

    #[test]
    fn test_render_markdown_fenced_code_block() {
        let (lines, _) = render_markdown("```rust\nfn main() {}\n```", 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("fn main()"));
    }

    #[test]
    fn test_render_markdown_unordered_list() {
        let (lines, _) = render_markdown("- item 1\n- item 2", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_ordered_list() {
        let (lines, _) = render_markdown("1. first\n2. second", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_blockquote() {
        let (lines, _) = render_markdown("> quoted text", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_inline_code() {
        let (lines, _) = render_markdown("use `let` keyword", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_link() {
        let (lines, _) = render_markdown("[example](https://example.com)", 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("example"));
    }

    #[test]
    fn test_render_markdown_task_list_checked() {
        let (lines, _) = render_markdown("- [x] done", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_task_list_unchecked() {
        let (lines, _) = render_markdown("- [ ] todo", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_horizontal_rule() {
        let (lines, _) = render_markdown("---", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_markdown_mixed_formatting() {
        let md = "# Title\n\n**bold** and *italic* and `code`\n\n- list item";
        let (lines, total) = render_markdown(md, 80);
        assert!(!lines.is_empty());
        assert!(total > 1);
    }

    #[test]
    fn test_render_markdown_word_wrap() {
        let long = "word ";
        let text: String = std::iter::repeat(long).take(30).collect();
        let (lines, _) = render_markdown(&text, 40);
        assert!(lines.len() > 1, "long text should wrap to multiple lines");
    }

    #[test]
    fn test_render_markdown_table() {
        let md = "| H1 | H2 |\n|---|---|\n| A | B |\n| C | D |";
        let (lines, _) = render_markdown(md, 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("H1") || joined.contains("H2"),
            "table header should appear"
        );
    }

    #[test]
    fn test_render_markdown_nested_list() {
        let md = "- outer\n  - inner";
        let (lines, _) = render_markdown(md, 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("outer"));
    }

    #[test]
    fn test_render_markdown_heading_levels() {
        for (md, _level) in [
            ("# H1", 1),
            ("## H2", 2),
            ("### H3", 3),
            ("#### H4", 4),
            ("##### H5", 5),
            ("###### H6", 6),
        ] {
            let (lines, _) = render_markdown(md, 80);
            assert!(!lines.is_empty(), "{} should render", md);
        }
    }

    #[test]
    fn test_render_markdown_soft_break() {
        let (lines, _) = render_markdown("line1\nline2", 80);
        assert!(lines.len() >= 2, "soft break should create new line");
    }

    #[test]
    fn test_render_markdown_code_block_with_lang() {
        let (lines, _) = render_markdown("```python\nprint('hello')\n```", 80);
        assert!(!lines.is_empty());
        let joined: String = lines
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains("python") || joined.contains("print"),
            "code block with lang should show lang and code"
        );
    }
}
