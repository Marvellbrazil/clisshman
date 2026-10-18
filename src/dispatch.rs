use crate::{
    cli::*,
    model::{generate_id, Registry},
    output, selection,
    service::AppService,
    storage::StorageManager,
    validation,
};
use clap::CommandFactory;
use std::{
    io::{self, IsTerminal, Write},
    path::Path,
};

pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn confirm(message: &str, yes: bool, as_json: bool) -> Result<bool, String> {
    if yes {
        return Ok(true);
    }
    if !interactive() || as_json {
        return Err("Confirmation requires a terminal; pass --yes explicitly".into());
    }
    print!("{message} [y/N]: ");
    io::stdout()
        .flush()
        .map_err(|_| "Cannot display confirmation")?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .map_err(|_| "Cannot read confirmation")?;
    Ok(value.trim().eq_ignore_ascii_case("y") || value.trim().eq_ignore_ascii_case("yes"))
}

pub fn target(
    service: &AppService,
    value: Option<&str>,
    group: Option<&str>,
    as_json: bool,
) -> Result<Option<String>, String> {
    let registry = service.storage.load_registry()?;
    let candidates = if let Some(value) = value {
        selection::candidates(&registry, value, group)
    } else {
        selection::ranked(
            selection::filtered(
                &registry,
                &ListArgs {
                    group: group.map(str::to_owned),
                    ..Default::default()
                },
            ),
            "",
        )
    };
    if candidates.is_empty() {
        return Err("No matching sessions".into());
    }
    if value.is_some() && candidates.len() == 1 {
        return Ok(Some(candidates[0].id.clone()));
    }
    if !interactive() || as_json {
        return Err(format!(
            "Specify an unambiguous target ID. Candidates: {}",
            candidates
                .iter()
                .map(|s| format!("{} ({})", s.id, s.name))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    crate::wizard::pick_session(candidates)
}

pub fn run(cli: Cli, service: &AppService) -> Result<i32, String> {
    let json = cli.json;
    match cli.command {
        Some(Commands::Add(args)) => output::session(&service.add(args)?, json)?,
        Some(Commands::Edit(mut args)) => {
            let Some(id) = target(
                service,
                Some(&args.target),
                args.from_group.as_deref(),
                json,
            )?
            else {
                return Ok(0);
            };
            args.target = id;
            args.from_group = None;
            output::session(&service.edit(args)?, json)?;
        }
        Some(Commands::Duplicate(mut args)) => {
            let Some(id) = target(
                service,
                Some(&args.target),
                args.from_group.as_deref(),
                json,
            )?
            else {
                return Ok(0);
            };
            args.target = id;
            args.from_group = None;
            output::session(&service.duplicate(args)?, json)?;
        }
        Some(Commands::List(args)) => output::sessions(&service.list(&args)?, json)?,
        Some(Commands::Show(args)) => {
            let candidates = if let Some(value) = args.target {
                let Some(id) = target(service, Some(&value), args.filters.group.as_deref(), json)?
                else {
                    return Ok(0);
                };
                let s = service.get(&id, None)?;
                let mut registry = Registry::default();
                registry.sessions.push(s);
                selection::filtered(&registry, &args.filters)
            } else {
                service.list(&args.filters)?
            };
            let selected = if candidates.len() == 1 {
                Some(candidates[0].id.clone())
            } else if interactive() && !json {
                crate::wizard::pick_session(candidates)?
            } else {
                return Err("show requires exactly one match; use list or an exact ID".into());
            };
            if let Some(id) = selected {
                output::session(&service.get(&id, None)?, json)?;
            }
        }
        Some(Commands::Delete(args)) => {
            let Some(id) = target(service, Some(&args.target), args.group.as_deref(), json)? else {
                return Ok(0);
            };
            let config = service.storage.load_config()?;
            let yes = args.yes || (interactive() && !json && !config.confirm_delete);
            if confirm(
                &format!("Delete session '{}' ?", service.get(&id, None)?.name),
                yes,
                json,
            )? {
                output::session(&service.delete(&id, None)?, json)?;
            }
        }
        Some(Commands::Favorite(args)) => {
            if let Some(id) = target(service, Some(&args.target), args.group.as_deref(), json)? {
                output::session(&service.favorite(&id, None, !args.unset)?, json)?;
            }
        }
        Some(Commands::Quick(args)) => {
            if !interactive() || json {
                return Err("quick requires an interactive terminal".into());
            }
            let sessions = selection::ranked(
                service.list(&args.filters)?,
                args.query.as_deref().unwrap_or(""),
            );
            if let Some(id) = crate::wizard::pick_session(sessions)? {
                return service.connect_returning(&id);
            }
        }
        Some(Commands::Connect(args)) => {
            if args.target.is_none() && args.dry_run {
                return Err("--dry-run requires a target".into());
            }
            let Some(id) = target(service, args.target.as_deref(), args.group.as_deref(), json)?
            else {
                return Ok(0);
            };
            if args.dry_run {
                output::command(
                    &crate::connect::SshConnection::build_args(
                        &service.get(&id, None)?,
                        &args.passthrough,
                    ),
                    json,
                )?;
            } else {
                if json {
                    return Err("Use --dry-run with --json for connect".into());
                }
                let session = service.record_attempt(&id)?;
                return crate::connect::SshConnection::execute(&session, &args.passthrough);
            }
        }
        Some(Commands::Ping(args)) => {
            if let Some(id) = target(service, Some(&args.target), args.group.as_deref(), json)? {
                let result = service.ping(&id, args.icmp, args.timeout)?;
                if json {
                    output::json(&result)?;
                } else {
                    println!("{}: {}", result.target, result.message);
                }
                return Ok(if result.success { 0 } else { 1 });
            }
        }
        Some(Commands::Config(args)) => {
            let config = match (args.key.as_deref(), args.value.as_deref()) {
                (Some(key), Some(value)) => service.set_config(key, value)?,
                (None, Some(_)) => return Err("Config key is required".into()),
                _ => service.storage.load_config()?,
            };
            if let (Some(key), None) = (args.key, args.value) {
                let value = serde_json::to_value(&config).map_err(|_| "Cannot format config")?;
                output::json(value.get(key.to_lowercase()).ok_or("Unknown config key")?)?;
            } else {
                output::json(&config)?;
            }
        }
        Some(Commands::Tunnel(command)) => return tunnels(command, service, json),
        Some(Commands::Import(args)) => import(args, service, json)?,
        Some(Commands::Export(args)) => export(args, service, json)?,
        Some(Commands::Completions(args)) => {
            let mut command = Cli::command();
            clap_complete::generate(args.shell, &mut command, "clisshman", &mut io::stdout());
        }
        Some(Commands::Doctor) => {
            let report = crate::doctor::DoctorService::run(&service.storage);
            if json {
                output::json(
                    &serde_json::json!({"ssh_available":report.ssh_available,"ssh_version":report.ssh_version,"config_path":report.config_dir_path,"storage_readable":report.config_dir_writable}),
                )?;
            } else {
                crate::doctor::DoctorService::print_report(&report);
            }
            return Ok(if report.ssh_available && report.config_dir_writable {
                0
            } else {
                1
            });
        }
        Some(Commands::Cli) => {
            require_terminal(json)?;
            crate::wizard::Wizard::start(service, &service.storage)?;
        }
        Some(Commands::Tui) => {
            require_terminal(json)?;
            crate::wizard::onboard(&service.storage)?;
            crate::tui::TuiApp::new(service.clone(), service.storage.clone())?.run()?;
        }
        None => {
            if !interactive() || json {
                Cli::command()
                    .print_help()
                    .map_err(|_| "Cannot print help")?;
                println!();
                return Ok(1);
            }
            crate::wizard::onboard(&service.storage)?;
            crate::wizard::menu(service)?;
        }
    }
    Ok(0)
}
fn require_terminal(json: bool) -> Result<(), String> {
    if interactive() && !json {
        Ok(())
    } else {
        Err("This mode requires an interactive terminal without --json".into())
    }
}

fn tunnels(command: TunnelCommands, service: &AppService, json: bool) -> Result<i32, String> {
    match command {
        TunnelCommands::Add(args) => output::json(&service.add_tunnel(args)?)?,
        TunnelCommands::List => output::json(&service.storage.load_registry()?.tunnels)?,
        TunnelCommands::Show(args) => output::json(&service.get_tunnel(&args.target)?)?,
        TunnelCommands::Edit(args) => output::json(&service.edit_tunnel(args)?)?,
        TunnelCommands::Delete(args) => {
            if confirm("Delete tunnel?", args.yes, json)? {
                output::json(&service.delete_tunnel(&args.target)?)?;
            }
        }
        TunnelCommands::Run(args) => {
            let tunnel = service.get_tunnel(&args.target)?;
            let extra = crate::service::tunnel_args(&tunnel)?;
            let session = service.get(&tunnel.session_id, None)?;
            if args.dry_run {
                output::command(
                    &crate::connect::SshConnection::build_args(&session, &extra),
                    json,
                )?;
            } else {
                if json {
                    return Err("Use --dry-run with --json for tunnel run".into());
                }
                eprintln!(
                    "Tunnel '{}' binds {}:{} ({:?}); Ctrl+C stops it.",
                    tunnel.name, tunnel.bind, tunnel.listen_port, tunnel.kind
                );
                let session = service.record_attempt(&session.id)?;
                return crate::connect::SshConnection::execute(&session, &extra);
            }
        }
    }
    Ok(0)
}

fn export(args: ExportArgs, service: &AppService, json: bool) -> Result<(), String> {
    let registry = service.storage.load_registry()?;
    let mut bytes = Vec::new();
    match args.format {
        TransferFormat::Json => {
            bytes = serde_json::to_vec_pretty(&registry).map_err(|_| "Cannot serialize backup")?
        }
        TransferFormat::Ssh => {
            if json {
                return Err("Use export --format json rather than --json for backups".into());
            }
            crate::impexp::SshConfigManager::export_to_writer(&registry, &mut bytes)?;
        }
    }
    if let Some(path) = args.output {
        let path = Path::new(&path);
        let exists = path.exists();
        if path == service.storage.registry_path()
            || path == service.storage.config_path()
            || path == service.storage.lock_path()
        {
            return Err("Export destination must not be an application storage file".into());
        }
        if exists && !confirm("Replace existing export file?", args.yes, json)? {
            return Ok(());
        }
        StorageManager::atomic_write(path, &bytes, exists)?;
    } else {
        io::stdout()
            .write_all(&bytes)
            .map_err(|_| "Cannot write export")?;
        println!();
    }
    Ok(())
}

fn import(args: ImportArgs, service: &AppService, json: bool) -> Result<(), String> {
    let mut incoming = match args.format {
        TransferFormat::Ssh => Registry {
            sessions: crate::impexp::SshConfigManager::parse_from_file(&args.file)?,
            ..Default::default()
        },
        TransferFormat::Json => {
            let data = std::fs::read(&args.file).map_err(|_| "Cannot read backup")?;
            let mut registry: Registry =
                serde_json::from_slice(&data).map_err(|_| "Invalid JSON backup")?;
            if registry.version == 0 || registry.version > 2 {
                return Err("Unsupported backup version".into());
            }
            if registry.version == 1 {
                let highest = registry
                    .sessions
                    .iter()
                    .filter_map(|s| s.id.parse::<u64>().ok())
                    .max()
                    .unwrap_or(0);
                registry.next_id = registry
                    .next_id
                    .max(highest.checked_add(1).ok_or("ID counter exhausted")?);
            }
            validation::registry(&mut registry)?;
            registry
        }
    };
    for session in &mut incoming.sessions {
        validation::session(session)?;
    }
    for (i, s) in incoming.sessions.iter().enumerate() {
        if incoming.sessions[..i]
            .iter()
            .any(|other| validation::same_profile(s, other))
        {
            return Err("Duplicate profiles in import".into());
        }
    }
    let previous = service.storage.load_registry()?;
    if incoming.tunnels.iter().any(|t| {
        previous
            .tunnels
            .iter()
            .any(|old| old.name.eq_ignore_ascii_case(&t.name))
    }) {
        return Err("Tunnel name conflict; rename before import".into());
    }
    let conflicts: Vec<_> = incoming
        .sessions
        .iter()
        .filter(|s| {
            previous
                .sessions
                .iter()
                .any(|old| validation::same_profile(old, s))
        })
        .map(|s| s.name.clone())
        .collect();
    output::json(
        &serde_json::json!({"sessions":incoming.sessions.len(),"tunnels":incoming.tunnels.len(),"conflicts":conflicts,"dry_run":args.dry_run}),
    )?;
    if args.dry_run {
        return Ok(());
    }
    if !conflicts.is_empty()
        && !confirm(
            "Overwrite conflicting profiles (IDs retained)?",
            args.yes,
            json,
        )?
    {
        return Ok(());
    }
    let config = service.storage.load_config()?;
    service.storage.update_registry(|current| {
        if serde_json::to_value(&previous).ok() != serde_json::to_value(&*current).ok() {
            return Err("Registry changed during import preview; retry".into());
        }
        let mut remap = std::collections::HashMap::new();
        for mut s in incoming.sessions {
            let source_id = s.id.clone();
            if let Some(old) = current
                .sessions
                .iter_mut()
                .find(|old| validation::same_profile(old, &s))
            {
                s.id = old.id.clone();
                s.created_at = old.created_at;
                s.favorite = old.favorite;
                s.last_connect_attempt_at = old.last_connect_attempt_at;
                s.updated_at = chrono::Utc::now();
                remap.insert(source_id, s.id.clone());
                *old = s;
            } else {
                s.id = generate_id(config.id_type, &mut current.next_id)?;
                remap.insert(source_id, s.id.clone());
                current.sessions.push(s);
            }
        }
        for mut tunnel in incoming.tunnels {
            tunnel.session_id = remap
                .get(&tunnel.session_id)
                .ok_or("Tunnel source session missing")?
                .clone();
            if current
                .tunnels
                .iter()
                .any(|t| t.name.eq_ignore_ascii_case(&tunnel.name))
            {
                return Err("Tunnel name conflict; rename before import".into());
            }
            tunnel.id = generate_id(config.id_type, &mut current.next_tunnel_id)?;
            current.tunnels.push(tunnel);
        }
        Ok(())
    })
}
