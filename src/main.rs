mod config;
mod extensions;
mod history;
mod schemas;
mod tui;
mod types;
mod utils;

use std::io::Read;

use crate::config::{Config, ExtensionConfig, Oneliner};
use crate::types::{
    Action, ActionType, CommandMode, CopyAction, EditAction, ExecAction, Input, InputType,
    ListItem, Payload, RunAction,
};
use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::{Arg, Args, Command as ClapCommand, CommandFactory, Parser, Subcommand};

fn main() -> Result<()> {
    let raw: Vec<String> = std::env::args().collect();

    let cfg = load_config_or_default()?;

    if raw.len() <= 1 {
        return run_root(&cfg);
    }

    let first = &raw[1];

    let is_core = matches!(
        first.as_str(),
        "validate" | "edit" | "copy" | "paste" | "open" | "extension"
    );
    if is_core {
        let cli = Cli::parse();
        return dispatch_core(cli, &cfg);
    }

    if cfg
        .extensions
        .as_ref()
        .is_some_and(|e| e.contains_key(first))
    {
        return run_extension_invocation(&cfg, first, &raw[2..]);
    }

    anyhow::bail!(
        "unknown command: {first}\n\n{}{}",
        Cli::command().render_help(),
        extension_help(&cfg)
    )
}

fn extension_help(cfg: &Config) -> String {
    let exts = match &cfg.extensions {
        Some(e) if !e.is_empty() => e,
        _ => return String::new(),
    };

    let max_len = exts.keys().map(|k| k.len()).max().unwrap_or(0);
    let mut s = String::new();
    for (alias, ext_cfg) in exts {
        let title = extensions::load_extension(&ext_cfg.origin)
            .map(|e| e.manifest.title)
            .unwrap_or_default();
        s.push_str(&format!("  {:>width$}    {title}\n", alias, width = max_len));
    }
    format!("\n\nExtensions:\n{s}")
}

// ─── Core CLI (derive) ─────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "sunbeam",
    version,
    about = "Command Line Launcher",
    hide = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a Sunbeam schema
    #[command(subcommand)]
    Validate(ValidateCommands),
    /// Open a file in your editor
    Edit(EditArgs),
    /// Copy text from stdin to clipboard
    Copy,
    /// Paste text from clipboard to stdout
    Paste,
    /// Open a file or URL
    Open { target: String },
    /// Manage sunbeam extensions
    #[command(subcommand)]
    Extension(ExtensionCommands),
}

#[derive(Subcommand)]
enum ValidateCommands {
    List,
    Detail,
    Manifest,
    Config { path: Option<String> },
}

#[derive(Args)]
struct EditArgs {
    file: Option<String>,
    #[arg(short = 'e', long = "extension")]
    extension: Option<String>,
    #[arg(short = 'c', long = "config", default_value_t = false)]
    config: bool,
}

#[derive(Subcommand)]
enum ExtensionCommands {
    #[command(alias = "add")]
    Install {
        origin: String,
        #[arg(long, short)]
        alias: Option<String>,
    },
    #[command(alias = "ls")]
    List,
    #[command(alias = "rm", alias = "uninstall")]
    Remove {
        aliases: Vec<String>,
    },
    #[command(alias = "mv")]
    Rename {
        old: String,
        new: String,
    },
    Upgrade {
        alias: Option<String>,
        #[arg(long)]
        all: bool,
    },
    #[command(alias = "config")]
    Configure {
        alias: String,
    },
    Edit {
        alias: String,
    },
}

fn dispatch_core(cli: Cli, _cfg: &Config) -> Result<()> {
    match cli.command {
        Some(Commands::Validate(cmd)) => run_validate(cmd),
        Some(Commands::Edit(args)) => run_edit(args),
        Some(Commands::Copy) => run_copy(),
        Some(Commands::Paste) => run_paste(),
        Some(Commands::Open { target }) => utils::open_target(&target),
        Some(Commands::Extension(cmd)) => run_extension(cmd),
        None => unreachable!(),
    }
}

// ─── Extension invocation ──────────────────────────────────────────────────

