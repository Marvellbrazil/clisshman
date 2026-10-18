pub use crate::selection::{resolve_session, resolve_session_index};
use crate::{
    cli::*,
    config::IdType,
    model::{generate_id, normalize_tags, Session, Tunnel, TunnelKind},
    storage::StorageManager,
    validation,
};
use chrono::Utc;

#[derive(Clone)]
pub struct AppService {
    pub storage: StorageManager,
}

impl AppService {
    pub fn new(storage: StorageManager) -> Self {
        Self { storage }
    }
    pub fn list(&self, filters: &ListArgs) -> Result<Vec<Session>, String> {
        Ok(crate::selection::filtered(
            &self.storage.load_registry()?,
            filters,
        ))
    }
    pub fn get(&self, target: &str, group: Option<&str>) -> Result<Session, String> {
        Ok(resolve_session(&self.storage.load_registry()?, target, group)?.clone())
    }
    pub fn add(&self, args: AddArgs) -> Result<Session, String> {
        let config = self.storage.load_config()?;
        let parsed = args
            .target
            .as_deref()
            .map(validation::parse_target)
            .transpose()?;
        let user = args
            .username
            .or_else(|| parsed.as_ref().and_then(|v| v.0.clone()))
            .ok_or("Username is required (-u or user@host)")?;
        let host = args
            .host
            .or_else(|| parsed.as_ref().map(|v| v.1.clone()))
            .ok_or("Hostname is required (-H or user@host)")?;
        let port = args
            .port
            .or_else(|| parsed.as_ref().and_then(|v| v.2))
            .unwrap_or(config.default_port);
        check_tags(&args.tags)?;
        let now = Utc::now();
        let mut session = Session {
            id: String::new(),
            name: args.name.unwrap_or_else(|| format!("{user}@{host}")),
            group: args.group,
            host,
            user,
            port,
            identity_file: args.identity_file,
            jump_host: args.jump_host,
            description: args.description,
            tags: normalize_tags(&args.tags),
            favorite: false,
            last_connect_attempt_at: None,
            created_at: now,
            updated_at: now,
        };
        validation::session(&mut session)?;
        self.storage.update_registry(|registry| {
            session.id = generate_id(config.id_type, &mut registry.next_id)?;
            registry.sessions.push(session.clone());
            Ok(session)
        })
    }
    pub fn edit(&self, args: EditArgs) -> Result<Session, String> {
        check_tags(&args.tags)?;
        if (args.clear_group && args.group.is_some())
            || (args.clear_identity && args.identity_file.is_some())
            || (args.clear_jump && args.jump_host.is_some())
            || (args.clear_description && args.description.is_some())
            || (args.clear_tags && !args.tags.is_empty())
        {
            return Err("Cannot set and clear the same field".into());
        }
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, &args.target, args.from_group.as_deref())?;
            let s = &mut registry.sessions[index];
            if let Some(value) = args.name {
                s.name = value;
            }
            if let Some(value) = args.group {
                s.group = Some(value);
            }
            if let Some(value) = args.host {
                s.host = value;
            }
            if let Some(value) = args.username {
                s.user = value;
            }
            if let Some(value) = args.port {
                s.port = value;
            }
            if let Some(value) = args.identity_file {
                s.identity_file = Some(value);
            }
            if let Some(value) = args.jump_host {
                s.jump_host = Some(value);
            }
            if let Some(value) = args.description {
                s.description = Some(value);
            }
            if !args.tags.is_empty() {
                s.tags = normalize_tags(&args.tags);
            }
            if args.clear_tags {
                s.tags.clear();
            }
            if args.clear_group {
                s.group = None;
            }
            if args.clear_identity {
                s.identity_file = None;
            }
            if args.clear_jump {
                s.jump_host = None;
            }
            if args.clear_description {
                s.description = None;
            }
            s.updated_at = Utc::now();
            validation::session(s)?;
            Ok(s.clone())
        })
    }
    pub fn duplicate(&self, args: DuplicateArgs) -> Result<Session, String> {
        let config = self.storage.load_config()?;
        self.storage.update_registry(|registry| {
            let mut copy =
                resolve_session(registry, &args.target, args.from_group.as_deref())?.clone();
            copy.name = args.name;
            if let Some(value) = args.group {
                copy.group = Some(value);
            }
            if let Some(value) = args.host {
                copy.host = value;
            }
            if let Some(value) = args.username {
                copy.user = value;
            }
            if let Some(value) = args.port {
                copy.port = value;
            }
            copy.id = generate_id(config.id_type, &mut registry.next_id)?;
            copy.created_at = Utc::now();
            copy.updated_at = copy.created_at;
            copy.favorite = false;
            copy.last_connect_attempt_at = None;
            validation::session(&mut copy)?;
            registry.sessions.push(copy.clone());
            Ok(copy)
        })
    }
    pub fn delete(&self, target: &str, group: Option<&str>) -> Result<Session, String> {
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, target, group)?;
            let id = &registry.sessions[index].id;
            let dependencies: Vec<_> = registry
                .tunnels
                .iter()
                .filter(|t| &t.session_id == id)
                .map(|t| t.name.as_str())
                .collect();
            if !dependencies.is_empty() {
                return Err(format!(
                    "Session is used by tunnels: {}. Remove or move them first.",
                    dependencies.join(", ")
                ));
            }
            Ok(registry.sessions.remove(index))
        })
    }
    pub fn favorite(
        &self,
        target: &str,
        group: Option<&str>,
        favorite: bool,
    ) -> Result<Session, String> {
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, target, group)?;
            let s = &mut registry.sessions[index];
            s.favorite = favorite;
            s.updated_at = Utc::now();
            Ok(s.clone())
        })
    }
    pub fn record_attempt(&self, id: &str) -> Result<Session, String> {
        self.storage.update_registry(|registry| {
            let index = resolve_session_index(registry, id, None)?;
            registry.sessions[index].last_connect_attempt_at = Some(Utc::now());
            Ok(registry.sessions[index].clone())
        })
    }
    pub fn connect_returning(&self, id: &str) -> Result<i32, String> {
        let session = self.record_attempt(id)?;
        crate::connect::SshConnection::execute_returning(&session, &[])
    }
    pub fn ping(
        &self,
        id: &str,
        icmp: bool,
        seconds: Option<u64>,
    ) -> Result<crate::connect::PingResult, String> {
        let s = self.get(id, None)?;
        let seconds = seconds.unwrap_or(self.storage.load_config()?.ping_timeout_seconds);
        validation::timeout(seconds)?;
        Ok(if icmp {
            crate::connect::PingService::ping_icmp(&s.host, seconds)
        } else {
            crate::connect::PingService::ping_tcp(
                &s.host,
                s.port,
                std::time::Duration::from_secs(seconds),
            )
        })
    }
    pub fn set_config(&self, key: &str, value: &str) -> Result<crate::config::AppConfig, String> {
        self.storage.update_config(|config| {
            match key.to_lowercase().as_str() {
                "id_type" => {
                    config.id_type = match value {
                        "integer" => IdType::Integer,
                        "uuid" => IdType::Uuid,
                        _ => return Err("id_type must be integer or uuid".into()),
                    }
                }
                "default_port" => {
                    config.default_port = value.parse().map_err(|_| "Invalid port")?
                }
                "confirm_delete" => {
                    config.confirm_delete = match value.to_lowercase().as_str() {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        _ => return Err("Expected true/false/1/0".into()),
                    }
                }
                "ping_timeout_seconds" => {
                    config.ping_timeout_seconds = value.parse().map_err(|_| "Invalid timeout")?
                }
                _ => return Err("Unknown configuration key".into()),
            }
            Ok(config.clone())
        })
    }
    pub fn add_tunnel(&self, args: TunnelAddArgs) -> Result<Tunnel, String> {
        let config = self.storage.load_config()?;
        self.storage.update_registry(|registry| {
            let session_id = resolve_session(registry, &args.session, args.group.as_deref())?
                .id
                .clone();
            let now = Utc::now();
            let tunnel = Tunnel {
                id: generate_id(config.id_type, &mut registry.next_tunnel_id)?,
                name: args.name.trim().to_owned(),
                session_id,
                kind: args.kind,
                bind: args.bind,
                listen_port: args.listen_port,
                destination_host: args.destination_host,
                destination_port: args.destination_port,
                allow_public_bind: args.allow_public_bind,
                created_at: now,
                updated_at: now,
            };
            validation::tunnel(&tunnel)?;
            registry.tunnels.push(tunnel.clone());
            Ok(tunnel)
        })
    }
    pub fn edit_tunnel(&self, args: TunnelEditArgs) -> Result<Tunnel, String> {
        self.storage.update_registry(|registry| {
            let index = crate::selection::resolve_tunnel_index(registry, &args.target)?;
            let session_id = args
                .session
                .as_deref()
                .map(|target| {
                    resolve_session(registry, target, args.group.as_deref()).map(|s| s.id.clone())
                })
                .transpose()?;
            let t = &mut registry.tunnels[index];
            if let Some(value) = args.name {
                t.name = value.trim().to_owned();
            }
            if let Some(value) = session_id {
                t.session_id = value;
            }
            if let Some(value) = args.kind {
                if value == TunnelKind::Dynamic
                    && (args.destination_host.is_some() || args.destination_port.is_some())
                {
                    return Err("Dynamic tunnels do not accept destination fields".into());
                }
                t.kind = value;
                if value == TunnelKind::Dynamic {
                    t.destination_host = None;
                    t.destination_port = None;
                }
            }
            if let Some(value) = args.bind {
                if value != t.bind {
                    t.allow_public_bind = false;
                }
                t.bind = value;
            }
            if let Some(value) = args.listen_port {
                t.listen_port = value;
            }
            if let Some(value) = args.destination_host {
                t.destination_host = Some(value);
            }
            if let Some(value) = args.destination_port {
                t.destination_port = Some(value);
            }
            if args.allow_public_bind {
                t.allow_public_bind = true;
            }
            t.updated_at = Utc::now();
            validation::tunnel(t)?;
            Ok(t.clone())
        })
    }
    pub fn get_tunnel(&self, target: &str) -> Result<Tunnel, String> {
        let registry = self.storage.load_registry()?;
        Ok(registry.tunnels[crate::selection::resolve_tunnel_index(&registry, target)?].clone())
    }
    pub fn delete_tunnel(&self, target: &str) -> Result<Tunnel, String> {
        self.storage.update_registry(|registry| {
            let index = crate::selection::resolve_tunnel_index(registry, target)?;
            Ok(registry.tunnels.remove(index))
        })
    }
    pub fn run_tunnel_returning(&self, target: &str) -> Result<i32, String> {
        let tunnel = self.get_tunnel(target)?;
        let session = self.record_attempt(&tunnel.session_id)?;
        crate::connect::SshConnection::execute_returning(&session, &tunnel_args(&tunnel)?)
    }
}

fn check_tags(tags: &[String]) -> Result<(), String> {
    if !tags.is_empty() && normalize_tags(tags).is_empty() {
        Err("Tags must not be entirely empty; use --clear-tags".into())
    } else {
        Ok(())
    }
}

pub fn tunnel_args(tunnel: &Tunnel) -> Result<Vec<String>, String> {
    validation::tunnel(tunnel)?;
    let address = |value: &str| {
        if value.contains(':') {
            format!("[{value}]")
        } else {
            value.to_owned()
        }
    };
    let mut value = format!("{}:{}", address(&tunnel.bind), tunnel.listen_port);
    if tunnel.kind != TunnelKind::Dynamic {
        value.push_str(&format!(
            ":{}:{}",
            address(
                tunnel
                    .destination_host
                    .as_deref()
                    .ok_or("Missing destination")?
            ),
            tunnel.destination_port.ok_or("Missing destination port")?
        ));
    }
    Ok(vec![
        "-N".into(),
        "-o".into(),
        "ExitOnForwardFailure=yes".into(),
        match tunnel.kind {
            TunnelKind::Local => "-L",
            TunnelKind::Remote => "-R",
            TunnelKind::Dynamic => "-D",
        }
        .into(),
        value,
    ])
}
