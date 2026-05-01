mod config;
mod extensions;
mod history;
mod schemas;
mod tui;
mod types;
mod utils;

use std::io::Read;

use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::{Parser, Subcommand, Args};
use crate::config::{Config, ExtensionConfig, Oneliner};
use crate::types::{Action, ActionType, CopyAction, EditAction, ExecAction, Input, ListItem, RunAction};

#[derive(Parser)]
#[command(name = "sunbeam", version, about = "Command Line Launcher")]
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
    /// Validate a list
    List,
    /// Validate a detail
    Detail,
    /// Validate a manifest
    Manifest,
    /// Validate a config
    Config { path: Option<String> },
}

#[derive(Args)]
struct EditArgs {
    /// File to edit
    file: Option<String>,
    /// File extension for temp file
    #[arg(short = 'e', long = "extension")]
    extension: Option<String>,
    /// Edit config file
    #[arg(short = 'c', long = "config", default_value_t = false)]
    config: bool,
}

#[derive(Subcommand)]
enum ExtensionCommands {
    /// Install an extension
    #[command(alias = "add")]
    Install {
        origin: String,
        #[arg(long, short)]
        alias: Option<String>,
    },
    /// List extensions
    #[command(alias = "ls")]
    List,
    /// Remove extensions
    #[command(alias = "rm", alias = "uninstall")]
    Remove {
        #[arg(required = true)]
        aliases: Vec<String>,
    },
    /// Rename an extension
    #[command(alias = "mv")]
    Rename { old: String, new: String },
    /// Upgrade extensions
    Upgrade {
        alias: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Configure extension preferences
    #[command(alias = "config")]
    Configure { alias: String },
    /// Edit an extension script
    Edit { alias: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => run_root()?,
        Some(Commands::Validate(cmd)) => run_validate(cmd)?,
        Some(Commands::Edit(args)) => run_edit(args)?,
        Some(Commands::Copy) => run_copy()?,
        Some(Commands::Paste) => run_paste()?,
        Some(Commands::Open { target }) => utils::open_target(&target)?,
        Some(Commands::Extension(cmd)) => run_extension(cmd)?,
    }

    Ok(())
}

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
                    let mut buf = Vec::new();
                    std::io::stdin().read_to_end(&mut buf)?;
                    buf
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
            .args(["-c", &format!("{} {}", editor, file)])
            .status()
            .context("editor exited with error")?;
        return Ok(());
    }

    if args.config {
        let config_path = config::resolve_config_path();
        std::process::Command::new("sh")
            .args(["-c", &format!("{} {}", editor, config_path.display())])
            .status()
            .context("editor exited with error")?;
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;
    let ext_str = args.extension.as_deref().unwrap_or("");
    let file_name = if ext_str.is_empty() {
        "sunbeam-edit".to_string()
    } else {
        format!("sunbeam-edit.{}", ext_str)
    };
    let temp_path = temp_dir.path().join(&file_name);

    if !atty::is(atty::Stream::Stdin) {
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input)?;
        std::fs::write(&temp_path, &input)?;
    } else {
        std::fs::write(&temp_path, "")?;
    }

    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;
    std::process::Command::new("sh")
        .args(["-c", &format!("{} {}", editor, temp_path.display())])
        .stdin(tty)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("editor exited with error")?;

    let content = std::fs::read_to_string(&temp_path)?;
    print!("{}", content);
    Ok(())
}

fn run_copy() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(input)?;
    Ok(())
}

fn run_paste() -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    let text = clipboard.get_text()?;
    print!("{}", text);
    Ok(())
}