fn run_extension_invocation(cfg: &Config, alias: &str, args: &[String]) -> Result<()> {
    let ext_cfg = cfg.extensions.as_ref().and_then(|e| e.get(alias)).unwrap();
    let extension =
        extensions::load_extension(&ext_cfg.origin).context("failed to load extension")?;

    let preferences = resolve_preferences(alias, &extension, ext_cfg);

    if !atty::is(atty::Stream::Stdin) {
        let mut stdin = Vec::new();
        std::io::stdin().lock().read_to_end(&mut stdin)?;
        if !stdin.is_empty() {
            let payload: Payload = serde_json::from_slice(&stdin)?;
            return run_extension_cmd(&extension, &payload);
        }
    }

    if args.is_empty() {
        if !atty::is(atty::Stream::Stdout) {
            serde_json::to_writer_pretty(std::io::stdout(), &extension.manifest)?;
            return Ok(());
        }
        // Do not show TUI when inside sunbeam
        return run_extension_tui(alias, cfg, &extension, ext_cfg);
    }

    let cmd_name = &args[0];
    let cmd_spec = extension
        .command(cmd_name)
        .ok_or_else(|| anyhow::anyhow!("unknown command `{cmd_name}` for extension `{alias}`"))?;

    let cmd_args = &args[1..];

    let params = if cmd_args.is_empty() && atty::is(atty::Stream::Stdin) {
        serde_json::Map::new()
    } else {
        parse_params(cmd_spec, cmd_args)?
    };

    let payload = Payload {
        command: cmd_name.clone(),
        preferences: Some(preferences),
        params: Some(params),
        cwd: Some(std::env::current_dir()?.to_string_lossy().to_string()),
        r#query: None,
    };

    if !atty::is(atty::Stream::Stdout) {
        let mut cmd = extension.cmd(&payload)?;
        cmd.stdin(std::process::Stdio::inherit());
        cmd.stdout(std::process::Stdio::inherit());
        cmd.stderr(std::process::Stdio::inherit());
        let status = cmd.status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        return Ok(());
    }

    match cmd_spec.mode.clone().unwrap_or(CommandMode::Filter) {
        CommandMode::Search | CommandMode::Filter => {
            let output = extension.run(&payload)?;
            schemas::validate_list(&output).context("invalid list output")?;
            let list: types::List = serde_json::from_slice(&output)?;
            if !atty::is(atty::Stream::Stdout) {
                serde_json::to_writer_pretty(std::io::stdout(), &list)?;
                return Ok(());
            }
            println!("{}", serde_json::to_string_pretty(&list)?);
        }
        CommandMode::Detail => {
            let output = extension.run(&payload)?;
            schemas::validate_detail(&output).context("invalid detail output")?;
            let detail: types::Detail = serde_json::from_slice(&output)?;
            if let Some(ref md) = detail.markdown {
                println!("{md}");
            } else if let Some(ref t) = detail.text {
                println!("{t}");
            }
        }
        CommandMode::Silent => {
            extension.run_quiet(&payload)?;
        }
        CommandMode::Tty => {
            let mut cmd = extension.cmd(&payload)?;
            cmd.stdin(std::process::Stdio::inherit());
            cmd.stdout(std::process::Stdio::inherit());
            cmd.stderr(std::process::Stdio::inherit());
            let status = cmd.status()?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
    }

    Ok(())
}

fn run_extension_cmd(extension: &extensions::Extension, payload: &Payload) -> Result<()> {
    if !atty::is(atty::Stream::Stdout) {
        let mut cmd = extension.cmd(payload)?;
        cmd.stdin(std::process::Stdio::inherit());
        cmd.stdout(std::process::Stdio::inherit());
        cmd.stderr(std::process::Stdio::inherit());
        let status = cmd.status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        return Ok(());
    }

    let output = extension.run(payload)?;
    let cmd_spec = extension.command(&payload.command);

    if let Some(spec) = cmd_spec {
        match spec.mode.clone().unwrap_or(CommandMode::Filter) {
            CommandMode::Search | CommandMode::Filter => {
                if let Ok(list) = serde_json::from_slice::<types::List>(&output) {
                    println!("{}", serde_json::to_string_pretty(&list)?);
                }
            }
            CommandMode::Detail => {
                if let Ok(detail) = serde_json::from_slice::<types::Detail>(&output) {
                    if let Some(md) = detail.markdown {
                        println!("{md}");
                    } else if let Some(t) = detail.text {
                        println!("{t}");
                    }
                }
            }
            _ => {
                print!("{}", String::from_utf8_lossy(&output));
            }
        }
    }

    Ok(())
}

fn run_extension_tui(
    alias: &str,
    _cfg: &Config,
    extension: &extensions::Extension,
    _ext_cfg: &ExtensionConfig,
) -> Result<()> {
    let items: Vec<ListItem> = extension
        .root_commands()
        .iter()
        .map(|cmd| ListItem {
            id: Some(format!("{alias} - {}", cmd.name)),
            title: cmd.title.clone(),
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
                    extension: Some(alias.to_string()),
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
        })
        .collect();

    let config_path = config::resolve_config_path();
    let mut history = history::History::load(&history::history_path())?;
    let mut sorted = items.clone();
    history.sort(&mut sorted);

    let actions = vec![Action {
        title: Some("Edit Extension".to_string()),
        key: Some("e".to_string()),
        action_type: ActionType::Edit,
        open: None,
        copy: None,
        run: None,
        exec: None,
        edit: Some(EditAction {
            path: extension.entrypoint.to_string_lossy().to_string(),
            exit: None,
            reload: Some(true),
        }),
        config: None,
        reload: None,
    }];

    let fallback_cfg = Config {
        oneliners: None,
        extensions: None,
        oneliner: None,
        path: config_path.clone(),
    };
    tui::run_root_list(
        &extension.manifest.title,
        &config_path,
        &mut history,
        config::load(&config_path).unwrap_or(fallback_cfg),
        sorted,
        actions,
    )
}

fn resolve_preferences(
    alias: &str,
    extension: &extensions::Extension,
    ext_cfg: &ExtensionConfig,
) -> serde_json::Map<String, serde_json::Value> {
    let mut prefs = ext_cfg.preferences.clone().unwrap_or_default();
    for input in extension.manifest.preferences.iter().flatten() {
        let env_name = format!(
            "{}_{}",
            alias.to_uppercase().replace('-', "_"),
            input.name.to_uppercase().replace('-', "_")
        );
        if let Ok(val) = std::env::var(&env_name) {
            let v: serde_json::Value = match input.input_type {
                InputType::String => serde_json::Value::String(val),
                InputType::Boolean => {
                    serde_json::Value::Bool(val.eq_ignore_ascii_case("true") || val == "1")
                }
                InputType::Number => serde_json::Value::String(val),
            };
            prefs.insert(input.name.clone(), v);
        }
        if !prefs.contains_key(&input.name) && input.optional.unwrap_or(false) {
            if let Some(ref default) = input.default {
                prefs.insert(input.name.clone(), default.clone());
            }
        }
    }
    prefs
}

fn parse_params(
    cmd_spec: &types::CommandSpec,
    args: &[String],
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let name: &'static str = Box::leak(cmd_spec.name.clone().into_boxed_str());
    let mut cmd = ClapCommand::new(name);
    for param in cmd_spec.params.iter().flatten() {
        let pname: &'static str = Box::leak(param.name.clone().into_boxed_str());
        let ptitle: &'static str = Box::leak(param.title.clone().into_boxed_str());
        let arg = Arg::new(pname).long(pname).help(ptitle);
        let arg = match param.input_type {
            InputType::String => arg.num_args(1),
            InputType::Boolean => arg.num_args(0),
            InputType::Number => arg.num_args(1),
        };
        cmd = cmd.arg(arg);
    }

    let iter: Vec<&str> = std::iter::once(cmd_spec.name.as_str())
        .chain(args.iter().map(String::as_str))
        .collect();
    let matches = cmd.try_get_matches_from(iter)?;
    let mut params = serde_json::Map::new();

    for param in cmd_spec.params.iter().flatten() {
        let pname: &'static str = Box::leak(param.name.clone().into_boxed_str());
        if matches.value_source(pname).is_none() {
            continue;
        }
        let v = match param.input_type {
            InputType::String => serde_json::Value::String(
                matches
                    .get_one::<String>(pname)
                    .cloned()
                    .unwrap_or_default(),
            ),
            InputType::Boolean => serde_json::Value::Bool(matches.get_flag(pname)),
            InputType::Number => {
                let s = matches
                    .get_one::<String>(pname)
                    .cloned()
                    .unwrap_or_default();
                serde_json::Value::String(s)
            }
        };
        params.insert(param.name.clone(), v);
    }

    Ok(params)
}

// ─── Config helpers ─────────────────────────────────────────────────────────

fn default_config_bytes() -> &'static [u8] {
    br#"{
    "oneliners": [
        { "title": "Open Sunbeam Docs", "command": "sunbeam open https://sunbeam.pomdtr.me/docs", "exit": true },
        { "title": "Open Sunbeam Repository", "command": "sunbeam open https://github.com/pomdtr/sunbeam", "exit": true }
    ]
}"#
}

