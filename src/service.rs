use std::io::{self, Write};
use chrono::Utc;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, ContentArrangement, Table};
use uuid::Uuid;

use crate::cli::*;
use crate::config::IdType;
use crate::connect::{PingService, SshConnection};
use crate::model::{normalize_tags, Registry, Session};
use crate::storage::StorageManager;

pub struct AppService {
    storage: StorageManager,
}

impl AppService {
    pub fn new(storage: StorageManager) -> Self {
        Self { storage }
    }

    pub fn handle_add(&self, args: AddArgs, as_json: bool) -> Result<(), String> {
        let mut host = args.host;
        let mut user = args.username;
        let mut port = args.port;

        if let Some(target) = args.target {
            let (parsed_user, parsed_host, parsed_port) = parse_target_syntax(&target)?;
            if user.is_none() {
                user = parsed_user;
            }
            if host.is_none() {
                host = Some(parsed_host);
            }
            if port.is_none() && parsed_port.is_some() {
                port = parsed_port;
            }
        }

        let host = host.ok_or("Hostname is required (-H or user@host)".to_string())?;
        let user = user.ok_or("Username is required (-u or user@host)".to_string())?;

        let cfg = self.storage.load_config()?;
        let port = port.unwrap_or(cfg.default_port);
        let name = args.name.unwrap_or_else(|| format!("{}@{}", user, host));

        if name.chars().all(|c| c.is_ascii_digit()) {
            return Err("Session name cannot consist entirely of numbers".to_string());
        }

        let mut reg = self.storage.load_registry()?;
        let group_norm = args.group.as_deref().unwrap_or("").trim().to_lowercase();
        let name_norm = name.trim().to_lowercase();

        for s in &reg.sessions {
            let existing_group = s.group.as_deref().unwrap_or("").trim().to_lowercase();
            let existing_name = s.name.trim().to_lowercase();
            if existing_group == group_norm && existing_name == name_norm {
                return Err(format!(
                    "Session with name '{}' already exists in group '{}'",
                    name,
                    args.group.as_deref().unwrap_or("(none)")
                ));
            }
        }

        let id = match cfg.id_type {
            IdType::Integer => {
                let id_str = reg.next_id.to_string();
                reg.next_id += 1;
                id_str
            }
            IdType::Uuid => Uuid::new_v4().to_string(),
        };

        let tags = normalize_tags(&args.tags);
        let now = Utc::now();
        let session = Session {
            id: id.clone(),
            group: args.group,
            name,
            host,
            user,
            port,
            identity_file: args.identity_file,
            jump_host: args.jump_host,
            description: args.description,
            tags,
            created_at: now,
            updated_at: now,
        };

        reg.sessions.push(session.clone());
        self.storage.save_registry(&reg)?;

        if as_json {
            println!("{}", serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?);
        } else {
            println!("Added session '{}' with ID {}", session.name, session.id);
        }
        Ok(())
    }

    pub fn handle_list(&self, args: ListArgs, as_json: bool) -> Result<(), String> {
        let reg = self.storage.load_registry()?;
        let filtered: Vec<&Session> = reg
            .sessions
            .iter()
            .filter(|s| {
                if let Some(ref g) = args.group {
                    let g_norm = g.trim().to_lowercase();
                    let s_group = s.group.as_deref().unwrap_or("").trim().to_lowercase();
                    if s_group != g_norm {
                        return false;
                    }
                }
                if let Some(ref n) = args.name {
                    if !s.name.to_lowercase().contains(&n.trim().to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref h) = args.host {
                    if !s.host.to_lowercase().contains(&h.trim().to_lowercase()) {
                        return false;
                    }
                }
                if let Some(ref u) = args.username {
                    if !s.user.to_lowercase().contains(&u.trim().to_lowercase()) {
                        return false;
                    }
                }
                if let Some(p) = args.port {
                    if s.port != p {
                        return false;
                    }
                }
                if !args.tags.is_empty() {
                    let required_tags = normalize_tags(&args.tags);
                    for req in required_tags {
                        if !s.tags.iter().any(|t| t.eq_ignore_ascii_case(&req)) {
                            return false;
                        }
                    }
                }
                true
            })
            .collect();

        if as_json {
            println!("{}", serde_json::to_string_pretty(&filtered).map_err(|e| e.to_string())?);
            return Ok(());
        }

        if filtered.is_empty() {
            println!("No sessions found.");
            return Ok(());
        }

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic);

        table.set_header(vec![
            Cell::new("ID").fg(Color::Cyan),
            Cell::new("Group").fg(Color::Cyan),
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Host").fg(Color::Cyan),
            Cell::new("User").fg(Color::Cyan),
            Cell::new("Port").fg(Color::Cyan),
            Cell::new("Tags").fg(Color::Cyan),
        ]);

        for s in filtered {
            let display_id = if s.id.len() > 8 && s.id.contains('-') {
                &s.id[..8]
            } else {
                &s.id
            };
            table.add_row(vec![
                Cell::new(display_id),
                Cell::new(s.group.as_deref().unwrap_or("-")),
                Cell::new(&s.name),
                Cell::new(&s.host),
                Cell::new(&s.user),
                Cell::new(s.port),
                Cell::new(s.tags.join(", ")),
            ]);
        }

        println!("{table}");
        Ok(())
    }

