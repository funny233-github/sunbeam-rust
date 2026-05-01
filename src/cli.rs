use std::io::Read;

use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::config::{self, Config, ExtensionConfig, Oneliner};
use crate::extensions;
use crate::history;
use crate::types::{self, Action, ActionType, CopyAction, EditAction, ExecAction, Input, ListItem, RunAction};
use crate::utils;
use crate::tui;

/// Builds the top-level CLI definition with all subcommands.
pub fn build_cli() -> Command {
    Command::new("sunbeam")
        .about("Command Line Launcher")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(
            Command::new("validate")
                .about("Validate a Sunbeam schema")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list")
                        .about("Validate a list")
                )
                .subcommand(
                    Command::new("detail")
                        .about("Validate a detail")
                )
                .subcommand(
                    Command::new("manifest")
                        .about("Validate a manifest")
                )
                .subcommand(
                    Command::new("config")
                        .about("Validate a config")
                        .arg(Arg::new("path").help("config file path"))
                )
        )
        .subcommand(
            Command::new("edit")
                .about("Open a file in your editor")
                .arg(Arg::new("file").help("file to edit"))
                .arg(Arg::new("extension")
                    .short('e')
                    .long("extension")
                    .help("File extension for temp file"))
                .arg(Arg::new("config")
                    .short('c')
                    .long("config")
                    .help("Edit config file")
                    .action(ArgAction::SetTrue))
        )
        .subcommand(
            Command::new("copy")
                .about("Copy text from stdin to clipboard")
        )
        .subcommand(
            Command::new("paste")
                .about("Paste text from clipboard to stdout")
        )
        .subcommand(
            Command::new("open")
                .about("Open a file or URL")
                .arg(Arg::new("target").required(true))
        )
        .subcommand(
            Command::new("extension")
                .about("Manage sunbeam extensions")
                .subcommand_required(true)
                .subcommand(
                    Command::new("install")
                        .about("Install an extension")
                        .alias("add")
                        .arg(Arg::new("origin").required(true))
                        .arg(Arg::new("alias").long("alias").short('a'))
                )
                .subcommand(
                    Command::new("list")
                        .about("List extensions")
                        .alias("ls")
                )
                .subcommand(
                    Command::new("remove")
                        .about("Remove extensions")
                        .alias("rm")
                        .alias("uninstall")
                        .arg(Arg::new("aliases").required(true).num_args(1..))
                )
                .subcommand(
                    Command::new("rename")
                        .about("Rename an extension")
                        .alias("mv")
                        .arg(Arg::new("old").required(true))
                        .arg(Arg::new("new").required(true))
                )
                .subcommand(
                    Command::new("upgrade")
                        .about("Upgrade extensions")
                        .arg(Arg::new("alias"))
                        .arg(Arg::new("all").long("all").action(ArgAction::SetTrue))
                )
                .subcommand(
                    Command::new("configure")
                        .about("Configure extension preferences")
                        .alias("config")
                        .arg(Arg::new("alias").required(true))
                )
                .subcommand(
                    Command::new("edit")
                        .about("Edit an extension script")
                        .arg(Arg::new("alias").required(true))
                )
        )
}

/// Default sunbeam.json content used when no config file exists.
fn default_config_bytes() -> &'static [u8] {
    br#"{
    "oneliners": [
        { "title": "Open Sunbeam Docs", "command": "sunbeam open https://sunbeam.pomdtr.me/docs", "exit": true },
        { "title": "Open Sunbeam Repository", "command": "sunbeam open https://github.com/pomdtr/sunbeam", "exit": true }
    ]
}"#
}

/// Returns `true` when the `SUNBEAM` environment variable is set (i.e. the
/// current process was spawned by another sunbeam process).
#[allow(dead_code)]
fn is_sunbeam_running() -> bool {
    std::env::var("SUNBEAM").is_ok()
}

/// Loads the config, creating a default one if the file does not exist.
///
/// # Panics
/// Exits the process on error.
fn load_config_or_default() -> Config {
    let config_path = config::resolve_config_path();
    if !config_path.exists() {
        if std::env::var("SUNBEAM_CONFIG").is_ok() {
            eprintln!("config file not found: {}", config_path.display());
            std::process::exit(1);
        }
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&config_path, default_config_bytes()).ok();
    }
    config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("error loading config: {}", e);
        std::process::exit(1);
    })
}

/// Entry point: parses CLI arguments and dispatches to the appropriate handler.
///
/// # Example
/// ```
/// cli::run()?; // equivalent to `sunbeam [subcommand]`
/// ```
///
/// # Errors
/// Propagates errors from the dispatched subcommand.
pub fn run() -> Result<()> {
    let cli = build_cli();
    let matches = cli.get_matches();

    match matches.subcommand() {
        Some(("validate", sub)) => run_validate(sub)?,
        Some(("edit", sub)) => run_edit(sub)?,
        Some(("copy", _)) => run_copy()?,
        Some(("paste", _)) => run_paste()?,
        Some(("open", sub)) => run_open(sub)?,
        Some(("extension", sub)) => run_extension(sub)?,
        _ => run_root()?,
    }

    Ok(())
}