fn load_config_or_default() -> Result<Config> {
    let config_path = config::resolve_config_path();
    if !config_path.exists() {
        if std::env::var("SUNBEAM_CONFIG").is_ok() {
            anyhow::bail!("config file not found: {}", config_path.display());
        }
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&config_path, default_config_bytes()).ok();
    }
    config::load(&config_path)
}

// ─── Core command handlers ──────────────────────────────────────────────────

fn run_validate(cmd: ValidateCommands) -> Result<()> {
    match cmd {
        ValidateCommands::List => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input)?;
            schemas::validate_list(&input).context("list is invalid")?;
            println!("List is valid!");
        }
        ValidateCommands::Detail => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input)?;
            schemas::validate_detail(&input).context("detail is invalid")?;
            println!("Detail is valid!");
        }
        ValidateCommands::Manifest => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input)?;
            schemas::validate_manifest(&input).context("manifest is invalid")?;
            println!("Manifest is valid!");
        }
        ValidateCommands::Config { path } => {
            let input = match path {
                Some(p) => std::fs::read(p)?,
                None if !atty::is(atty::Stream::Stdin) => {
                    let mut b = Vec::new();
                    std::io::stdin().read_to_end(&mut b)?;
                    b
                }
                _ => std::fs::read(config::resolve_config_path())?,
            };
            schemas::validate_config(&input).context("config is invalid")?;
            println!("Config is valid!");
        }
    }
    Ok(())
}

