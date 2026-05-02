use std::io::{self};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use arboard::Clipboard;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::List as TuiList;
use ratatui::widgets::ListItem as TuiListItem;
use ratatui::widgets::{Block, Borders, ListDirection, Paragraph, Wrap};
use ratatui::Frame;

use crate::config::{Config, ExtensionConfig};
use crate::extensions;
use crate::history::History;
use crate::types::*;

#[allow(dead_code)]
#[derive(Clone)]
struct FilterItem {
    item: ListItem,
    filter_text: String,
}

/// Central application state for the TUI event loop.
struct AppState {
    items: Vec<FilterItem>,
    filtered_items: Vec<usize>,
    selection: usize,
    query: String,

    actions: Vec<Action>,
    action_selection: usize,
    action_mode: bool,

    page_stack: Vec<Page>,

    config_path: PathBuf,
    config: Config,
    history: History,

    form: Option<FormState>,

    detail: Option<PageDetail>,

    notification: String,
    notification_until: Option<Instant>,

    width: u16,
    height: u16,
    should_quit: bool,
}

#[derive(Clone)]
struct PageDetail {
    markdown: String,
    actions: Vec<Action>,
    inner_selection: usize,
    action_mode: bool,
}

#[allow(dead_code)]
#[derive(Clone)]
struct FormState {
    title: String,
    fields: Vec<FormField>,
    selection: usize,
    config: Config,
    ext_cfg: ExtensionConfig,
    alias: String,
}

#[derive(Clone)]
struct FormField {
    input: Input,
    value: String,
    checked: bool,
}

#[allow(dead_code)]
#[derive(Clone)]
enum Page {
    Root,
    Detail(PageDetail),
    Runner(RunnerPage),
}

#[allow(dead_code)]
#[derive(Clone)]
struct RunnerPage {
    extension_origin: String,
    command_name: String,
    items: Vec<FilterItem>,
    filtered_items: Vec<usize>,
    selection: usize,
    query: String,
    actions: Vec<Action>,
    action_selection: usize,
    action_mode: bool,
    is_detail: bool,
    detail_text: String,
    detail_actions: Vec<Action>,
    has_loaded: bool,
    auto_refresh: Option<i32>,
}

