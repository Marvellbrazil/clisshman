use super::confirm;
use crate::{
    cli::{ExportArgs, ImportArgs, TransferFormat},
    model::{generate_id, Registry},
    output,
    service::AppService,
    storage::StorageManager,
    validation,
};
use std::{
    io::{self, Write},
    path::Path,
};
pub(super) fn export(args: ExportArgs, service: &AppService, json: bool) -> Result<(), String> {
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

pub(super) fn import(args: ImportArgs, service: &AppService, json: bool) -> Result<(), String> {
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
