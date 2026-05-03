//! Demonstrates all CommandSpec attribute combinations: different modes,
//! optional/required parameters with defaults, hidden commands, and more.
//!
//! Constructs 6+ command specifications, builds a Manifest, serializes it,
//! and shows how each variant is used in the sunbeam extension system.
//!
//! Usage: cargo run --example command_spec_variants

fn main() {
    use sunbeam_rust::types::*;

    let commands = vec![
        // 1. Minimal command: name + title, no frills
        CommandSpec {
            name: "ping".into(),
            title: "Ping Server".into(),
            hidden: None,
            params: None,
            mode: None,
        },
        // 2. Filter mode with string and number params
        CommandSpec {
            name: "list".into(),
            title: "List Items".into(),
            hidden: None,
            params: Some(vec![
                Input {
                    input_type: InputType::String,
                    name: "category".into(),
                    title: "Category".into(),
                    optional: Some(true),
                    default: Some(serde_json::json!("all")),
                },
                Input {
                    input_type: InputType::Number,
                    name: "limit".into(),
                    title: "Max Items".into(),
                    optional: Some(true),
                    default: Some(serde_json::json!(20)),
                },
            ]),
            mode: Some(CommandMode::Filter),
        },
        // 3. Search mode with a string param
        CommandSpec {
            name: "search".into(),
            title: "Search".into(),
            hidden: None,
            params: Some(vec![Input {
                input_type: InputType::String,
                name: "query".into(),
                title: "Search Query".into(),
                optional: None,
                default: None,
            }]),
            mode: Some(CommandMode::Search),
        },
        // 4. Detail mode with optional param + default value
        CommandSpec {
            name: "info".into(),
            title: "Show Info".into(),
            hidden: None,
            params: Some(vec![Input {
                input_type: InputType::String,
                name: "section".into(),
                title: "Info Section".into(),
                optional: Some(true),
                default: Some(serde_json::json!("overview")),
            }]),
            mode: Some(CommandMode::Detail),
        },
        // 5. TTY mode with a boolean param
        CommandSpec {
            name: "deploy".into(),
            title: "Deploy App".into(),
            hidden: None,
            params: Some(vec![Input {
                input_type: InputType::Boolean,
                name: "force".into(),
                title: "Force Deploy".into(),
                optional: None,
                default: Some(serde_json::json!(false)),
            }]),
            mode: Some(CommandMode::Tty),
        },
        // 6. Silent mode, no params
        CommandSpec {
            name: "cleanup".into(),
            title: "Cleanup Temp Files".into(),
            hidden: None,
            params: None,
            mode: Some(CommandMode::Silent),
        },
        // 7. Hidden command (won't show in root list)
        CommandSpec {
            name: "_internal".into(),
            title: "Internal Helper".into(),
            hidden: Some(true),
            params: Some(vec![Input {
                input_type: InputType::String,
                name: "key".into(),
                title: "Auth Key".into(),
                optional: None,
                default: None,
            }]),
            mode: Some(CommandMode::Silent),
        },
    ];

    // Build a Manifest containing all these commands
    let manifest = Manifest {
        title: "DevOps Toolkit".into(),
        description: Some("Collection of devops utilities".into()),
        preferences: None,
        commands,
    };

    // Serialize and print the full manifest
    println!("=== Full Manifest ===");
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    println!("{}", json);
    println!();

    // Print summary table
    println!("=== Command Summary ===");
    println!("  {:<12} {:<20} {:<8} {:<15} Params", "Name", "Title", "Mode", "Hidden");
    println!("  {}", "-".repeat(70));
    for cmd in &manifest.commands {
        let mode = cmd
            .mode
            .as_ref()
            .map(|m| format!("{m:?}"))
            .unwrap_or_else(|| "None".into());
        let hidden = if cmd.hidden.unwrap_or(false) {
            "hidden"
        } else {
            "visible"
        };
        let param_count = cmd
            .params
            .as_ref()
            .map(|p| p.len().to_string())
            .unwrap_or_else(|| "0".into());
        println!(
            "  {:<12} {:<20} {:<8} {:<15} {} params",
            cmd.name, cmd.title, mode, hidden, param_count
        );

        // Show param details
        if let Some(params) = &cmd.params {
            for param in params {
                let opt = if param.optional.unwrap_or(false) {
                    "optional"
                } else {
                    "required"
                };
                let default_str = param
                    .default
                    .as_ref()
                    .map(|d| format!("default: {d}"))
                    .unwrap_or_else(|| "no default".into());
                println!(
                    "    - {:?} {} ({}), {}",
                    param.input_type, param.name, opt, default_str
                );
            }
        }
    }

    // Demonstrate root_commands filtering
    println!("\n=== Root Commands (non-hidden) ===");
    let root_cmds: Vec<&CommandSpec> = manifest
        .commands
        .iter()
        .filter(|c| !c.hidden.unwrap_or(false))
        .collect();
    for cmd in &root_cmds {
        println!("  {} — {}", cmd.name, cmd.title);
    }
    println!(
        "  ({} visible of {} total)",
        root_cmds.len(),
        manifest.commands.len()
    );

    println!("\n✅ CommandSpec variants demo complete!");
}