/// Computes a fuzzy match score between `text` and `pattern`.
///
/// Returns a positive score when all characters of `pattern` appear in `text`
/// in order (not necessarily contiguously). Consecutive matches score higher.
/// Case-sensitive search is used when `pattern` contains uppercase letters.
fn fuzzy_score(text: &str, pattern: &str) -> i32 {
    if pattern.is_empty() {
        return 1;
    }
    let lower_text = text.to_lowercase();
    let lower_pattern = pattern.to_lowercase();
    let case_sensitive = pattern.chars().any(|c| c.is_uppercase());

    let search_text = if case_sensitive { text } else { &lower_text };
    let search_pattern = if case_sensitive {
        pattern
    } else {
        &lower_pattern
    };

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
///
/// Returns indices into `items` sorted by descending score.
fn filter_items(items: &[FilterItem], query: &str) -> Vec<(usize, i32)> {
    if query.is_empty() {
        return items
            .iter()
            .enumerate()
            .map(|(i, _)| (i, i32::MAX))
            .collect();
    }
    let mut scored: Vec<(usize, i32)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let score = fuzzy_score(&item.filter_text, query);
            if score > 0 {
                Some((i, score))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by_key(|k| std::cmp::Reverse(k.1));
    scored
}

/// Launches the root list TUI.
///
/// The root list shows all oneliners and extension commands. The user can
/// search, select items, and trigger actions.
///
/// # Example
/// ```
/// tui::run_root_list("Sunbeam", &config_path, &mut history, cfg, items)?;
/// ```
///
/// # Errors
/// Returns an error if the terminal cannot be initialised or the event loop
/// encounters an I/O error.
pub fn run_root_list(
    title: &str,
    config_path: &Path,
    history: &mut History,
    config: Config,
    items: Vec<ListItem>,
    extra_actions: Vec<Action>,
) -> Result<()> {
    let items_vec: Vec<FilterItem> = items
        .into_iter()
        .map(|item| {
            let filter_text = format!(
                "{} {} {}",
                item.title,
                item.subtitle.as_deref().unwrap_or(""),
                item.accessories
                    .as_ref()
                    .map(|a| a.join(" "))
                    .unwrap_or_default()
            );
            FilterItem { item, filter_text }
        })
        .collect();

    let actions = if extra_actions.is_empty() {
        vec![Action {
            title: Some("Edit Config".to_string()),
            key: Some("s".to_string()),
            action_type: ActionType::Exec,
            open: None,
            copy: None,
            run: None,
            exec: Some(ExecAction {
                command: "sunbeam edit --config".into(),
                interactive: Some(true),
                dir: None,
                exit: None,
            }),
            edit: None,
            config: None,
            reload: None,
        }]
    } else {
        extra_actions
    };

    let filtered = filter_items(&items_vec, "");
    let selection = filtered.first().copied().map(|(i, _)| i).unwrap_or(0);

    let app = AppState {
        items: items_vec,
        filtered_items: filtered.iter().map(|(i, _)| *i).collect(),
        selection,
        query: String::new(),
        actions: actions.clone(),
        action_selection: 0,
        action_mode: false,
        page_stack: vec![Page::Root],
        config_path: config_path.to_path_buf(),
        config,
        history: history.clone(),
        form: None,
        detail: None,
        notification: String::new(),
        notification_until: None,
        width: 0,
        height: 0,
        should_quit: false,
    };

    run_app(app, title)
}

/// Launches a TUI form for configuring an extension's preferences.
///
/// # Errors
/// Returns an error if the terminal cannot be initialised.
pub fn run_form(
    alias: &str,
    cfg: &mut Config,
    ext_cfg: ExtensionConfig,
    inputs: Vec<Input>,
) -> Result<()> {
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

    let app = AppState {
        items: vec![],
        filtered_items: vec![],
        selection: 0,
        query: String::new(),
        actions: vec![],
        action_selection: 0,
        action_mode: false,
        page_stack: vec![Page::Root],
        config_path: cfg.path.clone(),
        config: cfg.clone(),
        history: History::load(&crate::history::history_path())?,
        form: Some(FormState {
            title: format!("Configure {}", alias),
            fields,
            selection: 0,
            config: cfg.clone(),
            ext_cfg,
            alias: alias.to_string(),
        }),
        detail: None,
        notification: String::new(),
        notification_until: None,
        width: 0,
        height: 0,
        should_quit: false,
    };

    run_app(app, "Configure Extension")
}

/// Initialises the terminal and runs the TUI event loop.
fn run_app(mut app: AppState, title: &str) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    let result = run_event_loop(&mut terminal, &mut app, title);

    terminal.show_cursor()?;
    terminal::disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(LeaveAlternateScreen)?;

    result
}

/// The main event loop: draws frames and processes keyboard input.
fn run_event_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut AppState,
    _title: &str,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(50);

    let (_cols, _rows) = crossterm::terminal::size().unwrap_or((80, 24));
    app.width = _cols;
    app.height = _rows;

    loop {
        terminal.draw(|f| render(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if !handle_key(app, key)? {
                    app.should_quit = true;
                }
            }
        }

        if app.should_quit {
            break;
        }

        if let Some(until) = app.notification_until {
            if Instant::now() >= until {
                app.notification.clear();
                app.notification_until = None;
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    app.history.save().ok();
    Ok(())
}

/// Processes a single keyboard event and mutates the application state.
///
/// Returns `false` when the application should exit.
///
/// ## Key design
///
/// The TUI has two focus modes — **list mode** (default) and **action mode**
/// (activated by Tab). In action mode the search bar becomes an action filter,
/// arrow keys navigate actions instead of items, and Enter triggers the
/// selected action instead of the default item action. Every handler checks
/// `app.action_mode` first so the two modes share the same keybindings.
fn handle_key(app: &mut AppState, key: KeyEvent) -> Result<bool> {
    match key.code {
        // ── Ctrl+C ──────────────────────────────────────────────────────
        // Standard terminal SIGINT. Always returns false → should_quit.
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(false);
        }

        // ── Escape ──────────────────────────────────────────────────────
        // Context-sensitive back-out: closes the innermost overlay first,
        // then the current page, then the whole app.
        //
        // Order: action mode → detail overlay → form overlay → runner page
        // → quit. This lets the user "drill up" one step at a time.
        KeyCode::Esc => {
            // Action mode active → deactivate it (back to list, keep current page)
            if app.action_mode {
                app.action_mode = false;
                app.action_selection = 0;
                return Ok(true);
            }
            // Detail overlay visible → close it
            if app.detail.is_some() {
                app.detail = None;
                return Ok(true);
            }
            // Form overlay visible → close it
            if app.form.is_some() {
                app.form = None;
                return Ok(true);
            }
            // Inside a runner page → pop back to parent
            if app.page_stack.len() > 1 {
                app.page_stack.pop();
                return Ok(true);
            }
            // Root page with no overlay → quit
            return Ok(false);
        }

        // ── Enter ───────────────────────────────────────────────────────
        // Triggers the primary action for the current context. Priority:
        // action mode (selected action) → form (submit) → detail (first action)
        // → list item (first action, and record usage history).
        KeyCode::Enter => {
            // Action mode: execute the highlighted action
            if app.action_mode {
                let action = get_selected_action(app);
                if let Some(action) = action {
                    return dispatch_action(app, action);
                }
                return Ok(true);
            }
            // Form mode: collect field values and submit
            if app.form.is_some() {
                let form = app.form.take().unwrap();
                let result = submit_form(app, form);
                return result;
            }
            // Detail mode: trigger the first (or selected) action
            if let Some(ref detail) = app.detail.clone() {
                if detail.actions.is_empty() {
                    return Ok(true);
                }
                let action = if detail.action_mode && detail.inner_selection < detail.actions.len()
                {
                    detail.actions[detail.inner_selection].clone()
                } else if !detail.actions.is_empty() {
                    detail.actions[0].clone()
                } else {
                    return Ok(true);
                };
                app.detail.as_mut().unwrap().action_mode = false;
                return dispatch_action(app, action);
            }
            // List mode: execute the selected item's first action;
            // record the selection in MRU history so frequently-used
            // items float to the top.
            let idx = *app.filtered_items.get(app.selection).unwrap_or(&0);
            if let Some(item) = app.items.get(idx) {
                let item_actions = item.item.actions.as_ref().cloned().unwrap_or_default();
                if item_actions.is_empty() {
                    return Ok(true);
                }
                let action = item_actions[0].clone();

                let key = item
                    .item
                    .id
                    .as_deref()
                    .unwrap_or(&item.item.title)
                    .to_string();
                app.history.update(&key);

                return dispatch_action(app, action);
            }
        }

        // ── j / k (Vim-style navigation) ────────────────────────────────
        // Down / up. Works in both list mode and action mode.
        // j/k are matched before the generic KeyCode::Char(c) catch-all so
        // they are NOT treated as search characters.
        KeyCode::Char('k') => {
            if app.action_mode {
                if app.action_selection > 0 {
                    app.action_selection -= 1;
                }
                return Ok(true);
            }
            if app.selection > 0 {
                app.selection -= 1;
            }
        }
        KeyCode::Char('j') => {
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
        }

        // ── Backspace / Ctrl+Backspace ───────────────────────────────────
        // Backspace deletes the last character. Ctrl+Backspace (which the
        // terminal sends as ASCII 0x08, parsed by crossterm as Ctrl+H)
        // deletes the last word (trims trailing whitespace, then truncates
        // at the previous whitespace boundary). After deletion, the item
        // list is re-filtered and the cursor resets to the top.
        KeyCode::Backspace => {
            if app.action_mode {
                app.query.pop();
                return Ok(true);
            }
            app.query.pop();
            app.filtered_items = filter_items(&app.items, &app.query)
                .iter()
                .map(|(i, _)| *i)
                .collect();
            if !app.filtered_items.is_empty() {
                app.selection = 0;
            }
        }

        // ── Tab ─────────────────────────────────────────────────────────
        // Toggles action mode. When there are 2+ actions available, Tab
        // switches from "search items" to "search and pick an action".
        // A second Tab or Esc returns to item-selection mode.
        // For detail pages, Tab opens the action filter directly.
        KeyCode::Tab => {
            if let Some(ref detail) = app.detail {
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

        // ── Arrow keys ──────────────────────────────────────────────────
        // Duplicate of j/k for users who prefer arrow keys.
        KeyCode::Up => {
            if app.action_mode {
                if app.action_selection > 0 {
                    app.action_selection -= 1;
                }
                return Ok(true);
            }
            if app.selection > 0 {
                app.selection -= 1;
            }
        }
        KeyCode::Down => {
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
        }

        // ── Ctrl+R ──────────────────────────────────────────────────────
        // Reload: re-reads the config file from disk and rebuilds the
        // entire item list. Equivalent to restarting the TUI without
        // quitting. Useful after editing config or installing an extension.
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
                                    open: None,
                                    copy: None,
                                    run: None,
                                    exec: Some(ExecAction {
                                        command: o.command.clone(),
                                        interactive: o.interactive,
                                        dir: o.cwd.clone(),
                                        exit: o.exit,
                                    }),
                                    edit: None,
                                    config: None,
                                    reload: None,
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
                                            open: None,
                                            copy: None,
                                            run: Some(RunAction {
                                                extension: Some(alias.clone()),
                                                command: cmd.name.clone(),
                                                params: None,
                                                reload: None,
                                                exit: None,
                                            }),
                                            exec: None,
                                            edit: None,
                                            config: None,
                                            reload: None,
                                        }]),
                                    },
                                });
                            }
                        }
                    }
                }
                app.items = new_items;
                app.filtered_items = filter_items(&app.items, &app.query)
                    .iter()
                    .map(|(i, _)| *i)
                    .collect();
                if !app.filtered_items.is_empty() {
                    app.selection = 0;
                }
            }
        }

        // ── Ctrl+H (Ctrl+Backspace) ─────────────────────────────────────
        // In ASCII, Ctrl+H = 0x08, which is the same byte most terminals
        // send for Ctrl+Backspace. crossterm parses it as
        // `KeyCode::Char('h') + CONTROL`. We catch it here before the
        // generic Char(c) handler (which would append 'h' to the query).
        // The behaviour is "delete the last word" — find the last
        // whitespace boundary in the query and truncate there.
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
            }
        }

        // ── Ctrl+S ──────────────────────────────────────────────────────
        // Opens the config file (~/.config/sunbeam/sunbeam.json) in the
        // user's $EDITOR. After the editor exits, reloads the config so
        // changes (new extensions, modified oneliners) take effect without
        // restarting the TUI. Raw mode is temporarily disabled so the
        // editor can use the terminal normally.
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

        // ── Character input (catch-all) ─────────────────────────────────
        // Appends the typed character to the query string, re-filters the
        // item list with the updated query (fuzzy match), and resets the
        // cursor to the first match. In action mode, same but for action
        // filtering — no re-filter needed since actions are matched
        // separately in the statusbar module.
        KeyCode::Char(c) => {
            if app.action_mode {
                app.query.push(c);
                return Ok(true);
            }
            app.query.push(c);
            app.filtered_items = filter_items(&app.items, &app.query)
                .iter()
                .map(|(i, _)| *i)
                .collect();
            if !app.filtered_items.is_empty() {
                app.selection = 0;
            }
        }

        // ── All other keys ──────────────────────────────────────────────
        // Ignored silently (Shift, Alt, function keys, etc.).
        _ => {}
    }
    Ok(true)
}