fn run_extension(cmd: ExtensionCommands) -> Result<()> {
    let mut cfg = load_config_or_default()?;

    match cmd {
        ExtensionCommands::Install { origin, alias } => {
            let origin = utils::normalize_origin(&origin)?;
            let alias = alias.unwrap_or_else(|| {
                utils::extract_alias(&origin).unwrap_or_else(|_| "extension".to_string())
            });

            extensions::load_extension(&origin).context("failed to load extension")?;

            let exts = cfg.extensions.get_or_insert_with(std::collections::HashMap::new);
            if exts.contains_key(&alias) {
                anyhow::bail!("extension {} already exists", alias);
            }
            exts.insert(alias.clone(), ExtensionConfig {
                origin,
                preferences: None,
                root: None,
            });
            cfg.save()?;
            println!("Installed {}", alias);
        }
        ExtensionCommands::List => {
            if let Some(exts) = &cfg.extensions {
                for (alias, ext) in exts {
                    println!("{}\t{}", alias, ext.origin);
                }
            }
        }
        ExtensionCommands::Remove { aliases } => {
            if let Some(exts) = &mut cfg.extensions {
                for alias in &aliases {
                    exts.remove(alias);
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
            let exts = cfg.extensions.get_or_insert_with(std::collections::HashMap::new);
            if exts.contains_key(&new) {
                anyhow::bail!("extension {} already exists", new);
            }
            if let Some(ext) = exts.remove(&old) {
                exts.insert(new.clone(), ext);
            } else {
                anyhow::bail!("extension {} not found", old);
            }
            cfg.save()?;
            println!("Renamed {} to {}", old, new);
        }
        ExtensionCommands::Upgrade { alias, all } => {
            if let Some(alias) = alias {
                if let Some(ext_cfg) = cfg.extensions.as_ref().and_then(|e| e.get(&alias)) {
                    extensions::upgrade(ext_cfg)?;
                    println!("Upgraded {}", alias);
                } else {
                    anyhow::bail!("extension {} not found", alias);
                }
            } else if all {
                if let Some(exts) = &cfg.extensions {
                    for (alias, ext_cfg) in exts {
                        extensions::upgrade(ext_cfg)?;
                        println!("Upgraded {}", alias);
                    }
                }
            } else {
                anyhow::bail!("provide an extension or use --all");
            }
        }
        ExtensionCommands::Configure { alias } => {
            let ext_cfg = cfg.extensions.as_ref()
                .and_then(|e| e.get(&alias))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("extension {} not found", alias))?;

            let extension = extensions::load_extension(&ext_cfg.origin)?;
            let prefs = extension.manifest.preferences.unwrap_or_default();
            if prefs.is_empty() {
                anyhow::bail!("extension {} has no preferences", alias);
            }

            let inputs: Vec<Input> = prefs.into_iter().map(|mut p| {
                if let Some(prefs_map) = &ext_cfg.preferences {
                    if let Some(val) = prefs_map.get(&p.name) {
                        p.default = Some(val.clone());
                    }
                }
                p.optional = Some(false);
                p
            }).collect();

            tui::run_form(&alias, &mut cfg, ext_cfg, inputs)?;
        }
        ExtensionCommands::Edit { alias } => {
            let ext_cfg = cfg.extensions.as_ref()
                .and_then(|e| e.get(&alias))
                .ok_or_else(|| anyhow::anyhow!("extension {} not found", alias))?;
            if ext_cfg.origin.starts_with("http://") || ext_cfg.origin.starts_with("https://") {
                anyhow::bail!("cannot edit remote extensions");
            }
            let path = cfg.resolve(&ext_cfg.origin);
            let editor = utils::find_editor();
            std::process::Command::new("sh")
                .args(["-c", &format!("{} {}", editor, path.display())])
                .status()
                .context("editor exited with error")?;
        }
    }
    Ok(())
}

fn run_root() -> Result<()> {
    let cfg = load_config_or_default()?;
    let config_path = config::resolve_config_path();

    if !atty::is(atty::Stream::Stdout) {
        serde_json::to_writer_pretty(std::io::stdout(), &cfg)?;
        return Ok(());
    }

    let mut history = history::History::load(&history::history_path())?;

    let mut items = build_root_items(&cfg);
    history.sort(&mut items);

    tui::run_root_list("Sunbeam", &config_path, &mut history, cfg, items)
}

fn oneliner_list_items(oneliners: &[Oneliner]) -> Vec<ListItem> {
    oneliners.iter().map(|o| ListItem {
        id: Some(format!("oneliner - {}", o.title)),
        title: o.title.clone(),
        subtitle: None,
        detail: None,
        accessories: Some(vec!["Oneliner".to_string()]),
        actions: Some(vec![
            Action {
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
            },
            Action {
                title: Some("Copy Command".to_string()),
                key: Some("c".to_string()),
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
    }).collect()
}

fn extension_list_items(alias: &str, extension: &extensions::Extension, ext_cfg: &ExtensionConfig) -> Vec<ListItem> {
    let mut items = Vec::new();

    if let Some(root_items) = &ext_cfg.root {
        for root_item in root_items {
            items.push(ListItem {
                id: Some(format!("{} - {}", alias, root_item.title)),
                title: root_item.title.clone(),
                subtitle: Some(extension.manifest.title.clone()),
                detail: None,
                accessories: Some(vec!["Command".to_string()]),
                actions: Some(vec![
                    Action {
                        title: Some("Run".to_string()),
                        key: None,
                        action_type: ActionType::Run,
                        open: None,
                        copy: None,
                        run: Some(RunAction {
                            extension: Some(alias.to_string()),
                            command: root_item.command.clone(),
                            params: root_item.params.clone(),
                            reload: None,
                            exit: None,
                        }),
                        exec: None,
                        edit: None,
                        config: None,
                        reload: None,
                    },
                ]),
            });
        }
    }

    for command in extension.root_commands() {
        let mut actions = vec![
            Action {
                title: Some("Run".to_string()),
                key: None,
                action_type: ActionType::Run,
                open: None,
                copy: None,
                run: Some(RunAction {
                    extension: Some(alias.to_string()),
                    command: command.name.clone(),
                    params: None,
                    reload: None,
                    exit: None,
                }),
                exec: None,
                edit: None,
                config: None,
                reload: None,
            },
        ];

        if !extensions::is_remote(&ext_cfg.origin) {
            actions.push(Action {
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
            });
        }

        if let Some(prefs) = &extension.manifest.preferences {
            if !prefs.is_empty() {
                actions.push(Action {
                    title: Some("Configure Extension".to_string()),
                    key: Some("s".to_string()),
                    action_type: ActionType::Config,
                    open: None,
                    copy: None,
                    run: None,
                    exec: None,
                    edit: None,
                    config: Some(types::ConfigAction {
                        extension: alias.to_string(),
                    }),
                    reload: None,
                });
            }
        }

        items.push(ListItem {
            id: Some(format!("{} - {}", alias, command.name)),
            title: command.title.clone(),
            subtitle: Some(extension.manifest.title.clone()),
            detail: None,
            accessories: Some(vec!["Command".to_string()]),
            actions: Some(actions),
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
