use crate::config::{Config, ExtensionConfig, Oneliner};
use crate::extensions;
use crate::types;
use crate::types::*;

pub fn oneliner_list_items(oneliners: &[Oneliner]) -> Vec<ListItem> {
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

pub fn extension_list_items(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RootItem;

    fn make_test_config() -> Config {
        Config {
            oneliners: None,
            extensions: None,
            oneliner: None,
            path: std::path::PathBuf::from("/tmp/test.json"),
        }
    }

    #[test]
    fn test_oneliner_list_items_basic() {
        let oneliners = vec![Oneliner {
            title: "Hello".into(),
            command: "echo hello".into(),
            interactive: Some(false),
            cwd: None,
            exit: None,
        }];
        let items = oneliner_list_items(&oneliners);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Hello");
        assert_eq!(items[0].accessories.as_ref().unwrap(), &["Oneliner"]);
    }

    #[test]
    fn test_oneliner_list_items_has_actions() {
        let oneliners = vec![Oneliner {
            title: "Test".into(),
            command: "echo test".into(),
            interactive: None,
            cwd: None,
            exit: None,
        }];
        let items = oneliner_list_items(&oneliners);
        let actions = items[0].actions.as_ref().unwrap();
        assert_eq!(actions.len(), 2);
        assert!(actions.iter().any(|a| a.action_type == ActionType::Exec));
        assert!(actions.iter().any(|a| a.action_type == ActionType::Copy));
    }

    #[test]
    fn test_build_root_items_empty() {
        let items = build_root_items(&make_test_config());
        assert!(items.is_empty());
    }

    #[test]
    fn test_build_root_items_with_oneliners() {
        let cfg = Config {
            oneliners: Some(vec![Oneliner {
                title: "One".into(),
                command: "echo one".into(),
                interactive: None,
                cwd: None,
                exit: None,
            }]),
            ..make_test_config()
        };
        let items = build_root_items(&cfg);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "One");
    }

    #[test]
    fn test_extension_list_items_has_root_items() {
        let ext_cfg = ExtensionConfig {
            origin: "https://example.com/ext.ts".into(),
            preferences: None,
            root: Some(vec![RootItem {
                title: "Quick Action".into(),
                command: "quick".into(),
                params: None,
            }]),
        };

        let manifest = types::Manifest {
            title: "Test Ext".into(),
            description: None,
            preferences: None,
            commands: vec![types::CommandSpec {
                name: "main".into(),
                title: "Main".into(),
                hidden: None,
                params: None,
                mode: None,
            }],
        };
        let extension = crate::extensions::Extension {
            manifest,
            entrypoint: std::path::PathBuf::from("/tmp/ext.ts"),
        };

        let items = extension_list_items("test-ext", &extension, &ext_cfg);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Quick Action");
        assert_eq!(items[1].title, "Main");
    }
}

pub fn build_root_items(cfg: &Config) -> Vec<ListItem> {
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