/// Returns the actions for the currently selected item or page.
fn get_current_list_actions(app: &AppState) -> Vec<Action> {
    if let Some(Page::Runner(runner)) = app.page_stack.last() {
        let idx = runner
            .filtered_items
            .get(runner.selection)
            .copied()
            .unwrap_or(0);
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
fn get_selected_action(app: &AppState) -> Option<Action> {
    let actions = get_current_list_actions(app);
    actions.get(app.action_selection).cloned()
}

/// Dispatches an action: executes the behaviour associated with the action type.
///
/// Handles `Run`, `Copy`, `Open`, `Edit`, `Exec`, `Exit`, `Reload`, and
/// `Config` action types. Returns `false` when the application should exit.
fn dispatch_action(app: &mut AppState, action: Action) -> Result<bool> {
    match action.action_type {
        ActionType::Run => {
            if let Some(run) = action.run {
                let extension_origin = run.extension.clone().unwrap_or_default();
                if let Some(exts) = &app.config.extensions {
                    if let Some(ext_cfg) = exts.get(&extension_origin) {
                        let origin = ext_cfg.origin.clone();
                        if let Ok(extension) = extensions::load_extension(&origin) {
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
                                    return run_extension_list(app, extension, payload);
                                }
                                CommandMode::Detail => {
                                    return run_extension_detail(app, extension, payload);
                                }
                                CommandMode::Silent => {
                                    terminal::disable_raw_mode()?;
                                    let result = extension.run_quiet(&payload);
                                    terminal::enable_raw_mode()?;
                                    if let Err(e) = result {
                                        app.notification = format!("Error: {}", e);
                                        app.notification_until =
                                            Some(Instant::now() + Duration::from_secs(2));
                                    }
                                }
                                CommandMode::Tty => {
                                    let mut cmd = extension.cmd(&payload)?;
                                    terminal::disable_raw_mode()?;
                                    let mut child = cmd.spawn()?;
                                    child.wait()?;
                                    terminal::enable_raw_mode()?;
                                }
                            }
                        }
                    }
                }
            }
        }
        ActionType::Copy => {
            if let Some(copy) = action.copy {
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
                            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            app.notification = text;
                            app.notification_until = Some(Instant::now() + Duration::from_secs(2));
                        }
                    }
                }
                if exec.exit.unwrap_or(false) {
                    return Ok(false);
                }
            }
        }
        ActionType::Exit => {
            return Ok(false);
        }
        ActionType::Reload => {}
        ActionType::Config => {
            if let Some(config_action) = action.config {
                let alias = config_action.extension;
                if let Some(ext_cfg) = app.config.extensions.as_ref().and_then(|e| e.get(&alias)) {
                    if let Ok(extension) = extensions::load_extension(&ext_cfg.origin) {
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
                            config: app.config.clone(),
                            ext_cfg: ext_cfg.clone(),
                            alias: alias.clone(),
                        });
                    }
                }
            }
        }
    }
    Ok(true)
}

