use anyhow::{Context, Result};

use crate::extensions;
use crate::schemas;
use crate::types::{self, *};
use crate::tui::render_md;
use crate::tui::types::*;

/// Runs an extension command in `search` or `filter` mode, embedding the
/// returned list as a new page in the stack.
pub fn run_extension_list(
    app: &mut AppState,
    extension: extensions::Extension,
    payload: Payload,
) -> Result<bool> {
    let mode = extension.command(&payload.command)
        .map(|c| c.mode.clone().unwrap_or(CommandMode::Filter))
        .unwrap_or(CommandMode::Filter);
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
                        item.accessories.as_ref().map(|a| a.join(" ")).unwrap_or_default()
                    );
                    FilterItem { item, filter_text }
                })
                .collect();

            let filtered = filter_items(&items, "");
            let page = RunnerPage {
                extension: Some(extension.clone()),
                command_name: payload.command.clone(),
                preferences: payload.preferences.clone(),
                mode,
                items,
                filtered_items: filtered.iter().map(|(i, _)| *i).collect(),
                selection: 0,
                query: String::new(),
                actions: list.actions.unwrap_or_default(),
                is_loading: false,
                page: 0,
                page_size: 15,
                show_detail: list.show_detail.unwrap_or(false),
                auto_refresh_seconds: list.auto_refresh_seconds,
                pending_query: String::new(),
            };

            app.page_stack.push(Page::Runner(page));
        }
        Err(e) => {
            app.notification = format!("Error: {}", e);
            app.notification_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
        }
    }
    Ok(true)
}

/// Runs an extension command in `detail` mode and shows the result.
pub fn run_extension_detail(
    app: &mut AppState,
    extension: extensions::Extension,
    payload: Payload,
) -> Result<bool> {
    match run_extension_and_parse_detail(&extension, &payload) {
        Ok(detail) => {
            let text = detail.markdown.unwrap_or_else(|| detail.text.unwrap_or_default());
            let (rendered_lines, _) = render_md::render_markdown(&text, 80);
            app.detail = Some(PageDetail {
                markdown: text,
                actions: detail.actions.unwrap_or_default(),
                inner_selection: 0,
                action_mode: false,
                scroll_offset: 0,
                rendered_lines,
                action_query: String::new(),
            });
        }
        Err(e) => {
            app.notification = format!("Error: {}", e);
            app.notification_until = Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
        }
    }
    Ok(true)
}

fn run_extension_and_parse(extension: &extensions::Extension, payload: &Payload) -> Result<List> {
    let output = extension.run(payload)?;
    schemas::validate_list(&output).context("invalid list output")?;
    Ok(serde_json::from_slice(&output)?)
}

fn run_extension_and_parse_detail(extension: &extensions::Extension, payload: &Payload) -> Result<Detail> {
    let output = extension.run(payload)?;
    schemas::validate_detail(&output).context("invalid detail output")?;
    Ok(serde_json::from_slice(&output)?)
}

/// Re-runs the extension with the runner's current query and replaces its items.
/// Used in "search" mode where every keystroke triggers a new invocation.
pub fn reload_runner(runner: &mut RunnerPage) -> Result<()> {
    runner.is_loading = true;
    let payload = Payload {
        command: runner.command_name.clone(),
        preferences: runner.preferences.clone(),
        params: None,
        cwd: None,
        r#query: Some(runner.query.clone()),
    };

    let extension = runner.extension.as_ref()
        .context("cannot reload: extension not available")?;

    let output = extension.run(&payload)?;
    schemas::validate_list(&output).context("invalid list output")?;
    let list: types::List = serde_json::from_slice(&output)?;

    runner.items = list.items.unwrap_or_default().into_iter().map(|item| {
        let filter_text = format!(
            "{} {} {}",
            item.title,
            item.subtitle.as_deref().unwrap_or(""),
            item.accessories.as_ref().map(|a| a.join(" ")).unwrap_or_default()
        );
        FilterItem { item, filter_text }
    }).collect();
    runner.filtered_items = filter_items(&runner.items, &runner.query).iter().map(|(i, _)| *i).collect();
    clamp_page_to_selection(runner.filtered_items.len(), &mut runner.selection, &mut runner.page, runner.page_size);
    runner.actions = list.actions.unwrap_or_default();
    runner.show_detail = list.show_detail.unwrap_or(false);
    runner.auto_refresh_seconds = list.auto_refresh_seconds;
    runner.is_loading = false;
    Ok(())
}

/// Collects form field values and persists the updated preferences to config.
pub fn submit_form(app: &mut AppState, form: FormState) -> Result<bool> {
    let mut values = serde_json::Map::new();
    for field in &form.fields {
        match field.input.input_type {
            InputType::String | InputType::Number => {
                values.insert(field.input.name.clone(), serde_json::Value::String(field.value.clone()));
            }
            InputType::Boolean => {
                values.insert(field.input.name.clone(), serde_json::Value::Bool(field.checked));
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
