use std::time::{Duration, Instant};

use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;

use crate::extensions;
use crate::types::*;
use crate::tui::types::*;
use crate::tui::runner;

/// Processes a single keyboard event and mutates the application state.
///
/// Returns `false` when the application should exit.
pub fn handle_key(app: &mut AppState, key: KeyEvent) -> Result<bool> {
    let runner_page = app.page_stack.len() > 1
        && matches!(app.page_stack.last(), Some(Page::Runner(_)));
    if runner_page {
        return handle_runner_key(app, key);
    }
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(false);
        }

        KeyCode::Esc => {
            if app.err.is_some() {
                app.err = None;
                return Ok(true);
            }
            if app.action_mode {
                app.action_mode = false;
                app.action_selection = 0;
                return Ok(true);
            }
            if app.detail.as_ref().map(|d| d.action_mode).unwrap_or(false) {
                app.detail.as_mut().unwrap().action_mode = false;
                app.action_selection = 0;
                return Ok(true);
            }
            if app.detail.is_some() {
                app.detail = None;
                return Ok(true);
            }
            if app.form.is_some() {
                app.form = None;
                return Ok(true);
            }
            if app.page_stack.len() > 1 {
                app.page_stack.pop();
                return Ok(true);
            }
            return Ok(false);
        }

        KeyCode::Enter => {
            if app.action_mode {
                let action = get_selected_action(app);
                if let Some(action) = action {
                    return dispatch_action(app, action);
                }
                return Ok(true);
            }
            if let Some(form) = app.form.take() {
                let result = runner::submit_form(app, form);
                return result;
            }
            if let Some(detail) = app.detail.as_ref() {
                if detail.actions.is_empty() {
                    return Ok(true);
                }
                let action = if detail.action_mode && detail.inner_selection < detail.actions.len() {
                    detail.actions[detail.inner_selection].clone()
                } else if !detail.actions.is_empty() {
                    detail.actions[0].clone()
                } else {
                    return Ok(true);
                };
                app.detail.as_mut().unwrap().action_mode = false;
                return dispatch_action(app, action);
            }
            let idx = *app.filtered_items.get(app.selection).unwrap_or(&0);
            if let Some(item) = app.items.get(idx) {
                let item_actions = item.item.actions.as_ref().cloned().unwrap_or_default();
                if item_actions.is_empty() {
                    return Ok(true);
                }
                let action = item_actions[0].clone();
                let key = item.item.id.as_deref().unwrap_or(&item.item.title).to_string();
                app.history.update(&key);
                return dispatch_action(app, action);
            }
        }

        KeyCode::Backspace => {
            app.query.pop();
            if !app.action_mode {
                app.filtered_items = filter_items(&app.items, &app.query)
                    .iter().map(|(i, _)| *i).collect();
                if !app.filtered_items.is_empty() {
                    app.selection = 0;
                }
                clamp_page_to_selection(app.filtered_items.len(), &mut app.selection, &mut app.page, app.page_size);
            }
        }

        KeyCode::Tab => {
            if let Some(detail) = app.detail.as_ref() {
                if detail.actions.len() > 1 {
                    app.detail.as_mut().unwrap().action_mode = true;
                    return Ok(true);
                }
            }
            let actions = get_current_list_actions(app);
            if actions.len() > 1 {
                app.action_mode = !app.action_mode;
                app.action_selection = 0;
            }
        }

        KeyCode::PageUp => {
            app.page = app.page.saturating_sub(1);
            clamp_page_to_selection(app.filtered_items.len(), &mut app.selection, &mut app.page, app.page_size);
        }
        KeyCode::PageDown => {
            let max_page = (app.filtered_items.len().max(1) - 1) / app.page_size.max(1);
            if app.page < max_page {
                app.page += 1;
            }
            clamp_page_to_selection(app.filtered_items.len(), &mut app.selection, &mut app.page, app.page_size);
        }
        KeyCode::Char('g') if !app.action_mode => {
            app.page = 0;
            app.selection = 0;
        }
        KeyCode::Char('G') if !app.action_mode => {
            let max_page = (app.filtered_items.len().max(1) - 1) / app.page_size.max(1);
            app.page = max_page;
            app.selection = app.filtered_items.len().saturating_sub(1);
        }
        KeyCode::Up => {
            if let Some(ref mut detail) = app.detail {
                if detail.action_mode {
                    if detail.inner_selection > 0 {
                        detail.inner_selection -= 1;
                    }
                } else if detail.scroll_offset > 0 {
                    detail.scroll_offset -= 1;
                }
                return Ok(true);
            }
            if app.action_mode {
                if app.action_selection > 0 {
                    app.action_selection -= 1;
                }
                return Ok(true);
            }
            if app.selection > 0 {
                app.selection -= 1;
            }
            clamp_page_to_selection(app.filtered_items.len(), &mut app.selection, &mut app.page, app.page_size);
        }
        KeyCode::Down => {
            if let Some(ref mut detail) = app.detail {
                if detail.action_mode {
                    if detail.inner_selection + 1 < detail.actions.len() {
                        detail.inner_selection += 1;
                    }
                } else {
                    detail.scroll_offset += 1;
                }
                return Ok(true);
            }
            if app.action_mode {
                let actions = get_current_list_actions(app);
                if app.action_selection + 1 < actions.len() {
                    app.action_selection += 1;
                }
                return Ok(true);
            }
            if app.selection + 1 < app.filtered_items.len() {
                app.selection += 1;
            }
            clamp_page_to_selection(app.filtered_items.len(), &mut app.selection, &mut app.page, app.page_size);
        }

        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Ok(new_cfg) = crate::config::load(&app.config_path) {
                app.config = new_cfg;
                let mut new_items = Vec::new();
                if let Some(oneliners) = &app.config.oneliners {
                    for o in oneliners {
                        new_items.push(FilterItem {
                            filter_text: o.title.clone(),
                            item: ListItem {
                                id: Some(format!("oneliner - {}", o.title)),
                                title: o.title.clone(),
                                subtitle: None,
                                detail: None,
                                accessories: Some(vec!["Oneliner".to_string()]),
                                actions: Some(vec![Action {
                                    title: Some("Run".to_string()),
                                    key: None,
                                    action_type: ActionType::Exec,
                                    open: None, copy: None, run: None,
                                    exec: Some(ExecAction {
                                        command: o.command.clone(),
                                        interactive: o.interactive,
                                        dir: o.cwd.clone(),
                                        exit: o.exit,
                                    }),
                                    edit: None, config: None, reload: None,
                                }]),
                            },
                        });
                    }
                }
                if let Some(exts) = &app.config.extensions {
                    for (alias, ext_cfg) in exts {
                        if let Ok(extension) = extensions::load_extension(&ext_cfg.origin) {
                            for cmd in extension.root_commands() {
                                let title = cmd.title.clone();
                                new_items.push(FilterItem {
                                    filter_text: format!("{} {}", title, extension.manifest.title),
                                    item: ListItem {
                                        id: Some(format!("{} - {}", alias, cmd.name)),
                                        title,
                                        subtitle: Some(extension.manifest.title.clone()),
                                        detail: None,
                                        accessories: Some(vec!["Command".to_string()]),
                                        actions: Some(vec![Action {
                                            title: Some("Run".to_string()),
                                            key: None,
                                            action_type: ActionType::Run,
                                            open: None, copy: None,
                                            run: Some(RunAction {
                                                extension: Some(alias.clone()),
                                                command: cmd.name.clone(),
                                                params: None, reload: None, exit: None,
                                            }),
                                            exec: None, edit: None, config: None, reload: None,
                                        }]),
                                    },
                                });
                            }
                        }
                    }
                }
                app.items = new_items;
                app.filtered_items = filter_items(&app.items, &app.query)
                    .iter().map(|(i, _)| *i).collect();
                if !app.filtered_items.is_empty() {
                    app.selection = 0;
                }
            }
        }

        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let trimmed = app.query.trim_end().to_string();
            let len = trimmed.len();
            if let Some(pos) = trimmed[..len].rfind(char::is_whitespace) {
                app.query.truncate(pos + 1);
            } else {
                app.query.clear();
            }
            if !app.action_mode {
                app.filtered_items = filter_items(&app.items, &app.query)
                    .iter().map(|(i, _)| *i).collect();
                if !app.filtered_items.is_empty() {
                    app.selection = 0;
                }
                clamp_page_to_selection(app.filtered_items.len(), &mut app.selection, &mut app.page, app.page_size);
            }
        }

        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let editor = crate::utils::find_editor();
            terminal::disable_raw_mode()?;
            let _ = std::process::Command::new("sh")
                .args(["-c", &format!("{} {}", editor, app.config_path.display())])
                .status();
            terminal::enable_raw_mode()?;
            if let Ok(cfg) = crate::config::load(&app.config_path) {
                app.config = cfg;
            }
        }

        KeyCode::Char(c) => {
            // Detail action mode: filter actions by name
            if let Some(ref mut detail) = app.detail {
                if detail.action_mode {
                    if c == '\n' { return Ok(true); }
                    detail.action_query.push(c);
                    // Filter actions by fuzzy matching name
                    let q = detail.action_query.to_lowercase();
                    let matching: Vec<usize> = detail.actions.iter().enumerate()
                        .filter(|(_, a)| a.title.as_deref().is_some_and(|t| t.to_lowercase().contains(&q)))
                        .map(|(i, _)| i)
                        .collect();
                    if !matching.is_empty() {
                        detail.inner_selection = matching[0];
                    }
                    return Ok(true);
                }
                // Not in action mode — ignore char input
                return Ok(true);
            }
            if app.action_mode {
                app.query.push(c);
                return Ok(true);
            }
            app.query.push(c);
            app.filtered_items = filter_items(&app.items, &app.query)
                .iter().map(|(i, _)| *i).collect();
            if !app.filtered_items.is_empty() {
                app.selection = 0;
            }
            clamp_page_to_selection(app.filtered_items.len(), &mut app.selection, &mut app.page, app.page_size);
        }

        _ => {}
    }
    Ok(true)
}