/// Runs an extension command in `search` or `filter` mode, embedding the
/// returned list as a new page in the stack.
fn run_extension_list(
    app: &mut AppState,
    extension: extensions::Extension,
    payload: Payload,
) -> Result<bool> {
    let result = run_extension_and_parse(&extension, &payload);
    match result {
        Ok(list) => {
            let items: Vec<FilterItem> = list
                .items
                .unwrap_or_default()
                .into_iter()
                .map(|item| {
                    let filter_text = format!(
                        "{} {} {}",
                        item.title,
                        item.subtitle.as_deref().unwrap_or(""),
                        item.accessories
                            .as_ref()
                            .map(|a| a.join(" "))
                            .unwrap_or_default()
                    );
                    FilterItem { item, filter_text }
                })
                .collect();

            let filtered = filter_items(&items, "");
            let page = RunnerPage {
                extension_origin: extension.entrypoint.to_string_lossy().to_string(),
                command_name: payload.command.clone(),
                items,
                filtered_items: filtered.iter().map(|(i, _)| *i).collect(),
                selection: 0,
                query: String::new(),
                actions: list.actions.unwrap_or_default(),
                action_selection: 0,
                action_mode: false,
                is_detail: false,
                detail_text: String::new(),
                detail_actions: vec![],
                has_loaded: true,
                auto_refresh: list.auto_refresh_seconds,
            };

            app.page_stack.push(Page::Runner(page));
        }
        Err(e) => {
            app.notification = format!("Error: {}", e);
            app.notification_until = Some(Instant::now() + Duration::from_secs(2));
        }
    }
    Ok(true)
}