/// Handles `sunbeam validate <list|detail|manifest|config>`.
///
/// Reads JSON from stdin (or a file for `validate config`) and validates it
/// against the corresponding JSON Schema.
fn run_validate(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("list", _)) => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input)?;
            crate::schemas::validate_list(&input)
                .context("list is invalid")?;
            println!("List is valid!");
        }
        Some(("detail", _)) => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input)?;
            crate::schemas::validate_detail(&input)
                .context("detail is invalid")?;
            println!("Detail is valid!");
        }
        Some(("manifest", _)) => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input)?;
            crate::schemas::validate_manifest(&input)
                .context("manifest is invalid")?;
            println!("Manifest is valid!");
        }
        Some(("config", sub)) => {
            let input = if let Some(path) = sub.get_one::<String>("path") {
                std::fs::read(path)?
            } else {
                let mut buf = Vec::new();
                if atty::is(atty::Stream::Stdin) {
                    let config_path = config::resolve_config_path();
                    std::fs::read(&config_path)?
                } else {
                    std::io::stdin().read_to_end(&mut buf)?;
                    buf
                }
            };
            crate::schemas::validate_config(&input)
                .context("config is invalid")?;
            println!("Config is valid!");
        }
        _ => unreachable!(),
    }
    Ok(())
}

/// Handles `sunbeam edit [file]`.
///
/// Opens the specified file, or the config file (`--config`), or creates a temp
/// file pre-populated from stdin and outputs the edited result.
fn run_edit(matches: &ArgMatches) -> Result<()> {
    let edit_config = matches.get_flag("config");
    let extension = matches.get_one::<String>("extension");

    if let Some(file) = matches.get_one::<String>("file") {
        let editor = utils::find_editor();
        std::process::Command::new("sh")
            .args(["-c", &format!("{} {}", editor, file)])
            .status()
            .context("editor exited with error")?;
        return Ok(());
    }

    if edit_config {
        let config_path = config::resolve_config_path();
        let editor = utils::find_editor();
        std::process::Command::new("sh")
            .args(["-c", &format!("{} {}", editor, config_path.display())])
            .status()
            .context("editor exited with error")?;
        return Ok(());
    }

    let temp_dir = tempfile::tempdir()?;
    let ext_str = extension.map(|s| s.as_str()).unwrap_or("");
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

    let editor = utils::find_editor();
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

/// Handles `sunbeam copy`: reads stdin and copies it to the clipboard.
fn run_copy() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(input)?;
    Ok(())
}

/// Handles `sunbeam paste`: reads clipboard content and writes to stdout.
fn run_paste() -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    let text = clipboard.get_text()?;
    print!("{}", text);
    Ok(())
}

/// Handles `sunbeam open <target>`.
fn run_open(matches: &ArgMatches) -> Result<()> {
    let target = matches.get_one::<String>("target").unwrap();
    utils::open_target(target)
}

/// Dispatches all `sunbeam extension <subcommand>` actions.
fn run_extension(matches: &ArgMatches) -> Result<()> {
    let mut cfg = load_config_or_default();

    match matches.subcommand() {
        Some(("install", sub)) => {
            let origin = sub.get_one::<String>("origin").unwrap();
            let origin = utils::normalize_origin(origin)?;
            let alias = sub.get_one::<String>("alias").cloned()
                .unwrap_or_else(|| utils::extract_alias(&origin).unwrap_or_else(|_| "extension".to_string()));

            extensions::load_extension(&origin)
                .context("failed to load extension")?;

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
        Some(("list", _)) => {
            if let Some(exts) = &cfg.extensions {
                for (alias, ext) in exts {
                    println!("{}\t{}", alias, ext.origin);
                }
            }
        }
        Some(("remove", sub)) => {
            let aliases: Vec<&String> = sub.get_many("aliases").unwrap().collect();
            if let Some(exts) = &mut cfg.extensions {
                for alias in &aliases {
                    exts.remove(*alias);
                }
            }
            cfg.save()?;
            if aliases.len() == 1 {
                println!("Removed {}", aliases[0]);
            } else {
                println!("Removed {} extensions", aliases.len());
            }
        }
        Some(("rename", sub)) => {
            let old = sub.get_one::<String>("old").unwrap();
            let new = sub.get_one::<String>("new").unwrap();
            let exts = cfg.extensions.get_or_insert_with(std::collections::HashMap::new);
            if exts.contains_key(new) {
                anyhow::bail!("extension {} already exists", new);
            }
            if let Some(ext) = exts.remove(old) {
                exts.insert(new.clone(), ext);
            } else {
                anyhow::bail!("extension {} not found", old);
            }
            cfg.save()?;
            println!("Renamed {} to {}", old, new);
        }
        Some(("upgrade", sub)) => {
            let all = sub.get_flag("all");
            let alias = sub.get_one::<String>("alias");

            if let Some(alias) = alias {
                if let Some(ext_cfg) = cfg.extensions.as_ref().and_then(|e| e.get(alias)) {
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
        Some(("configure", sub)) => {
            let alias = sub.get_one::<String>("alias").unwrap();
            let ext_cfg = cfg.extensions.as_ref()
                .and_then(|e| e.get(alias))
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

            tui::run_form(alias, &mut cfg, ext_cfg, inputs)?;
        }
        Some(("edit", sub)) => {
            let alias = sub.get_one::<String>("alias").unwrap();
            let ext_cfg = cfg.extensions.as_ref()
                .and_then(|e| e.get(alias))
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
        _ => unreachable!(),
    }
    Ok(())
}

/// Handles the default case (no subcommand, or piped output).
///
/// If stdout is not a TTY, the config is printed as JSON. Otherwise the TUI
/// root list is launched.
fn run_root() -> Result<()> {
    let cfg = load_config_or_default();

    if !atty::is(atty::Stream::Stdout) {
        serde_json::to_writer_pretty(std::io::stdout(), &cfg)?;
        return Ok(());
    }

    let config_path = config::resolve_config_path();
    let mut history = history::History::load(&history::history_path())?;

    let mut items = build_root_items(&cfg);
    history.sort(&mut items);

    tui::run_root_list("Sunbeam", &config_path, &mut history, cfg, items)
}

/// Converts `Oneliner` config entries into `ListItem`s for the root list.
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

/// Converts an installed extension and its commands into `ListItem`s.
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

/// Builds the full list of root items from oneliners and all installed extensions.
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