fn run_edit(args: EditArgs) -> Result<()> {
    let editor = utils::find_editor();

    if let Some(file) = args.file {
        std::process::Command::new("sh")
            .args(["-c", &format!("{editor} {file}")])
            .status()
            .context("editor exited with error")?;
        return Ok(());
    }
    if args.config {
        let p = config::resolve_config_path();
        std::process::Command::new("sh")
            .args(["-c", &format!("{editor} {}", p.display())])
            .status()
            .context("editor exited with error")?;
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;
    let ext = args.extension.as_deref().unwrap_or("");
    let name = if ext.is_empty() {
        "sunbeam-edit".into()
    } else {
        format!("sunbeam-edit.{ext}")
    };
    let path = temp_dir.path().join(&name);

    if !atty::is(atty::Stream::Stdin) {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        std::fs::write(&path, &buf)?;
    } else {
        std::fs::write(&path, "")?;
    }

    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;
    std::process::Command::new("sh")
        .args(["-c", &format!("{editor} {}", path.display())])
        .stdin(tty)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("editor exited with error")?;

    print!("{}", std::fs::read_to_string(&path)?);
    Ok(())
}

fn run_copy() -> Result<()> {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s)?;
    Clipboard::new()?.set_text(s)?;
    Ok(())
}

fn run_paste() -> Result<()> {
    print!("{}", Clipboard::new()?.get_text()?);
    Ok(())
}

