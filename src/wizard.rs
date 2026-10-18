use crate::{
    cli::*,
    model::{Session, TunnelKind},
    service::AppService,
    storage::StorageManager,
};
use inquire::{InquireError, Select, Text};
use std::fmt;

fn optional<T>(result: Result<T, InquireError>) -> Result<Option<T>, String> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(_) => Err("Interactive terminal input failed".into()),
    }
}
#[derive(Clone)]
struct SessionChoice(Session);
impl fmt::Display for SessionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} | {}/{} | {}@{}:{} | last {}",
            if self.0.favorite { "*" } else { " " },
            self.0.id,
            self.0.group.as_deref().unwrap_or("-"),
            self.0.name,
            self.0.user,
            self.0.host,
            self.0.port,
            self.0
                .last_connect_attempt_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| "never".into())
        )
    }
}
pub fn pick_session(sessions: Vec<Session>) -> Result<Option<String>, String> {
    if sessions.is_empty() {
        println!("No matching sessions.");
        return Ok(None);
    }
    Ok(optional(
        Select::new(
            "Choose session (type to filter, Esc to cancel):",
            sessions.into_iter().map(SessionChoice).collect(),
        )
        .with_scorer(&|input, choice, _, index| {
            let s = &choice.0;
            let fields = [
                s.name.as_str(),
                s.host.as_str(),
                s.user.as_str(),
                s.group.as_deref().unwrap_or(""),
            ];
            fields
                .into_iter()
                .chain(s.tags.iter().map(String::as_str))
                .filter_map(|value| crate::selection::fuzzy_score(value, input))
                .min()
                .map(|score| {
                    -(score as i64)
                        .saturating_mul(10000)
                        .saturating_add(index as i64)
                })
        })
        .prompt(),
    )?
    .map(|s| s.0.id))
}
fn select<'a>(message: &str, items: Vec<&'a str>) -> Result<Option<&'a str>, String> {
    optional(Select::new(message, items).prompt())
}
fn text(message: &str, initial: &str) -> Result<Option<String>, String> {
    optional(Text::new(message).with_initial_value(initial).prompt())
}
fn field(message: &str, initial: &str) -> Result<String, String> {
    text(message, initial)?.ok_or("Cancelled".into())
}
pub fn deletion(message: &str) -> Result<bool, String> {
    Ok(select(message, vec!["Cancel", "Delete"])? == Some("Delete"))
}