    pub fn handle_show(&self, args: ShowArgs, as_json: bool) -> Result<(), String> {
        let reg = self.storage.load_registry()?;
        let session = resolve_session(&reg, &args.target, args.group.as_deref())?;

        if as_json {
            println!("{}", serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?);
            return Ok(());
        }

        println!("ID:          {}", session.id);
        println!("Name:        {}", session.name);
        println!("Group:       {}", session.group.as_deref().unwrap_or("-"));
        println!("Host:        {}", session.host);
        println!("User:        {}", session.user);
        println!("Port:        {}", session.port);
        println!("Identity:    {}", session.identity_file.as_deref().unwrap_or("-"));
        println!("Jump Host:   {}", session.jump_host.as_deref().unwrap_or("-"));
        println!("Description: {}", session.description.as_deref().unwrap_or("-"));
        println!("Tags:        {}", session.tags.join(", "));
        println!("Created At:  {}", session.created_at.to_rfc3339());
        println!("Updated At:  {}", session.updated_at.to_rfc3339());

        Ok(())
    }

    pub fn handle_edit(&self, args: EditArgs, as_json: bool) -> Result<(), String> {
        let mut reg = self.storage.load_registry()?;
        let index = resolve_session_index(&reg, &args.target, args.group.as_deref())?;

        if let Some(ref new_name) = args.name {
            if new_name.chars().all(|c| c.is_ascii_digit()) {
                return Err("Session name cannot consist entirely of numbers".to_string());
            }
            let target_group = args
                .group
                .as_deref()
                .or(reg.sessions[index].group.as_deref())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let new_name_norm = new_name.trim().to_lowercase();

            for (i, s) in reg.sessions.iter().enumerate() {
                if i != index {
                    let s_group = s.group.as_deref().unwrap_or("").trim().to_lowercase();
                    let s_name = s.name.trim().to_lowercase();
                    if s_group == target_group && s_name == new_name_norm {
                        return Err(format!("Session with name '{new_name}' already exists in group"));
                    }
                }
            }
            reg.sessions[index].name = new_name.clone();
        }

        if let Some(new_group) = args.group {
            reg.sessions[index].group = Some(new_group);
        }
        if let Some(new_host) = args.host {
            reg.sessions[index].host = new_host;
        }
        if let Some(new_user) = args.username {
            reg.sessions[index].user = new_user;
        }
        if let Some(new_port) = args.port {
            reg.sessions[index].port = new_port;
        }
        if let Some(new_identity) = args.identity_file {
            reg.sessions[index].identity_file = Some(new_identity);
        }
        if let Some(new_jump) = args.jump_host {
            reg.sessions[index].jump_host = Some(new_jump);
        }
        if let Some(new_desc) = args.description {
            reg.sessions[index].description = Some(new_desc);
        }
        if args.clear_tags {
            reg.sessions[index].tags.clear();
        } else if !args.tags.is_empty() {
            reg.sessions[index].tags = normalize_tags(&args.tags);
        }

        reg.sessions[index].updated_at = Utc::now();
        let updated = reg.sessions[index].clone();
        self.storage.save_registry(&reg)?;

        if as_json {
            println!("{}", serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())?);
        } else {
            println!("Updated session '{}' ({})", updated.name, updated.id);
        }
        Ok(())
    }

    pub fn handle_delete(&self, args: DeleteArgs, as_json: bool) -> Result<(), String> {
        let mut reg = self.storage.load_registry()?;
        let index = resolve_session_index(&reg, &args.target, args.group.as_deref())?;
        let session = reg.sessions[index].clone();

        let cfg = self.storage.load_config()?;
        if cfg.confirm_delete && !args.yes {
            print!(
                "Are you sure you want to delete session '{}' ({})? [y/N]: ",
                session.name, session.id
            );
            io::stdout().flush().map_err(|e| e.to_string())?;

            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
            let trimmed = input.trim();
            if !trimmed.eq_ignore_ascii_case("y") && !trimmed.eq_ignore_ascii_case("yes") {
                println!("Deletion cancelled.");
                return Ok(());
            }
        }

        reg.sessions.remove(index);
        self.storage.save_registry(&reg)?;

        if as_json {
            println!("{}", serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?);
        } else {
            println!("Deleted session '{}' ({})", session.name, session.id);
        }
        Ok(())
    }

    pub fn handle_config(&self, args: ConfigArgs, as_json: bool) -> Result<(), String> {
        let mut cfg = self.storage.load_config()?;
        match (args.key, args.value) {
            (None, None) => {
                if as_json {
                    println!("{}", serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?);
                } else {
                    println!("id_type:              {:?}", cfg.id_type);
                    println!("default_port:         {}", cfg.default_port);
                    println!("confirm_delete:       {}", cfg.confirm_delete);
                    println!("ping_timeout_seconds: {}", cfg.ping_timeout_seconds);
                }
            }
            (Some(k), None) => {
                let key_norm = k.to_lowercase();
                match key_norm.as_str() {
                    "id_type" => println!("{:?}", cfg.id_type),
                    "default_port" => println!("{}", cfg.default_port),
                    "confirm_delete" => println!("{}", cfg.confirm_delete),
                    "ping_timeout_seconds" => println!("{}", cfg.ping_timeout_seconds),
                    other => return Err(format!("Unknown configuration key '{other}'")),
                }
            }
            (Some(k), Some(v)) => {
                let key_norm = k.to_lowercase();
                match key_norm.as_str() {
                    "id_type" => {
                        let v_norm = v.to_lowercase();
                        if v_norm == "uuid" {
                            cfg.id_type = IdType::Uuid;
                        } else if v_norm == "integer" {
                            cfg.id_type = IdType::Integer;
                        } else {
                            return Err("id_type must be 'integer' or 'uuid'".to_string());
                        }
                    }
                    "default_port" => {
                        let port = v.parse::<u16>().map_err(|_| "Port must be a number (1-65535)")?;
                        cfg.default_port = port;
                    }
                    "confirm_delete" => {
                        let b = parse_bool(&v)?;
                        cfg.confirm_delete = b;
                    }
                    "ping_timeout_seconds" => {
                        let t = v.parse::<u64>().map_err(|_| "Timeout must be a positive integer")?;
                        cfg.ping_timeout_seconds = t;
                    }
                    other => return Err(format!("Unknown configuration key '{other}'")),
                }
                self.storage.save_config(&cfg)?;
                if as_json {
                    println!("{}", serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?);
                } else {
                    println!("Configuration updated: {k} = {v}");
                }
            }
            (None, Some(_)) => {
                return Err("Cannot provide a value without specifying a configuration key".to_string());
            }
        }
        Ok(())
    }

    pub fn handle_connect(&self, args: ConnectArgs) -> Result<(), String> {
        let reg = self.storage.load_registry()?;
        let session = resolve_session(&reg, &args.target, args.group.as_deref())?;

        if args.dry_run {
            let cmd_args = SshConnection::build_args(session, &args.passthrough);
            println!("ssh {}", cmd_args.join(" "));
            return Ok(());
        }

        let exit_code = SshConnection::execute(session, &args.passthrough)?;
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
        Ok(())
    }

    pub fn handle_ping(&self, args: PingArgs, as_json: bool) -> Result<(), String> {
        let reg = self.storage.load_registry()?;
        let session = resolve_session(&reg, &args.target, args.group.as_deref())?;

        let cfg = self.storage.load_config()?;
        let timeout_secs = args.timeout.unwrap_or(cfg.ping_timeout_seconds);

        let result = if args.icmp {
            PingService::ping_icmp(&session.host, timeout_secs)
        } else {
            use std::time::Duration;
            PingService::ping_tcp(&session.host, session.port, Duration::from_secs(timeout_secs))
        };

        if as_json {
            println!("{}", serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?);
        } else {
            let status = if result.success { "SUCCESS" } else { "FAILED" };
            println!(
                "Ping [{}] to {} (type: {}): {}",
                status, result.target, result.ping_type, result.message
            );
        }

        if !result.success {
            std::process::exit(1);
        }
        Ok(())
    }

    pub fn handle_export(&self, args: ExportArgs) -> Result<(), String> {
        let reg = self.storage.load_registry()?;
        if let Some(path_str) = args.output {
            let file = std::fs::File::create(&path_str).map_err(|e| e.to_string())?;
            crate::impexp::SshConfigManager::export_to_writer(&reg, file)?;
            println!("Exported {} sessions to {}", reg.sessions.len(), path_str);
        } else {
            let stdout = std::io::stdout();
            crate::impexp::SshConfigManager::export_to_writer(&reg, stdout.lock())?;
        }
        Ok(())
    }

    pub fn handle_import(&self, args: ImportArgs) -> Result<(), String> {
        let cfg = self.storage.load_config()?;
        let mut reg = self.storage.load_registry()?;
        let count = crate::impexp::SshConfigManager::import_from_file(&args.file, &mut reg, cfg.id_type)?;
        self.storage.save_registry(&reg)?;
        println!("Successfully imported {} sessions from {}", count, args.file);
        Ok(())
    }

    pub fn handle_completions(&self, args: CompletionsArgs) -> Result<(), String> {
        use clap::CommandFactory;
        let mut cmd = crate::cli::Cli::command();
        let bin_name = cmd.get_name().to_string();
        clap_complete::generate(args.shell, &mut cmd, bin_name, &mut std::io::stdout());
        Ok(())
    }
}

