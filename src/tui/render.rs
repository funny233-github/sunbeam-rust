use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::List as TuiList;
use ratatui::widgets::ListItem as TuiListItem;
use ratatui::widgets::{Block, Borders, ListDirection, Paragraph, Wrap};
use ratatui::Frame;

use crate::types::*;
use crate::tui::types::*;
use crate::tui::key;
use crate::tui::render_md;

const SPINNER_CHARS: &[char] = &['◐', '◓', '◑', '◒'];

fn spinner_char(tick: u64) -> char {
    SPINNER_CHARS[(tick as usize) % SPINNER_CHARS.len()]
}

fn pagination_info(current: usize, total: usize, page_size: usize) -> String {
    if total <= page_size {
        return format!(" ({})", total);
    }
    let total_pages = (total + page_size - 1) / page_size;
    format!(" (page {}/{} — {} items)", current + 1, total_pages, total)
}

fn visible_items<'a>(
    filtered: &'a [usize],
    items: &'a [FilterItem],
    page: usize,
    page_size: usize,
) -> Vec<(usize, &'a FilterItem)> {
    let start = page * page_size;
    let end = (start + page_size).min(filtered.len());
    filtered[start..end]
        .iter()
        .enumerate()
        .map(|(i, idx)| (i + start, &items[*idx]))
        .collect()
}

/// Renders a full-screen error page.
fn render_error_page(f: &mut Frame, area: Rect, msg: &str) {
    let lines: Vec<Line> = vec![
        Line::from(Span::styled(" Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::raw(msg)),
        Line::from(""),
        Line::from(Span::styled(" Press Esc to go back", Style::default().fg(Color::DarkGray))),
    ];
    let para = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(Line::from(" Error ")));
    f.render_widget(para, area);
}

/// Top-level renderer that dispatches to the appropriate view.
pub fn render(f: &mut Frame, app: &AppState) {
    let area = f.area();

    if let Some(ref msg) = app.err {
        render_error_page(f, area, msg);
        return;
    }

    if let Some(ref detail) = app.detail {
        render_detail_page(f, area, detail);
        return;
    }

    if let Some(ref form) = app.form {
        render_prefs_form(f, area, form);
        return;
    }

    if let Some(Page::Runner(runner)) = app.page_stack.last() {
        render_extension_list(f, area, runner, app.tick);
        return;
    }

    render_root_list(f, area, app);
}

/// Renders the root list with search bar, item list, and action bar.
fn render_root_list(f: &mut Frame, area: Rect, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let search_style = if app.action_mode {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let search_text = if app.action_mode {
        "Search Actions..."
    } else {
        "Search Items..."
    };
    let query_display = if app.query.is_empty() {
        search_text.to_string()
    } else {
        app.query.clone()
    };
    let search = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::styled(&query_display, search_style),
    ]))
    .block(Block::default().borders(Borders::ALL).title(Line::from(" Sunbeam ")));
    f.render_widget(search, chunks[0]);

    let vis_items = visible_items(&app.filtered_items, &app.items, app.page, app.page_size);
    let tui_items: Vec<TuiListItem> = vis_items
        .iter()
        .map(|(i, item)| {
            let is_selected = *i == app.selection;
            let style = if is_selected {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if is_selected { ">" } else { " " };
            let title = format!("{} {}", prefix, item.item.title);
            let subtitle = item.item.subtitle.as_deref().map(|s| format!(" {}", s)).unwrap_or_default();
            let accessories = item.item.accessories.as_ref().map(|a| format!("  {}", a.join(" · "))).unwrap_or_default();
            TuiListItem::new(format!("{title}{subtitle}{accessories}")).style(style)
        })
        .collect();

    let list = TuiList::new(tui_items).direction(ListDirection::TopToBottom);
    let mut list_block = Block::default().borders(Borders::ALL);
    let page_info = pagination_info(app.page, app.filtered_items.len(), app.page_size);
    list_block = list_block.title(Line::from(format!(" Items{page_info} ")));
    let list_widget = list.block(list_block);
    f.render_widget(list_widget, chunks[1]);

    let actions = key::get_current_list_actions(app);
    let action_text = if actions.is_empty() {
        " No actions".to_string()
    } else if app.action_mode {
        actions.iter().enumerate().map(|(i, a)| {
            let title = a.title.as_deref().unwrap_or("");
            let key_hint = if i == 0 { "enter" } else if i == 1 { "alt+enter" } else { a.key.as_deref().unwrap_or("") };
            let selected = i == app.action_selection;
            let hint = if !key_hint.is_empty() { format!(" {} ", key_hint) } else { String::new() };
            if selected { format!("[{title} {hint}]") } else { format!(" {title} {hint} ") }
        }).collect::<Vec<_>>().join("·")
    } else {
        let first = actions.first().map(|a| a.title.as_deref().unwrap_or("")).unwrap_or("");
        format!(" {first} · Actions(tab)")
    };

    let status_text = if !app.notification.is_empty() {
        format!(" {}   {}", app.notification, action_text)
    } else {
        action_text
    };
    let page_nav = format!("  PgUp/PgDn  |{status_text}");
    let status = Paragraph::new(page_nav).block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}