fn run_extension(cmd: ExtensionCommands) -> Result<()> {
    let mut cfg = load_config_or_default()?;
    match cmd {
        ExtensionCommands::Install { origin, alias } => {
            let origin = utils::normalize_origin(&origin)?;
            let alias = alias.unwrap_or_else(|| {
                utils::extract_alias(&origin).unwrap_or_else(|_| "extension".into())
            });
            extensions::load_extension(&origin).context("failed to load extension")?;
            let exts = cfg
                .extensions
                .get_or_insert_with(std::collections::HashMap::new);
            if exts.contains_key(&alias) {
                anyhow::bail!("extension {alias} already exists");
            }
            exts.insert(
                alias.clone(),
                ExtensionConfig {
                    origin,
                    preferences: None,
                    root: None,
                },
            );
            cfg.save()?;
            println!("Installed {alias}");
        }
        ExtensionCommands::List => {
            if let Some(exts) = &cfg.extensions {
                for (a, e) in exts {
                    println!("{a}\t{}", e.origin);
                }
            }
        }
        ExtensionCommands::Remove { aliases } => {
            if let Some(exts) = &mut cfg.extensions {
                for a in &aliases {
                    exts.remove(a);
                }
            }
            cfg.save()?;
            if aliases.len() == 1 {
                println!("Removed {}", aliases[0]);
            } else {
                println!("Removed {} extensions", aliases.len());
            }
        }
        ExtensionCommands::Rename { old, new } => {
            let exts = cfg
                .extensions
                .get_or_insert_with(std::collections::HashMap::new);
            if exts.contains_key(&new) {
                anyhow::bail!("extension {new} already exists");
            }
            let ext = exts
                .remove(&old)
                .ok_or_else(|| anyhow::anyhow!("extension {old} not found"))?;
            exts.insert(new.clone(), ext);
            cfg.save()?;
            println!("Renamed {old} to {new}");
        }
        ExtensionCommands::Upgrade { alias, all } => {
            if let Some(a) = alias {
                let ec = cfg
                    .extensions
                    .as_ref()
                    .and_then(|e| e.get(&a))
                    .ok_or_else(|| anyhow::anyhow!("extension {a} not found"))?;
                extensions::upgrade(ec)?;
                println!("Upgraded {a}");
            } else if all {
                if let Some(exts) = &cfg.extensions {
                    for (a, ec) in exts {
                        extensions::upgrade(ec)?;
                        println!("Upgraded {a}");
                    }
                }
            } else {
                anyhow::bail!("provide an extension or use --all");
            }
        }
        ExtensionCommands::Configure { alias } => {
            let ec = cfg
                .extensions
                .as_ref()
                .and_then(|e| e.get(&alias))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("extension {alias} not found"))?;
            let ext = extensions::load_extension(&ec.origin)?;
            let prefs = ext.manifest.preferences.unwrap_or_default();
            if prefs.is_empty() {
                anyhow::bail!("extension {alias} has no preferences");
            }
            let inputs: Vec<Input> = prefs
                .into_iter()
                .map(|mut p| {
                    if let Some(m) = &ec.preferences {
                        if let Some(v) = m.get(&p.name) {
                            p.default = Some(v.clone());
                        }
                    }
                    p.optional = Some(false);
                    p
                })
                .collect();
            tui::run_form(&alias, &mut cfg, ec, inputs)?;
        }
        ExtensionCommands::Edit { alias } => {
            let ec = cfg
                .extensions
                .as_ref()
                .and_then(|e| e.get(&alias))
                .ok_or_else(|| anyhow::anyhow!("extension {alias} not found"))?;
            if ec.origin.starts_with("http://") || ec.origin.starts_with("https://") {
                anyhow::bail!("cannot edit remote extensions");
            }
            let path = cfg.resolve(&ec.origin);
            std::process::Command::new("sh")
                .args([
                    "-c",
                    &format!("{} {}", utils::find_editor(), path.display()),
                ])
                .status()
                .context("editor exited with error")?;
        }
    }
    Ok(())
}

// ─── Root command (no args) ─────────────────────────────────────────────────

fn run_root(cfg: &Config) -> Result<()> {
    let config_path = config::resolve_config_path();

    if !atty::is(atty::Stream::Stdout) {
        serde_json::to_writer_pretty(std::io::stdout(), cfg)?;
        return Ok(());
    }

    let mut history = history::History::load(&history::history_path())?;
    let mut items = build_root_items(cfg);
    history.sort(&mut items);

    tui::run_root_list(
        "Sunbeam",
        &config_path,
        &mut history,
        cfg.clone(),
        items,
        vec![],
    )
}

