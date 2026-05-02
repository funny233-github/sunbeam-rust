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

/// Top-level renderer that dispatches to the appropriate view.
pub fn render(f: &mut Frame, app: &AppState) {
    let area = f.area();

    if let Some(ref detail) = app.detail {
        render_detail_page(f, area, detail);
        return;
    }

    if let Some(ref form) = app.form {
        render_prefs_form(f, area, form);
        return;
    }

    if let Some(Page::Runner(runner)) = app.page_stack.last() {
        render_extension_list(f, area, runner);
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

    let tui_items: Vec<TuiListItem> = app
        .filtered_items
        .iter()
        .enumerate()
        .map(|(i, idx)| {
            let item = &app.items[*idx];
            let is_selected = i == app.selection;
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
    let list_block = Block::default().borders(Borders::ALL);
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

    let status = Paragraph::new(status_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}

/// Renders a detail page with optional action bar.
fn render_detail_page(f: &mut Frame, area: Rect, detail: &PageDetail) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    let detail_text: &str = if detail.markdown.is_empty() { "No content" } else { &detail.markdown };

    let detail_widget = Paragraph::new(Text::raw(detail_text))
        .block(Block::default().borders(Borders::ALL).title(Line::from(" Detail ")))
        .wrap(Wrap { trim: true });
    f.render_widget(detail_widget, chunks[0]);

    let action_text = if detail.actions.is_empty() {
        " q: back".to_string()
    } else if detail.action_mode {
        detail.actions.iter().enumerate().map(|(i, a)| {
            let title = a.title.as_deref().unwrap_or("");
            if i == detail.inner_selection { format!("[{title}]") } else { format!(" {title} ") }
        }).collect::<Vec<_>>().join("·")
    } else {
        let first = detail.actions.first().map(|a| a.title.as_deref().unwrap_or("")).unwrap_or("");
        format!(" {first} · Actions(tab)")
    };

    let status = Paragraph::new(format!(" q: back | {action_text}"))
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

/// Renders an extension runner page (search or filter mode).
fn render_extension_list(f: &mut Frame, area: Rect, runner: &RunnerPage) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let prompt = if runner.is_loading { "> ◌".to_string() } else { "> ".to_string() };
    let query_display = if runner.is_loading {
        "Searching...".to_string()
    } else if runner.query.is_empty() {
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

    let tui_items: Vec<TuiListItem> = runner
        .filtered_items.iter().enumerate()
        .map(|(i, idx)| {
            let item = &runner.items[*idx];
            let sel = i == runner.selection;
            let style = if sel { Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD) } else { Style::default() };
            let p = if sel { ">" } else { " " };
            let t = format!("{p} {}", item.item.title);
            let s = item.item.subtitle.as_deref().map(|s| format!(" {s}")).unwrap_or_default();
            let a = item.item.accessories.as_ref().map(|a| format!("  {}", a.join(" · "))).unwrap_or_default();
            TuiListItem::new(format!("{t}{s}{a}")).style(style)
        })
        .collect();

    let list = TuiList::new(tui_items)
        .direction(ListDirection::TopToBottom)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(list, chunks[1]);

    let action_text = if runner.actions.is_empty() {
        " No actions".into()
    } else {
        runner.actions.iter().map(|a| a.title.as_deref().unwrap_or("")).collect::<Vec<_>>().join(" · ")
    };
    let status = Paragraph::new(format!(" {action_text} | Esc: back"))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}