/// Renders a detail page with markdown rendering and viewport scrolling.
fn render_detail_page(f: &mut Frame, area: Rect, detail: &PageDetail) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let content_area = chunks[0];
    let inner_height = (content_area.height.max(3) - 2) as usize;

    // Use pre-rendered lines if available, else render now
    let rendered = if !detail.rendered_lines.is_empty() {
        detail.rendered_lines.clone()
    } else if !detail.markdown.is_empty() {
        let (lines, _) = render_md::render_markdown(&detail.markdown, content_area.width as usize);
        lines
    } else {
        vec![Line::from(Span::raw("No content"))]
    };

    let scroll = detail.scroll_offset.min(rendered.len().saturating_sub(inner_height));
    let end = (scroll + inner_height).min(rendered.len());
    let visible: Vec<Line> = if scroll < rendered.len() {
        rendered[scroll..end].to_vec()
    } else {
        vec![Line::from(Span::raw(""))]
    };

    let content = Paragraph::new(Text::from(visible))
        .block(Block::default().borders(Borders::ALL).title(Line::from(" Detail ")));
    f.render_widget(content, chunks[0]);

    let scroll_info = if rendered.len() > inner_height {
        format!(" (scroll {}/{}) ", scroll + 1, rendered.len())
    } else {
        String::new()
    };

    let action_text = if detail.actions.is_empty() {
        " q: back".to_string()
    } else if detail.action_mode {
        let query = &detail.action_query;
        let query_display = if query.is_empty() {
            " Filter actions...".to_string()
        } else {
            format!(" filter: {query}")
        };
        let filtered: Vec<&Action> = if query.is_empty() {
            detail.actions.iter().collect()
        } else {
            let q = query.to_lowercase();
            detail.actions.iter()
                .filter(|a| a.title.as_deref().map_or(false, |t| t.to_lowercase().contains(&q)))
                .collect()
        };
        let action_list = if filtered.is_empty() {
            " no matches ".into()
        } else {
            filtered.iter().enumerate().map(|(i, a)| {
                let title = a.title.as_deref().unwrap_or("");
                if i == detail.inner_selection { format!("[{title}]") } else { format!(" {title} ") }
            }).collect::<Vec<_>>().join("·")
        };
        format!("{query_display} |{action_list}")
    } else {
        let first = detail.actions.first().map(|a| a.title.as_deref().unwrap_or("")).unwrap_or("");
        format!(" {first} · Actions(tab)")
    };

    let status = Paragraph::new(format!(" q: back{scroll_info}| {action_text}"))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[1]);
}

/// Renders a preference configuration form.
fn render_prefs_form(f: &mut Frame, area: Rect, form: &FormState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let mut lines = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        let sel = i == form.selection;
        let title_style = if sel { Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD) } else { Style::default() };
        lines.push(Line::from(vec![Span::styled(format!("{}: ", field.input.title), title_style)]));
        let value = match field.input.input_type {
            InputType::Boolean => if field.checked { "[x]" } else { "[ ]" }.into(),
            _ => if field.value.is_empty() { field.input.title.clone() } else { field.value.clone() },
        };
        let value_style = if sel { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Gray) };
        lines.push(Line::from(vec![Span::styled(format!("  {value}"), value_style)]));
        lines.push(Line::from(""));
    }

    let content = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(Line::from(form.title.clone())))
        .wrap(Wrap { trim: true });
    f.render_widget(content, chunks[0]);

    let status = Paragraph::new(" Tab: next  Alt+Enter: submit  Esc: cancel ")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[1]);
}