// ─── List item builders ─────────────────────────────────────────────────────

fn oneliner_list_items(oneliners: &[Oneliner]) -> Vec<ListItem> {
    oneliners
        .iter()
        .map(|o| ListItem {
            id: Some(format!("oneliner - {}", o.title)),
            title: o.title.clone(),
            subtitle: None,
            detail: None,
            accessories: Some(vec!["Oneliner".into()]),
            actions: Some(vec![
                Action {
                    title: Some("Run".into()),
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
                },
                Action {
                    title: Some("Copy Command".into()),
                    key: Some("c".into()),
                    action_type: ActionType::Copy,
                    open: None,
                    copy: Some(CopyAction {
                        text: Some(o.command.clone()),
                        exit: None,
                    }),
                    run: None,
                    exec: None,
                    edit: None,
                    config: None,
                    reload: None,
                },
            ]),
        })
        .collect()
}

fn extension_list_items(
    alias: &str,
    extension: &extensions::Extension,
    ext_cfg: &ExtensionConfig,
) -> Vec<ListItem> {
    let mut items = Vec::new();

    if let Some(root) = &ext_cfg.root {
        for r in root {
            items.push(ListItem {
                id: Some(format!("{alias} - {}", r.title)),
                title: r.title.clone(),
                subtitle: Some(extension.manifest.title.clone()),
                detail: None,
                accessories: Some(vec!["Command".into()]),
                actions: Some(vec![Action {
                    title: Some("Run".into()),
                    key: None,
                    action_type: ActionType::Run,
                    open: None,
                    copy: None,
                    run: Some(RunAction {
                        extension: Some(alias.into()),
                        command: r.command.clone(),
                        params: r.params.clone(),
                        reload: None,
                        exit: None,
                    }),
                    exec: None,
                    edit: None,
                    config: None,
                    reload: None,
                }]),
            });
        }
    }

    for cmd in extension.root_commands() {
        let mut acts = vec![Action {
            title: Some("Run".into()),
            key: None,
            action_type: ActionType::Run,
            open: None,
            copy: None,
            run: Some(RunAction {
                extension: Some(alias.into()),
                command: cmd.name.clone(),
                params: None,
                reload: None,
                exit: None,
            }),
            exec: None,
            edit: None,
            config: None,
            reload: None,
        }];
        if !extensions::is_remote(&ext_cfg.origin) {
            acts.push(Action {
                title: Some("Edit Extension".into()),
                key: Some("e".into()),
                action_type: ActionType::Edit,
                open: None,
                copy: None,
                run: None,
                exec: None,
                edit: Some(EditAction {
                    path: extension.entrypoint.to_string_lossy().into(),
                    exit: None,
                    reload: Some(true),
                }),
                config: None,
                reload: None,
            });
        }
        if extension
            .manifest
            .preferences
            .as_ref()
            .is_some_and(|p| !p.is_empty())
        {
            acts.push(Action {
                title: Some("Configure Extension".into()),
                key: Some("s".into()),
                action_type: ActionType::Config,
                open: None,
                copy: None,
                run: None,
                exec: None,
                edit: None,
                config: Some(types::ConfigAction {
                    extension: alias.into(),
                }),
                reload: None,
            });
        }
        items.push(ListItem {
            id: Some(format!("{alias} - {}", cmd.name)),
            title: cmd.title.clone(),
            subtitle: Some(extension.manifest.title.clone()),
            detail: None,
            accessories: Some(vec!["Command".into()]),
            actions: Some(acts),
        });
    }

    items
}

fn build_root_items(cfg: &Config) -> Vec<ListItem> {
    let mut items = Vec::new();
    if let Some(oneliners) = &cfg.oneliners {
        items.extend(oneliner_list_items(oneliners));
    }
    if let Some(exts) = &cfg.extensions {
        for (alias, ext_cfg) in exts {
            if let Ok(extension) = extensions::load_extension(&ext_cfg.origin) {
                items.extend(extension_list_items(alias, &extension, ext_cfg));
            }
        }
    }
    items
}