pub fn onboard(storage: &StorageManager) -> Result<(), String> {
    if storage.config_path().exists() {
        return Ok(());
    }
    let Some(choice) = select(
        "Session IDs:",
        vec!["Incremental Integer (default)", "UUID v4"],
    )?
    else {
        return Ok(());
    };
    storage.update_config(|config| {
        config.id_type = if choice.starts_with("UUID") {
            crate::config::IdType::Uuid
        } else {
            crate::config::IdType::Integer
        };
        Ok(())
    })?;
    storage.update_registry(|_| Ok(()))?;
    let report = crate::doctor::DoctorService::run(storage);
    if !report.ssh_available {
        println!("OpenSSH client not found. Install it before connecting; profiles remain usable.");
    }
    Ok(())
}
pub fn menu(service: &AppService) -> Result<(), String> {
    match select("Clisshman", vec!["CLI", "TUI", "Help", "Exit"])? {
        Some("CLI") => Wizard::start(service, &service.storage),
        Some("TUI") => crate::tui::TuiApp::new(service.clone(), service.storage.clone())?.run(),
        Some("Help") => {
            use clap::CommandFactory;
            Cli::command()
                .print_help()
                .map_err(|_| "Cannot print help")?;
            println!();
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn session_form(
    service: &AppService,
    source: Option<&Session>,
    duplicate: bool,
) -> Result<(), String> {
    let config = service.storage.load_config()?;
    let get = |f: fn(&Session) -> &str| source.map(f).unwrap_or("");
    let name = field(
        "Name (empty generates user@host for new sessions):",
        if duplicate { "" } else { get(|s| &s.name) },
    )?;
    if duplicate && name.trim().is_empty() {
        return Err("Duplicate requires a new name".into());
    }
    let host = field("Hostname / IP:", get(|s| &s.host))?;
    let user = field("Username:", get(|s| &s.user))?;
    let port = field(
        "Port:",
        &source
            .map(|s| s.port)
            .unwrap_or(config.default_port)
            .to_string(),
    )?
    .parse()
    .map_err(|_| "Invalid port")?;
    let group = field(
        "Group (empty clears):",
        source.and_then(|s| s.group.as_deref()).unwrap_or(""),
    )?;
    let identity = field(
        "Identity file (empty clears):",
        source
            .and_then(|s| s.identity_file.as_deref())
            .unwrap_or(""),
    )?;
    let jump = field(
        "Jump host (empty clears):",
        source.and_then(|s| s.jump_host.as_deref()).unwrap_or(""),
    )?;
    let description = field(
        "Description (empty clears):",
        source.and_then(|s| s.description.as_deref()).unwrap_or(""),
    )?;
    let tags = field(
        "Tags (comma-separated, empty clears):",
        &source.map(|s| s.tags.join(", ")).unwrap_or_default(),
    )?;
    let option = |s: String| if s.trim().is_empty() { None } else { Some(s) };
    if let Some(source) = source.filter(|_| !duplicate) {
        service.edit(EditArgs {
            target: source.id.clone(),
            name: Some(name),
            host: Some(host),
            username: Some(user),
            port: Some(port),
            clear_group: group.trim().is_empty(),
            group: option(group),
            clear_identity: identity.trim().is_empty(),
            identity_file: option(identity),
            clear_jump: jump.trim().is_empty(),
            jump_host: option(jump),
            clear_description: description.trim().is_empty(),
            description: option(description),
            clear_tags: tags.trim().is_empty(),
            tags: if tags.trim().is_empty() {
                vec![]
            } else {
                vec![tags]
            },
            ..Default::default()
        })?;
    } else {
        service.add(AddArgs {
            name: option(name),
            host: Some(host),
            username: Some(user),
            port: Some(port),
            group: option(group),
            identity_file: option(identity),
            jump_host: option(jump),
            description: option(description),
            tags: if tags.trim().is_empty() {
                vec![]
            } else {
                vec![tags]
            },
            ..Default::default()
        })?;
    }
    Ok(())
}

pub fn tunnel_form(
    service: &AppService,
    existing: Option<&crate::model::Tunnel>,
) -> Result<(), String> {
    let name = field(
        "Tunnel name:",
        existing.map(|t| t.name.as_str()).unwrap_or(""),
    )?;
    let Some(session) = pick_session(crate::selection::ranked(
        service.list(&ListArgs::default())?,
        "",
    ))?
    else {
        return Ok(());
    };
    let Some(kind) = select(
        "Forwarding kind:",
        vec!["Local", "Remote", "Dynamic (SOCKS)"],
    )?
    else {
        return Ok(());
    };
    let kind = match kind {
        "Local" => TunnelKind::Local,
        "Remote" => TunnelKind::Remote,
        _ => TunnelKind::Dynamic,
    };
    let bind = field(
        "Bind IP (non-loopback exposes the listener to the network):",
        existing.map(|t| t.bind.as_str()).unwrap_or("127.0.0.1"),
    )?;
    let port = field(
        "Listen port:",
        &existing
            .map(|t| t.listen_port.to_string())
            .unwrap_or_default(),
    )?
    .parse()
    .map_err(|_| "Invalid listen port")?;
    let (destination_host, destination_port) = if kind == TunnelKind::Dynamic {
        (None, None)
    } else {
        (
            Some(field(
                "Destination host:",
                existing
                    .and_then(|t| t.destination_host.as_deref())
                    .unwrap_or("localhost"),
            )?),
            Some(
                field(
                    "Destination port:",
                    &existing
                        .and_then(|t| t.destination_port)
                        .map(|p| p.to_string())
                        .unwrap_or_default(),
                )?
                .parse()
                .map_err(|_| "Invalid destination port")?,
            ),
        )
    };
    let loopback = bind
        .parse::<std::net::IpAddr>()
        .map_err(|_| "Invalid bind IP")?
        .is_loopback();
    let allowed = if loopback {
        false
    } else {
        select(
            "This listener may accept network connections. Allow this binding?",
            vec!["Cancel", "Allow public bind"],
        )? == Some("Allow public bind")
    };
    if !loopback && !allowed {
        return Ok(());
    }
    if let Some(existing) = existing {
        service.edit_tunnel(TunnelEditArgs {
            target: existing.id.clone(),
            name: Some(name),
            session: Some(session),
            kind: Some(kind),
            bind: Some(bind),
            listen_port: Some(port),
            destination_host,
            destination_port,
            allow_public_bind: allowed,
            ..Default::default()
        })?;
    } else {
        service.add_tunnel(TunnelAddArgs {
            name,
            session,
            group: None,
            kind,
            bind,
            listen_port: port,
            destination_host,
            destination_port,
            allow_public_bind: allowed,
        })?;
    }
    Ok(())
}

pub fn tunnels(service: &AppService) -> Result<(), String> {
    loop {
        let Some(action) = select(
            "Tunnels",
            vec!["List", "Add", "Show", "Edit", "Delete", "Run", "Back"],
        )?
        else {
            return Ok(());
        };
        if action == "Back" {
            return Ok(());
        }
        let result = (|| {
            if action == "Add" {
                return tunnel_form(service, None);
            }
            let tunnels = service.storage.load_registry()?.tunnels;
            if action == "List" {
                return crate::output::json(&tunnels);
            }
            if tunnels.is_empty() {
                println!("No tunnels.");
                return Ok(());
            }
            let labels: Vec<_> = tunnels
                .iter()
                .map(|t| {
                    format!(
                        "{} | {} | {:?} {}:{}",
                        t.id, t.name, t.kind, t.bind, t.listen_port
                    )
                })
                .collect();
            let Some(selected) = optional(Select::new("Choose tunnel:", labels.clone()).prompt())?
            else {
                return Ok(());
            };
            let index = labels
                .iter()
                .position(|label| label == &selected)
                .ok_or("Tunnel selection changed")?;
            let tunnel = &tunnels[index];
            match action {
                "Show" => crate::output::json(tunnel)?,
                "Edit" => tunnel_form(service, Some(tunnel))?,
                "Delete" => {
                    if deletion("Delete tunnel?")? {
                        service.delete_tunnel(&tunnel.id)?;
                    }
                }
                "Run" => {
                    println!(
                        "Running tunnel {} on {}:{}, Ctrl+C stops it.",
                        tunnel.name, tunnel.bind, tunnel.listen_port
                    );
                    let code = service.run_tunnel_returning(&tunnel.id)?;
                    println!("SSH exited with status {code}");
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = result {
            println!("{error}");
        }
    }
}

pub struct Wizard;
impl Wizard {
    pub fn start(service: &AppService, storage: &StorageManager) -> Result<(), String> {
        onboard(storage)?;
        loop {
            let Some(action) = select(
                "Clisshman",
                vec![
                    "Quick connect",
                    "List",
                    "Show",
                    "Add",
                    "Edit",
                    "Duplicate",
                    "Favorite / Unfavorite",
                    "Delete",
                    "Ping",
                    "Tunnels",
                    "Exit",
                ],
            )?
            else {
                return Ok(());
            };
            if action == "Exit" {
                return Ok(());
            }
            let result = (|| {
                match action {
                    "Add" => return session_form(service, None, false),
                    "List" => {
                        return crate::output::sessions(&service.list(&ListArgs::default())?, false)
                    }
                    "Tunnels" => return tunnels(service),
                    _ => {}
                }
                let sessions = crate::selection::ranked(service.list(&ListArgs::default())?, "");
                let Some(id) = pick_session(sessions)? else {
                    return Ok(());
                };
                let session = service.get(&id, None)?;
                match action {
                    "Quick connect" => {
                        println!("SSH exited with status {}", service.connect_returning(&id)?);
                    }
                    "Show" => crate::output::session(&session, false)?,
                    "Edit" => session_form(service, Some(&session), false)?,
                    "Duplicate" => session_form(service, Some(&session), true)?,
                    "Favorite / Unfavorite" => {
                        service.favorite(&id, None, !session.favorite)?;
                    }
                    "Delete" => {
                        if deletion("Delete session?")? {
                            service.delete(&id, None)?;
                        }
                    }
                    "Ping" => {
                        let result = service.ping(&id, false, None)?;
                        println!("{}", result.message);
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(error) = result {
                println!("{error}");
            }
        }
    }
}