/// Runs an extension command in `detail` mode and shows the result.
fn run_extension_detail(
    app: &mut AppState,
    extension: extensions::Extension,
    payload: Payload,
) -> Result<bool> {
    let result = run_extension_and_parse_detail(&extension, &payload);
    match result {
        Ok(detail) => {
            app.detail = Some(PageDetail {
                markdown: detail.markdown.unwrap_or_default(),
                actions: detail.actions.unwrap_or_default(),
                inner_selection: 0,
                action_mode: false,
            });
        }
        Err(e) => {
            app.notification = format!("Error: {}", e);
            app.notification_until = Some(Instant::now() + Duration::from_secs(2));
        }
    }
    Ok(true)
}

/// Runs an extension command and parses the output as a `List`.
fn run_extension_and_parse(extension: &extensions::Extension, payload: &Payload) -> Result<List> {
    let output = extension.run(payload)?;
    crate::schemas::validate_list(&output).context("invalid list output")?;
    let list: List = serde_json::from_slice(&output)?;
    Ok(list)
}

/// Runs an extension command and parses the output as a `Detail`.
fn run_extension_and_parse_detail(
    extension: &extensions::Extension,
    payload: &Payload,
) -> Result<Detail> {
    let output = extension.run(payload)?;
    crate::schemas::validate_detail(&output).context("invalid detail output")?;
    let detail: Detail = serde_json::from_slice(&output)?;
    Ok(detail)
}

