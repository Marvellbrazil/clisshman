use inquire::{Select, Text};

use crate::cli::{AddArgs, ConnectArgs, DeleteArgs, EditArgs, ListArgs};
use crate::config::IdType;
use crate::service::AppService;
use crate::storage::StorageManager;

pub struct Wizard;

impl Wizard {
    pub fn start(service: &AppService, storage: &StorageManager) -> Result<(), String> {
        let mut cfg = storage.load_config()?;
        let reg_path = storage.registry_path();
        if !reg_path.exists() {
            println!("Welcome to Clisshman! Initializing first-run configuration...\n");
            let id_choice = Select::new(
                "Select session ID generation format:",
                vec!["Incremental Integer (e.g. 1, 2, 3)", "UUID v4 (e.g. a1b2c3d4...)"],
            )
            .prompt()
            .map_err(|e| e.to_string())?;

            if id_choice.starts_with("UUID") {
                cfg.id_type = IdType::Uuid;
            } else {
                cfg.id_type = IdType::Integer;
            }
            storage.save_config(&cfg)?;
            println!("Configuration saved.\n");
        }

        loop {
            let options = vec![
                "1. Connect to SSH session",
                "2. List all sessions",
                "3. Add new session",
                "4. Edit session",
                "5. Delete session",
                "6. Ping session",
                "7. Exit",
            ];

            let choice = Select::new("Clisshman Interactive Menu", options)
                .prompt()
                .map_err(|e| e.to_string())?;

            match choice {
                c if c.starts_with("1") => {
                    let reg = storage.load_registry()?;
                    if reg.sessions.is_empty() {
                        println!("No sessions available. Add one first!");
                        continue;
                    }
                    let session_items: Vec<String> = reg
                        .sessions
                        .iter()
                        .map(|s| format!("{} | {} ({})", s.id, s.name, s.host))
                        .collect();

                    let selected = Select::new("Choose session to connect:", session_items)
                        .prompt()
                        .map_err(|e| e.to_string())?;

                    let id = selected.split('|').next().unwrap().trim();
                    let _ = service.handle_connect(ConnectArgs {
                        target: id.to_string(),
                        group: None,
                        dry_run: false,
                        passthrough: vec![],
                    });
                }
                c if c.starts_with("2") => {
                    service.handle_list(ListArgs::default(), false)?;
                }
                c if c.starts_with("3") => {
                    let host = Text::new("Remote Hostname / IP:")
                        .prompt()
                        .map_err(|e| e.to_string())?;
                    let user = Text::new("Username:")
                        .prompt()
                        .map_err(|e| e.to_string())?;
                    let name_input = Text::new("Display Name (leave empty for user@host):")
                        .prompt()
                        .map_err(|e| e.to_string())?;
                    let group_input = Text::new("Group (optional):")
                        .prompt()
                        .map_err(|e| e.to_string())?;
                    let port_input = Text::new("Port (default 22):")
                        .prompt()
                        .map_err(|e| e.to_string())?;
                    let tags_input = Text::new("Tags (comma separated, e.g. web, debian):")
                        .prompt()
                        .map_err(|e| e.to_string())?;

                    let port = if port_input.trim().is_empty() {
                        None
                    } else {
                        Some(port_input.trim().parse::<u16>().map_err(|_| "Invalid port")?)
                    };

                    let tags = if tags_input.trim().is_empty() {
                        vec![]
                    } else {
                        vec![tags_input]
                    };

                    service.handle_add(
                        AddArgs {
                            target: None,
                            name: if name_input.trim().is_empty() { None } else { Some(name_input) },
                            group: if group_input.trim().is_empty() { None } else { Some(group_input) },
                            host: Some(host),
                            username: Some(user),
                            port,
                            identity_file: None,
                            jump_host: None,
                            description: None,
                            tags,
                        },
                        false,
                    )?;
                }
                c if c.starts_with("4") => {
                    let reg = storage.load_registry()?;
                    if reg.sessions.is_empty() {
                        println!("No sessions available to edit.");
                        continue;
                    }
                    let session_items: Vec<String> = reg
                        .sessions
                        .iter()
                        .map(|s| format!("{} | {} ({})", s.id, s.name, s.host))
                        .collect();

                    let selected = Select::new("Choose session to edit:", session_items)
                        .prompt()
                        .map_err(|e| e.to_string())?;

                    let id = selected.split('|').next().unwrap().trim();
                    let new_host = Text::new("New Hostname (leave empty to keep):")
                        .prompt()
                        .map_err(|e| e.to_string())?;
                    let new_user = Text::new("New Username (leave empty to keep):")
                        .prompt()
                        .map_err(|e| e.to_string())?;

                    service.handle_edit(
                        EditArgs {
                            target: id.to_string(),
                            name: None,
                            group: None,
                            host: if new_host.trim().is_empty() { None } else { Some(new_host) },
                            username: if new_user.trim().is_empty() { None } else { Some(new_user) },
                            port: None,
                            identity_file: None,
                            jump_host: None,
                            description: None,
                            tags: vec![],
                            clear_tags: false,
                        },
                        false,
                    )?;
                }
                c if c.starts_with("5") => {
                    let reg = storage.load_registry()?;
                    if reg.sessions.is_empty() {
                        println!("No sessions available to delete.");
                        continue;
                    }
                    let session_items: Vec<String> = reg
                        .sessions
                        .iter()
                        .map(|s| format!("{} | {} ({})", s.id, s.name, s.host))
                        .collect();

                    let selected = Select::new("Choose session to delete:", session_items)
                        .prompt()
                        .map_err(|e| e.to_string())?;

                    let id = selected.split('|').next().unwrap().trim();
                    let delete_action = Select::new("Confirm Deletion:", vec!["Cancel", "Delete"])
                        .prompt()
                        .map_err(|e| e.to_string())?;

                    if delete_action == "Delete" {
                        service.handle_delete(
                            DeleteArgs {
                                target: id.to_string(),
                                group: None,
                                yes: true,
                            },
                            false,
                        )?;
                    } else {
                        println!("Deletion cancelled.");
                    }
                }
                c if c.starts_with("6") => {
                    let reg = storage.load_registry()?;
                    if reg.sessions.is_empty() {
                        println!("No sessions available to ping.");
                        continue;
                    }
                    let session_items: Vec<String> = reg
                        .sessions
                        .iter()
                        .map(|s| format!("{} | {} ({})", s.id, s.name, s.host))
                        .collect();

                    let selected = Select::new("Choose session to ping:", session_items)
                        .prompt()
                        .map_err(|e| e.to_string())?;

                    let id = selected.split('|').next().unwrap().trim();
                    let _ = service.handle_ping(
                        crate::cli::PingArgs {
                            target: id.to_string(),
                            group: None,
                            icmp: false,
                            timeout: None,
                        },
                        false,
                    );
                }
                _ => break,
            }
            println!();
        }
        Ok(())
    }
}