/// Handles keyboard input when a runner page is active.
fn handle_runner_key(app: &mut AppState, key: KeyEvent) -> Result<bool> {
    let mut runner = match app.page_stack.pop() {
        Some(Page::Runner(r)) => r,
        _ => return Ok(true),
    };

    // For query-modifying keys, update the runner's query then re-search
    // and push back. For other keys, keep the result and push at the end.
    match key.code {
        KeyCode::Esc => {
            app.detail = None;
            app.form = None;
            return Ok(true);
        }
        KeyCode::Enter => {
            let idx = *runner.filtered_items.get(runner.selection).unwrap_or(&0);
            if let Some(item) = runner.items.get(idx) {
                let actions = item.item.actions.as_ref().cloned().unwrap_or_default();
                if let Some(action) = actions.first().cloned() {
                    // Same-extension Run action: use runner's extension directly
                    // instead of going through handle_run_action (which requires
                    // an extension alias from config).
                    if action.action_type == ActionType::Run {
                        if let Some(ref run) = action.run {
                            if run.extension.is_none() {
                                if let Some(ref ext) = runner.extension {
                                    let payload = Payload {
                                        command: run.command.clone(),
                                        preferences: runner.preferences.clone(),
                                        params: run.params.clone(),
                                        cwd: None,
                                        r#query: None,
                                    };
                                    let cmd_mode = ext.command(&run.command)
                                        .map(|c| c.mode.clone().unwrap_or(CommandMode::Filter))
                                        .unwrap_or(CommandMode::Filter);
                                    let result = match cmd_mode {
                                        CommandMode::Search | CommandMode::Filter =>
                                            crate::tui::runner::run_extension_list(app, ext.clone(), payload),
                                        CommandMode::Detail =>
                                            crate::tui::runner::run_extension_detail(app, ext.clone(), payload),
                                        CommandMode::Silent => {
                                            terminal::disable_raw_mode()?;
                                            let r = ext.run_quiet(&payload);
                                            terminal::enable_raw_mode()?;
                                            if let Err(e) = r {
                                                app.notification = format!("Error: {}", e);
                                                app.notification_until = Some(Instant::now() + Duration::from_secs(2));
                                            }
                                            Ok(true)
                                        }
                                        CommandMode::Tty => {
                                            let mut cmd = ext.cmd(&payload)?;
                                            terminal::disable_raw_mode()?;
                                            let r = cmd.spawn().and_then(|mut c| c.wait());
                                            terminal::enable_raw_mode()?;
                                            if let Err(e) = r {
                                                app.notification = format!("Error: {}", e);
                                                app.notification_until = Some(Instant::now() + Duration::from_secs(2));
                                            }
                                            Ok(true)
                                        }
                                    };
                                    match result {
                                        Ok(true) => {
                                            if app.page_stack.is_empty() {
                                                app.page_stack.push(Page::Runner(runner));
                                            }
                                            return Ok(true);
                                        }
                                        Ok(false) => return Ok(false),
                                        Err(e) => {
                                            app.page_stack.push(Page::Runner(runner));
                                            return Err(e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let result = dispatch_action(app, action);
                    match result {
                        Ok(true) => {
                            if app.page_stack.is_empty() {
                                app.page_stack.push(Page::Runner(runner));
                            }
                            return Ok(true);
                        }
                        Ok(false) => return Ok(false),
                        Err(e) => {
                            app.page_stack.push(Page::Runner(runner));
                            return Err(e);
                        }
                    }
                }
            }
            app.page_stack.push(Page::Runner(runner));
            return Ok(true);
        }
        KeyCode::PageUp => {
            runner.page = runner.page.saturating_sub(1);
            clamp_page_to_selection(runner.filtered_items.len(), &mut runner.selection, &mut runner.page, runner.page_size);
        }
        KeyCode::PageDown => {
            let max_page = (runner.filtered_items.len().max(1) - 1) / runner.page_size.max(1);
            if runner.page < max_page {
                runner.page += 1;
            }
            clamp_page_to_selection(runner.filtered_items.len(), &mut runner.selection, &mut runner.page, runner.page_size);
        }
        KeyCode::Up => {
            if runner.selection > 0 { runner.selection -= 1; }
            clamp_page_to_selection(runner.filtered_items.len(), &mut runner.selection, &mut runner.page, runner.page_size);
            app.page_stack.push(Page::Runner(runner));
            return Ok(true);
        }
        KeyCode::Down => {
            if runner.selection + 1 < runner.filtered_items.len() {
                runner.selection += 1;
            }
            clamp_page_to_selection(runner.filtered_items.len(), &mut runner.selection, &mut runner.page, runner.page_size);
            app.page_stack.push(Page::Runner(runner));
            return Ok(true);
        }
        KeyCode::Backspace => {
            runner.query.pop();
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(ref ext) = runner.extension.clone() {
                let editor = crate::utils::find_editor();
                let entrypoint = ext.entrypoint.clone();
                terminal::disable_raw_mode()?;
                let status = std::process::Command::new("sh")
                    .args(["-c", &format!("{} {}", editor, entrypoint.display())])
                    .status();
                terminal::enable_raw_mode()?;
                if status.is_ok() {
                    if let Ok(ext) = crate::extensions::load_extension(&entrypoint.to_string_lossy()) {
                        runner.extension = Some(ext);
                    }
                }
            }
            app.page_stack.push(Page::Runner(runner));
            return Ok(true);
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(ref ext) = runner.extension.clone() {
                let entrypoint = ext.entrypoint.clone();
                if let Ok(ext) = crate::extensions::load_extension(&entrypoint.to_string_lossy()) {
                    runner.extension = Some(ext);
                }
            }
            app.page_stack.push(Page::Runner(runner));
            return Ok(true);
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let trimmed = runner.query.trim_end().to_string();
            if let Some(pos) = trimmed[..trimmed.len()].rfind(char::is_whitespace) {
                runner.query.truncate(pos + 1);
            } else {
                runner.query.clear();
            }
        }
        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) => {
            // Alt+letter: dispatch the action with a matching .key field
            let idx = *runner.filtered_items.get(runner.selection).unwrap_or(&0);
            if let Some(item) = runner.items.get(idx) {
                if let Some(actions) = item.item.actions.as_ref() {
                    if let Some(action) = actions.iter().find(|a| a.key.as_deref() == Some(&c.to_string())).cloned() {
                        app.page_stack.push(Page::Runner(runner));
                        return dispatch_action(app, action);
                    }
                }
            }
            app.page_stack.push(Page::Runner(runner));
            return Ok(true);
        }
        KeyCode::Char(c) => {
            runner.query.push(c);
        }
        _ => {
            app.page_stack.push(Page::Runner(runner));
            return Ok(true);
        }
    }

    // Apply search (server-side) or filter (client-side) after query change.
    if runner.mode == CommandMode::Search {
        if runner.query.is_empty() {
            // No query: clear pending, show all items.
            runner.pending_query = String::new();
            runner.is_loading = false;
            runner.filtered_items = (0..runner.items.len()).collect();
        } else {
            // Debounce: record the query and timestamp; the event loop will
            // call reload_runner after ~300ms of no keystrokes.
            runner.pending_query = runner.query.clone();
            app.last_search_keystroke = Instant::now();
            runner.is_loading = true;
        }
    } else {
        runner.filtered_items = filter_items(&runner.items, &runner.query)
            .iter().map(|(i, _)| *i).collect();
    }
    if !runner.filtered_items.is_empty() { runner.selection = 0; }
    clamp_page_to_selection(runner.filtered_items.len(), &mut runner.selection, &mut runner.page, runner.page_size);

    app.page_stack.push(Page::Runner(runner));
    Ok(true)
}

/// Returns the actions for the currently selected item or page.
pub fn get_current_list_actions(app: &AppState) -> Vec<Action> {
    if let Some(Page::Runner(runner)) = app.page_stack.last() {
        let idx = runner.filtered_items.get(runner.selection).copied().unwrap_or(0);
        if let Some(item) = runner.items.get(idx) {
            return item.item.actions.as_ref().cloned().unwrap_or_default();
        }
        return runner.actions.clone();
    }

    let idx = app.filtered_items.get(app.selection).copied().unwrap_or(0);
    if let Some(item) = app.items.get(idx) {
        return item.item.actions.as_ref().cloned().unwrap_or_default();
    }
    app.actions.clone()
}

/// Returns the action currently selected in the action bar.
pub fn get_selected_action(app: &AppState) -> Option<Action> {
    let actions = get_current_list_actions(app);
    actions.get(app.action_selection).cloned()
}

fn handle_run_action(app: &mut AppState, run: RunAction) -> Result<bool> {
    let extension_alias = match &run.extension {
        Some(alias) => alias.clone(),
        None => {
            app.notification = "Error: action missing extension reference".to_string();
            app.notification_until = Some(Instant::now() + Duration::from_secs(2));
            return Ok(true);
        }
    };
    let exts = match &app.config.extensions {
        Some(e) => e,
        None => return Ok(true),
    };
    let ext_cfg = match exts.get(&extension_alias) {
        Some(c) => c,
        None => {
            app.notification = format!("Error: extension '{}' not found", extension_alias);
            app.notification_until = Some(Instant::now() + Duration::from_secs(2));
            return Ok(true);
        }
    };
    let origin = ext_cfg.origin.clone();
    let extension = match extensions::load_extension(&origin) {
        Ok(e) => e,
        Err(_) => return Ok(true),
    };

    let payload = Payload {
        command: run.command.clone(),
        preferences: ext_cfg.preferences.clone(),
        params: run.params.clone(),
        cwd: None,
        r#query: None,
    };

    let cmd_mode = extension
        .command(&run.command)
        .map(|c| c.mode.clone().unwrap_or(CommandMode::Filter))
        .unwrap_or(CommandMode::Filter);

    match cmd_mode {
        CommandMode::Search | CommandMode::Filter => {
            runner::run_extension_list(app, extension, payload)
        }
        CommandMode::Detail => runner::run_extension_detail(app, extension, payload),
        CommandMode::Silent => {
            terminal::disable_raw_mode()?;
            let result = extension.run_quiet(&payload);
            let _ = terminal::enable_raw_mode();
            if let Err(e) = result {
                app.notification = format!("Error: {}", e);
                app.notification_until = Some(Instant::now() + Duration::from_secs(2));
            }
            Ok(true)
        }
        CommandMode::Tty => {
            let mut cmd = extension.cmd(&payload)?;
            terminal::disable_raw_mode()?;
            let result = cmd.spawn().and_then(|mut child| child.wait());
            let _ = terminal::enable_raw_mode();
            if let Err(e) = result {
                app.notification = format!("Error: {}", e);
                app.notification_until = Some(Instant::now() + Duration::from_secs(2));
            }
            Ok(true)
        }
    }
}

fn handle_copy_action(app: &mut AppState, copy: CopyAction) -> Result<bool> {
    if let Some(text) = copy.text {
        if let Ok(mut clipboard) = Clipboard::new() {
            clipboard.set_text(text).ok();
        }
        app.notification = "Copied!".to_string();
        app.notification_until = Some(Instant::now() + Duration::from_secs(1));
        if copy.exit.unwrap_or(false) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn handle_config_action(app: &mut AppState, config_action: ConfigAction) -> Result<bool> {
    let alias = config_action.extension;
    let ext_cfg = match app.config.extensions.as_ref().and_then(|e| e.get(&alias)) {
        Some(c) => c,
        None => return Ok(true),
    };
    let extension = match extensions::load_extension(&ext_cfg.origin) {
        Ok(e) => e,
        Err(_) => return Ok(true),
    };

    let prefs = extension.manifest.preferences.unwrap_or_default();
    let inputs: Vec<Input> = prefs
        .into_iter()
        .map(|mut p| {
            if let Some(prefs_map) = &ext_cfg.preferences {
                if let Some(val) = prefs_map.get(&p.name) {
                    p.default = Some(val.clone());
                }
            }
            p.optional = Some(false);
            p
        })
        .collect();

    let fields: Vec<FormField> = inputs
        .into_iter()
        .map(|input| {
            let (value, checked) = match &input.default {
                Some(v) if input.input_type == InputType::Boolean => {
                    (String::new(), v.as_bool().unwrap_or(false))
                }
                Some(v) => (v.as_str().unwrap_or("").to_string(), false),
                None => (String::new(), false),
            };
            FormField {
                input,
                value,
                checked,
            }
        })
        .collect();

    app.form = Some(FormState {
        title: format!("Configure {}", alias),
        fields,
        selection: 0,
        ext_cfg: ext_cfg.clone(),
        alias: alias.clone(),
    });
    Ok(true)
}

/// Dispatches an action: executes the behaviour associated with the action type.
pub fn dispatch_action(app: &mut AppState, action: Action) -> Result<bool> {
    match action.action_type {
        ActionType::Run => {
            if let Some(run) = action.run {
                return handle_run_action(app, run);
            }
        }
        ActionType::Copy => {
            if let Some(copy) = action.copy {
                return handle_copy_action(app, copy);
            }
        }
        ActionType::Open => {
            if let Some(open) = action.open {
                if let Some(url) = open.url {
                    crate::utils::open_target(&url).ok();
                } else if let Some(path) = open.path {
                    crate::utils::open_target(&format!("file://{}", path)).ok();
                }
                return Ok(false);
            }
        }
        ActionType::Edit => {
            if let Some(edit) = action.edit {
                let editor = crate::utils::find_editor();
                terminal::disable_raw_mode()?;
                let _ = std::process::Command::new("sh")
                    .args(["-c", &format!("{} {}", editor, edit.path)])
                    .status();
                terminal::enable_raw_mode()?;
                if edit.exit.unwrap_or(false) {
                    return Ok(false);
                }
            }
        }
        ActionType::Exec => {
            if let Some(exec) = action.exec {
                let interactive = exec.interactive.unwrap_or(false);
                if interactive {
                    let dir = exec.dir.clone().unwrap_or_default();
                    terminal::disable_raw_mode()?;
                    let mut cmd = std::process::Command::new("sh");
                    cmd.args(["-c", &exec.command]);
                    if !dir.is_empty() {
                        cmd.current_dir(&dir);
                    }
                    let mut child = cmd.spawn()?;
                    child.wait()?;
                    terminal::enable_raw_mode()?;
                } else {
                    let dir = exec.dir.clone().unwrap_or_default();
                    let mut cmd = std::process::Command::new("sh");
                    cmd.args(["-c", &exec.command]);
                    if !dir.is_empty() {
                        cmd.current_dir(&dir);
                    }
                    if let Ok(output) = cmd.output() {
                        if !output.status.success() {
                            app.notification = "Command failed".to_string();
                        } else if !output.stdout.is_empty() {
                            let text =
                                String::from_utf8_lossy(&output.stdout).trim().to_string();
                            app.notification = text;
                            app.notification_until =
                                Some(Instant::now() + Duration::from_secs(2));
                        }
                    }
                }
                if exec.exit.unwrap_or(false) {
                    return Ok(false);
                }
            }
        }
        ActionType::Exit => return Ok(false),
        ActionType::Reload => {}
        ActionType::Config => {
            if let Some(config_action) = action.config {
                return handle_config_action(app, config_action);
            }
        }
    }
    Ok(true)
}