/// Collects form field values and persists the updated preferences to config.
fn submit_form(app: &mut AppState, form: FormState) -> Result<bool> {
    let mut values = serde_json::Map::new();
    for field in &form.fields {
        match field.input.input_type {
            InputType::String | InputType::Number => {
                values.insert(
                    field.input.name.clone(),
                    serde_json::Value::String(field.value.clone()),
                );
            }
            InputType::Boolean => {
                values.insert(
                    field.input.name.clone(),
                    serde_json::Value::Bool(field.checked),
                );
            }
        }
    }

    let mut ext_cfg = form.ext_cfg.clone();
    ext_cfg.preferences = Some(values);

    if let Some(exts) = &mut app.config.extensions {
        exts.insert(form.alias.clone(), ext_cfg);
    }
    app.config.save()?;
    app.form = None;
    Ok(true)
}

// ─── Rendering functions ──────────────────────────────────────────────────

/// Top-level renderer that dispatches to the appropriate view.
fn render(f: &mut Frame, app: &AppState) {
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
    // Split screen vertically: search bar (3 rows) | list (remaining) | status bar (3 rows)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    // ── Search bar ──────────────────────────────────────────────────
    // When action_mode is active the prompt turns yellow and the
    // placeholder changes from "Search Items..." to "Search Actions...",
    // meaning keystrokes will filter the action list instead of items.
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
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(" Sunbeam ")),
    );
    f.render_widget(search, chunks[0]);

    let tui_items: Vec<TuiListItem> = app
        .filtered_items
        .iter()
        .enumerate()
        .map(|(i, idx)| {
            let item = &app.items[*idx];
            let is_selected = i == app.selection;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if is_selected { ">" } else { " " };
            let title = format!("{} {}", prefix, item.item.title);
            let subtitle = item
                .item
                .subtitle
                .as_deref()
                .map(|s| format!(" {}", s))
                .unwrap_or_default();
            let accessories = item
                .item
                .accessories
                .as_ref()
                .map(|a| format!("  {}", a.join(" · ")))
                .unwrap_or_default();
            TuiListItem::new(format!("{}{}{}", title, subtitle, accessories)).style(style)
        })
        .collect();

    // ── Filtered item list ──────────────────────────────────────────
    // Rendered via ratatui's List widget. Each row shows:
    // selection indicator (> / space) + title + subtitle + accessories.
    // The selected row is highlighted with magenta + bold.
    let list = TuiList::new(tui_items).direction(ListDirection::TopToBottom);

    let list_block = Block::default().borders(Borders::ALL);
    let list_widget = list.block(list_block);
    f.render_widget(list_widget, chunks[1]);

    let actions = get_current_list_actions(app);
    let action_text = if actions.is_empty() {
        " No actions".to_string()
    } else if app.action_mode {
        actions
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let title = a.title.as_deref().unwrap_or("");
                let key_hint = if i == 0 {
                    "enter"
                } else if i == 1 {
                    "alt+enter"
                } else {
                    a.key.as_deref().unwrap_or("")
                };
                let selected = i == app.action_selection;
                let hint = if !key_hint.is_empty() {
                    format!(" {} ", key_hint)
                } else {
                    String::new()
                };
                if selected {
                    format!("[{} {}]", title, hint)
                } else {
                    format!(" {} {} ", title, hint)
                }
            })
            .collect::<Vec<_>>()
            .join("·")
    } else {
        let first = actions
            .first()
            .map(|a| a.title.as_deref().unwrap_or(""))
            .unwrap_or("");
        format!(" {} · Actions(tab)", first)
    };

    let status_text = if !app.notification.is_empty() {
        format!(" {}   {}", app.notification, action_text)
    } else {
        action_text
    };

    // ── Status / action bar ─────────────────────────────────────────
    // Shows the primary action name + "Actions(tab)" hint by default.
    // In action_mode it expands to a full action list with brackets
    // around the selected action. Notifications (e.g. "Copied!") appear
    // on the left and auto-dismiss after 1 second.
    let status = Paragraph::new(status_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}

