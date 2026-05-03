mod types;
mod key;
mod runner;
mod render;
mod render_md;

use std::io::{self};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;

use crate::config::{Config, ExtensionConfig};
use crate::history::History;
use crate::types::*;
use crate::tui::types::*;
use crate::tui::render::render;
use crate::tui::key::handle_key;



/// Launches the root list TUI.
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
                item.accessories.as_ref().map(|a| a.join(" ")).unwrap_or_default()
            );
            FilterItem { item, filter_text }
        })
        .collect();

    let actions = if extra_actions.is_empty() {
        vec![Action {
            title: Some("Edit Config".to_string()),
            key: Some("s".to_string()),
            action_type: ActionType::Exec,
            open: None, copy: None, run: None,
            exec: Some(ExecAction {
                command: "sunbeam edit --config".into(),
                interactive: Some(true),
                dir: None, exit: None,
            }),
            edit: None, config: None, reload: None,
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
        err: None,
        page: 0,
        page_size: 15,
        tick: 0,
        last_auto_refresh: Instant::now(),
    };

    run_app(app, title)
}

/// Launches a TUI form for configuring an extension's preferences.
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
                Some(v) if input.input_type == InputType::Boolean => (String::new(), v.as_bool().unwrap_or(false)),
                Some(v) => (v.as_str().unwrap_or("").to_string(), false),
                None => (String::new(), false),
            };
            FormField { input, value, checked }
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
            ext_cfg,
            alias: alias.to_string(),
        }),
        detail: None,
        notification: String::new(),
        notification_until: None,
        width: 0,
        height: 0,
        should_quit: false,
        err: None,
        page: 0,
        page_size: 15,
        tick: 0,
        last_auto_refresh: Instant::now(),
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

        let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or_default();

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if !handle_key(app, key)? {
                    app.should_quit = true;
                }
            }
        }

        if app.should_quit { break; }

        if let Some(until) = app.notification_until {
            if Instant::now() >= until {
                app.notification.clear();
                app.notification_until = None;
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            app.tick = app.tick.wrapping_add(1);
        }

        // Auto-refresh: check if the current runner page has auto_refresh_seconds set
        if let Some(Page::Runner(ref mut runner)) = app.page_stack.last_mut() {
            if let Some(secs) = runner.auto_refresh_seconds {
                let duration = Duration::from_secs(secs as u64);
                if app.last_auto_refresh.elapsed() >= duration {
                    app.last_auto_refresh = Instant::now();
                    let _ = crate::tui::runner::reload_runner(runner);
                }
            }
        }
    }

    app.history.save().ok();
    Ok(())
}