pub fn resolve_session<'a>(
    reg: &'a Registry,
    target: &str,
    group: Option<&str>,
) -> Result<&'a Session, String> {
    let index = resolve_session_index(reg, target, group)?;
    Ok(&reg.sessions[index])
}

pub fn resolve_session_index(
    reg: &Registry,
    target: &str,
    group: Option<&str>,
) -> Result<usize, String> {
    let target_trim = target.trim();
    let group_norm = group.map(|g| g.trim().to_lowercase());

    let mut matches = Vec::new();
    for (idx, s) in reg.sessions.iter().enumerate() {
        if let Some(ref g) = group_norm {
            let s_group = s.group.as_deref().unwrap_or("").trim().to_lowercase();
            if s_group != *g {
                continue;
            }
        }

        if s.id.eq_ignore_ascii_case(target_trim) {
            matches.push(idx);
            continue;
        }

        if s.id.len() > 8 && s.id.to_lowercase().starts_with(&target_trim.to_lowercase()) {
            matches.push(idx);
            continue;
        }

        if s.name.eq_ignore_ascii_case(target_trim) {
            matches.push(idx);
            continue;
        }
    }

    match matches.len() {
        0 => Err(format!("Session '{target}' not found")),
        1 => Ok(matches[0]),
        _ => Err(format!(
            "Ambiguous target '{target}'. Found {} matching sessions. Specify ID or group flag (-g).",
            matches.len()
        )),
    }
}

fn parse_target_syntax(target: &str) -> Result<(Option<String>, String, Option<u16>), String> {
    let mut user = None;
    let mut host = target;

    if let Some((u, h)) = target.split_once('@') {
        user = Some(u.to_string());
        host = h;
    }

    let mut port = None;
    if let Some((h, p)) = host.split_once(':') {
        host = h;
        let parsed_port = p.parse::<u16>().map_err(|_| format!("Invalid port '{p}'"))?;
        port = Some(parsed_port);
    }

    Ok((user, host.to_string(), port))
}

fn parse_bool(val: &str) -> Result<bool, String> {
    match val.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        other => Err(format!("Invalid boolean '{other}', expected true/false/1/0")),
    }
}