/// Renders a detail page with optional action bar.
fn render_detail_page(f: &mut Frame, area: Rect, detail: &PageDetail) {
    // Split vertically: content area | bottom action bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    // ── Content area ────────────────────────────────────────────────
    // The extension's Detail JSON can contain markdown or plain text.
    // The original Go version renders markdown via glamour and word-wraps
    // plain text. This simplified implementation uses raw Paragraph + Wrap.
    // Content over 5000 characters is truncated.
    let detail_text: &str = if detail.markdown.is_empty() {
        "No content"
    } else {
        &detail.markdown
    };

    let detail_widget = Paragraph::new(Text::raw(detail_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(" Detail ")),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(detail_widget, chunks[0]);

    // ── Bottom action bar ───────────────────────────────────────────
    // Same layout as render_root_list's status bar, but adds a "q: back"
    // shortcut. When action_mode is active, actions expand into a
    // selectable list; pressing Enter triggers the selected action.
    let action_text = if detail.actions.is_empty() {
        " q: back".to_string()
    } else if detail.action_mode {
        detail
            .actions
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let title = a.title.as_deref().unwrap_or("");
                let selected = i == detail.inner_selection;
                if selected {
                    format!("[{}]", title)
                } else {
                    format!(" {}", title)
                }
            })
            .collect::<Vec<_>>()
            .join("·")
    } else {
        let first = detail
            .actions
            .first()
            .map(|a| a.title.as_deref().unwrap_or(""))
            .unwrap_or("");
        format!(" {} · Actions(tab)", first)
    };

    let status = Paragraph::new(format!(" q: back | {}", action_text))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[1]);
}

