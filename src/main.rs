mod builder;
mod config;
mod dispatch;
mod extensions;
mod history;
mod schemas;
mod tui;
mod types;
mod utils;

use clap::{CommandFactory, Parser, Subcommand, Args};

use anyhow::Result;

#[derive(Parser)]
#[command(
    name = "sunbeam",
    version,
    about = "Command Line Launcher",
    hide = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(subcommand)]
    Validate(ValidateCommands),
    Edit(EditArgs),
    Copy,
    Paste,
    Open { target: String },
    #[command(subcommand)]
    Extension(ExtensionCommands),
    /// Generate shell completions
    Completion {
        shell: String,
    },
    /// Format extension list output for FZF
    Fzf,
    /// Generate CLI documentation
    Docs,
}

#[derive(Subcommand)]
pub enum ValidateCommands {
    List,
    Detail,
    Manifest,
    Config { path: Option<String> },
}

#[derive(Args)]
pub struct EditArgs {
    pub file: Option<String>,
    #[arg(short = 'e', long = "extension")]
    pub extension: Option<String>,
    #[arg(short = 'c', long = "config", default_value_t = false)]
    pub config: bool,
}

#[derive(Subcommand)]
pub enum ExtensionCommands {
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
    /// Create a new extension from a template
    #[command(alias = "new")]
    Create {
        name: String,
        #[arg(short = 'l', long = "language", default_value = "deno")]
        language: String,
    },
}

fn main() -> Result<()> {
    let raw: Vec<String> = std::env::args().collect();

    let cfg = dispatch::load_config_or_default()?;

    if raw.len() <= 1 {
        return dispatch::run_root(&cfg);
    }

    let first = &raw[1];

    let is_core = matches!(
        first.as_str(),
        "validate" | "edit" | "copy" | "paste" | "open" | "extension" | "completion" | "fzf" | "docs"
    );
    if is_core {
        let cli = Cli::parse();
        return dispatch::dispatch_core(cli, &cfg);
    }

    if cfg
        .extensions
        .as_ref()
        .is_some_and(|e| e.contains_key(first))
    {
        return dispatch::run_extension_invocation(&cfg, first, &raw[2..]);
    }

    anyhow::bail!(
        "unknown command: {first}\n\n{}{}",
        Cli::command().render_help(),
        dispatch::extension_help(&cfg)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_validate() {
        let cli = Cli::parse_from(["sunbeam", "validate", "list"]);
        match cli.command {
            Some(Commands::Validate(cmd)) => {
                assert!(matches!(cmd, ValidateCommands::List));
            }
            _ => panic!("expected Validate command"),
        }
    }

    #[test]
    fn test_cli_parse_edit() {
        let cli = Cli::parse_from(["sunbeam", "edit", "test.txt"]);
        match cli.command {
            Some(Commands::Edit(args)) => {
                assert_eq!(args.file, Some("test.txt".into()));
            }
            _ => panic!("expected Edit command"),
        }
    }

    #[test]
    fn test_cli_parse_copy() {
        let cli = Cli::parse_from(["sunbeam", "copy"]);
        match cli.command {
            Some(Commands::Copy) => {}
            _ => panic!("expected Copy command"),
        }
    }

    #[test]
    fn test_cli_parse_extension_install() {
        let cli = Cli::parse_from(["sunbeam", "extension", "install", "https://example.com/ext.ts"]);
        match cli.command {
            Some(Commands::Extension(cmd)) => {
                match cmd {
                    ExtensionCommands::Install { origin, alias } => {
                        assert_eq!(origin, "https://example.com/ext.ts");
                        assert!(alias.is_none());
                    }
                    _ => panic!("expected Install command"),
                }
            }
            _ => panic!("expected Extension command"),
        }
    }

    #[test]
    fn test_cli_parse_extension_create() {
        let cli = Cli::parse_from(["sunbeam", "extension", "create", "myext.ts"]);
        match cli.command {
            Some(Commands::Extension(cmd)) => {
                match cmd {
                    ExtensionCommands::Create { name, language } => {
                        assert_eq!(name, "myext.ts");
                        assert_eq!(language, "deno");
                    }
                    _ => panic!("expected Create command"),
                }
            }
            _ => panic!("expected Extension command"),
        }
    }

    #[test]
    fn test_cli_parse_extension_create_with_lang() {
        let cli = Cli::parse_from(["sunbeam", "extension", "create", "script.py", "--language", "python"]);
        match cli.command {
            Some(Commands::Extension(cmd)) => {
                match cmd {
                    ExtensionCommands::Create { name, language } => {
                        assert_eq!(name, "script.py");
                        assert_eq!(language, "python");
                    }
                    _ => panic!("expected Create command"),
                }
            }
            _ => panic!("expected Extension command"),
        }
    }

    #[test]
    fn test_cli_parse_completion() {
        let cli = Cli::parse_from(["sunbeam", "completion", "bash"]);
        match cli.command {
            Some(Commands::Completion { shell }) => {
                assert_eq!(shell, "bash");
            }
            _ => panic!("expected Completion command"),
        }
    }

    #[test]
    fn test_cli_parse_fzf() {
        let cli = Cli::parse_from(["sunbeam", "fzf"]);
        match cli.command {
            Some(Commands::Fzf) => {}
            _ => panic!("expected Fzf command"),
        }
    }

    #[test]
    fn test_cli_parse_docs() {
        let cli = Cli::parse_from(["sunbeam", "docs"]);
        match cli.command {
            Some(Commands::Docs) => {}
            _ => panic!("expected Docs command"),
        }
    }

}