/// Renders an extension runner page (search or filter mode), optionally with a side detail panel.
fn render_extension_list(f: &mut Frame, area: Rect, runner: &RunnerPage, tick: u64) {
    // If show_detail is enabled, split into list+detail panels
    if runner.show_detail {
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);
        render_extension_list_inner(f, horiz[0], runner, tick);
        render_item_detail_panel(f, horiz[1], runner);
        return;
    }
    render_extension_list_inner(f, area, runner, tick);
}

/// Renders the side detail panel for the currently selected list item.
fn render_item_detail_panel(f: &mut Frame, area: Rect, runner: &RunnerPage) {
    let idx = runner.filtered_items.get(runner.selection).copied().unwrap_or(0);
    let detail_text = runner.items.get(idx)
        .and_then(|item| item.item.detail.as_ref())
        .and_then(|d| d.markdown.as_deref().or(d.text.as_deref()))
        .unwrap_or("");

    let (lines, _) = render_md::render_markdown(detail_text, area.width.saturating_sub(2) as usize);
    let visible: Vec<Line> = lines.into_iter().take(area.height.saturating_sub(2) as usize).collect();

    let content = if visible.is_empty() {
        Paragraph::new(Text::raw(""))
    } else {
        Paragraph::new(Text::from(visible))
    };
    let panel = content.block(Block::default().borders(Borders::ALL).title(Line::from(" Detail ")));
    f.render_widget(panel, area);
}

/// Renders an extension runner page (search or filter mode).
fn render_extension_list_inner(f: &mut Frame, area: Rect, runner: &RunnerPage, tick: u64) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let spinner = spinner_char(tick);
    let prompt = if runner.is_loading { format!("> {spinner} ") } else { "> ".to_string() };
    let query_display = if runner.query.is_empty() {
        "Search Items...".to_string()
    } else {
        runner.query.clone()
    };
    let search = Paragraph::new(Line::from(vec![
        Span::styled(prompt, Style::default().fg(Color::Cyan)),
        Span::raw(query_display),
    ]))
    .block(Block::default().borders(Borders::ALL).title(Line::from(" Extension ")));
    f.render_widget(search, chunks[0]);

    let runner_page = runner.page;
    let runner_page_size = runner.page_size;
    let vis_items = visible_items(&runner.filtered_items, &runner.items, runner_page, runner_page_size);
    let tui_items: Vec<TuiListItem> = vis_items
        .iter()
        .map(|(i, item)| {
            let sel = *i == runner.selection;
            let style = if sel { Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD) } else { Style::default() };
            let p = if sel { ">" } else { " " };
            let t = format!("{p} {}", item.item.title);
            let s = item.item.subtitle.as_deref().map(|s| format!(" {s}")).unwrap_or_default();
            let a = item.item.accessories.as_ref().map(|a| format!("  {}", a.join(" · "))).unwrap_or_default();
            TuiListItem::new(format!("{t}{s}{a}")).style(style)
        })
        .collect();

    let page_info = pagination_info(runner_page, runner.filtered_items.len(), runner_page_size);
    let list = TuiList::new(tui_items)
        .direction(ListDirection::TopToBottom)
        .block(Block::default().borders(Borders::ALL).title(Line::from(format!(" Items{page_info} "))));
    f.render_widget(list, chunks[1]);

    // Resolve actions: selected item's actions first, fall back to page-level
    let idx = runner.filtered_items.get(runner.selection).copied().unwrap_or(0);
    let selected_actions = runner.items.get(idx)
        .and_then(|item| item.item.actions.as_ref())
        .filter(|a| !a.is_empty())
        .unwrap_or(&runner.actions);
    let action_text = if selected_actions.is_empty() {
        " No actions".into()
    } else {
        selected_actions.iter().map(|a| a.title.as_deref().unwrap_or("")).collect::<Vec<_>>().join(" · ")
    };
    let status = Paragraph::new(format!(" {action_text} | Esc: back"))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}