/// Renders a preference configuration form.
fn render_prefs_form(f: &mut Frame, area: Rect, form: &FormState) {
    // Split vertically: form fields | bottom hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(area);

    // ── Form fields ─────────────────────────────────────────────────
    // Three InputTypes are supported:
    //   string  → single-line text
    //   number  → single-line numeric
    //   boolean → [x] / [ ] checkbox
    //
    // Each field renders as two lines: a bold label row and a value row.
    // The focused field (is_selected) uses magenta+bold for the label and
    // yellow for the value; unfocused fields use gray.
    // Tab / Shift+Tab cycles focus, Alt+Enter submits.
    let mut lines = Vec::new();
    for (i, field) in form.fields.iter().enumerate() {
        let is_selected = i == form.selection;
        let title_style = if is_selected {
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![Span::styled(
            format!("{}: ", field.input.title),
            title_style,
        )]));
        let value = match field.input.input_type {
            InputType::Boolean => if field.checked { "[x]" } else { "[ ]" }.to_string(),
            _ => {
                if field.value.is_empty() {
                    field.input.title.clone()
                } else {
                    field.value.clone()
                }
            }
        };
        let value_style = if is_selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.push(Line::from(vec![Span::styled(
            format!("  {}", value),
            value_style,
        )]));
        lines.push(Line::from(""));
    }

    let content = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(form.title.clone())),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(content, chunks[0]);

    // ── Bottom hint ─────────────────────────────────────────────────
    // Note: Alt+Enter is used instead of plain Enter because Enter has
    // special meaning for some field types (e.g. toggling a checkbox).
    let status = Paragraph::new(" Tab: next  Alt+Enter: submit  Esc: cancel ")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[1]);
}

/// Renders an extension runner page (search or filter mode).
fn render_extension_list(f: &mut Frame, area: Rect, runner: &RunnerPage) {
    // Same three-row layout as render_root_list: search bar | list | status bar.
    // The key difference is the data source — the runner's data comes from the
    // extension process's List JSON output, not from the config file.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    // ── Search bar ──────────────────────────────────────────────────
    // The title shows "Extension" instead of "Sunbeam" so the user knows
    // they are not on the root page. In search mode every keystroke
    // triggers a re-invocation of the extension (on_query_change).
    // In filter mode only client-side fuzzy matching is performed.
    let query_display = if runner.query.is_empty() {
        "Search Items..."
    } else {
        &runner.query
    };
    let search = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(query_display),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(" Extension ")),
    );
    f.render_widget(search, chunks[0]);

    let tui_items: Vec<TuiListItem> = runner
        .filtered_items
        .iter()
        .enumerate()
        .map(|(i, idx)| {
            let item = &runner.items[*idx];
            let is_selected = i == runner.selection;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if is_selected { ">" } else { " " };
            let title = format!("{} {}", prefix, item.item.title);
            let subtitle = item
                .item
                .subtitle
                .as_deref()
                .map(|s| format!(" {}", s))
                .unwrap_or_default();
            let accessories = item
                .item
                .accessories
                .as_ref()
                .map(|a| format!("  {}", a.join(" · ")))
                .unwrap_or_default();
            TuiListItem::new(format!("{}{}{}", title, subtitle, accessories)).style(style)
        })
        .collect();

    // ── Item list ───────────────────────────────────────────────────
    // Identical layout to render_root_list's list section. No extra
    // outer border is needed because the search bar block above already
    // provides visual continuity.
    let list = TuiList::new(tui_items)
        .direction(ListDirection::TopToBottom)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(list, chunks[1]);

    // ── Status bar ──────────────────────────────────────────────────
    // Same action bar as the root list, but with a fixed "Esc: back"
    // reminder so the user knows they can return to the previous page
    // (the root list or a parent extension list).
    let actions = &runner.actions;
    let action_text = if actions.is_empty() {
        " No actions".to_string()
    } else {
        actions
            .iter()
            .map(|a| a.title.as_deref().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let status = Paragraph::new(format!(" {} | Esc: back", action_text))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, chunks[2]);
}
