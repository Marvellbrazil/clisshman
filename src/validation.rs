use crate::model::{normalize_tags, Registry, Session, Tunnel, TunnelKind};
use std::net::IpAddr;
use uuid::Uuid;

fn valid_id(value: &str) -> bool {
    value
        .parse::<u64>()
        .is_ok_and(|id| id > 0 && id.to_string() == value)
        || Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}

pub fn text(value: &str, field: &str) -> Result<(), String> {
    if value.len() > 4096 || value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(format!(
            "{field} must be nonempty, at most 4096 bytes, and contain no control characters"
        ));
    }
    Ok(())
}

pub fn host(value: &str) -> Result<(), String> {
    text(value, "Host")?;
    if value.starts_with('-')
        || value.chars().any(char::is_whitespace)
        || value.contains(['@', '/', '\\', '[', ']'])
    {
        return Err("Host must be a DNS name or an unbracketed IP address".into());
    }
    if value.contains(':') && value.parse::<IpAddr>().is_err() {
        return Err("Invalid IPv6 host".into());
    }
    if !value.contains(':')
        && !value
            .chars()
            .all(|c| c.is_alphanumeric() || "._-".contains(c))
    {
        return Err("Host contains unsupported characters".into());
    }
    Ok(())
}

pub fn name(value: &str) -> Result<(), String> {
    text(value, "Name")?;
    if value.bytes().all(|c| c.is_ascii_digit())
        || Uuid::parse_str(value).is_ok()
        || (value.len() >= 8 && value.bytes().all(|c| c.is_ascii_hexdigit()))
    {
        return Err("Name must not resemble an integer or UUID identifier".into());
    }
    Ok(())
}

pub fn port(value: u16) -> Result<(), String> {
    if value == 0 {
        Err("Port must be between 1 and 65535".into())
    } else {
        Ok(())
    }
}

pub fn timeout(value: u64) -> Result<(), String> {
    if value == 0 || value > 86400 {
        Err("Timeout must be between 1 and 86400 seconds".into())
    } else {
        Ok(())
    }
}

pub fn session(session: &mut Session) -> Result<(), String> {
    session.name = session.name.trim().to_owned();
    name(&session.name)?;
    host(&session.host)?;
    text(&session.user, "Username")?;
    if session.user.starts_with('-')
        || session.user.chars().any(char::is_whitespace)
        || session.user.contains('@')
    {
        return Err("Invalid SSH username".into());
    }
    port(session.port)?;
    if let Some(group) = &session.group {
        text(group, "Group")?;
    }
    session.group = session.group.take().map(|v| v.trim().to_owned());
    for (field, value) in [
        ("Identity", &session.identity_file),
        ("Jump host", &session.jump_host),
        ("Description", &session.description),
    ] {
        if let Some(value) = value {
            text(value, field)?;
        }
    }
    if let Some(jump) = &session.jump_host {
        for hop in jump.split(',') {
            let (user, hostname, port_number) = parse_target(hop)?;
            if user.as_deref().is_some_and(|user| {
                user.is_empty() || user.starts_with('-') || user.chars().any(char::is_whitespace)
            }) {
                return Err("Invalid jump host username".into());
            }
            host(&hostname)?;
            if let Some(port_number) = port_number {
                port(port_number)?;
            }
        }
    }
    for tag in &session.tags {
        for value in tag.split(',').filter(|v| !v.trim().is_empty()) {
            text(value, "Tag")?;
        }
    }
    session.tags = normalize_tags(&session.tags);
    Ok(())
}

pub fn same_profile(a: &Session, b: &Session) -> bool {
    a.name.to_lowercase() == b.name.to_lowercase()
        && a.group.as_deref().unwrap_or("").to_lowercase()
            == b.group.as_deref().unwrap_or("").to_lowercase()
}

pub fn registry(registry: &mut Registry) -> Result<(), String> {
    if registry.next_id == 0 || registry.next_tunnel_id == 0 {
        return Err("ID counters must be positive".into());
    }
    let mut ids = std::collections::HashSet::new();
    for session in &mut registry.sessions {
        if !valid_id(&session.id) || !ids.insert(session.id.clone()) {
            return Err("Duplicate or invalid session ID".into());
        }
        self::session(session)?;
    }
    for (i, session) in registry.sessions.iter().enumerate() {
        if registry.sessions[..i]
            .iter()
            .any(|other| same_profile(session, other))
        {
            return Err(format!("Duplicate session name in group: {}", session.name));
        }
    }
    if registry
        .sessions
        .iter()
        .filter_map(|s| s.id.parse::<u64>().ok())
        .any(|id| id >= registry.next_id)
    {
        return Err("Invalid session ID counter".into());
    }
    let mut tunnel_ids = std::collections::HashSet::new();
    let mut names = std::collections::HashSet::new();
    for tunnel in &registry.tunnels {
        self::tunnel(tunnel)?;
        if !ids.contains(&tunnel.session_id) {
            return Err(format!(
                "Tunnel '{}' references a missing session",
                tunnel.name
            ));
        }
        if !valid_id(&tunnel.id)
            || !tunnel_ids.insert(tunnel.id.clone())
            || !names.insert(tunnel.name.to_lowercase())
        {
            return Err("Duplicate tunnel ID or name".into());
        }
    }
    if registry
        .tunnels
        .iter()
        .filter_map(|t| t.id.parse::<u64>().ok())
        .any(|id| id >= registry.next_tunnel_id)
    {
        return Err("Invalid tunnel ID counter".into());
    }
    Ok(())
}

pub fn tunnel(tunnel: &Tunnel) -> Result<(), String> {
    name(&tunnel.name)?;
    port(tunnel.listen_port)?;
    let bind = tunnel
        .bind
        .parse::<IpAddr>()
        .map_err(|_| "Tunnel bind must be an IP address")?;
    if !bind.is_loopback() && !tunnel.allow_public_bind {
        return Err("Non-loopback bind requires --allow-public-bind".into());
    }
    match tunnel.kind {
        TunnelKind::Dynamic => {
            if tunnel.destination_host.is_some() || tunnel.destination_port.is_some() {
                return Err("Dynamic tunnels do not accept a destination".into());
            }
        }
        _ => {
            host(
                tunnel
                    .destination_host
                    .as_deref()
                    .ok_or("Destination host is required")?,
            )?;
            port(
                tunnel
                    .destination_port
                    .ok_or("Destination port is required")?,
            )?;
        }
    }
    Ok(())
}

pub fn parse_target(value: &str) -> Result<(Option<String>, String, Option<u16>), String> {
    let (user, address) = match value.split_once('@') {
        Some((user, address)) => (Some(user.to_owned()), address),
        None => (None, value),
    };
    let (hostname, port_number) = if let Some(rest) = address.strip_prefix('[') {
        let (hostname, suffix) = rest
            .split_once(']')
            .ok_or("IPv6 address requires closing bracket")?;
        let port_number = if suffix.is_empty() {
            None
        } else {
            Some(
                suffix
                    .strip_prefix(':')
                    .ok_or("Invalid IPv6 port syntax")?
                    .parse::<u16>()
                    .map_err(|_| "Invalid port")?,
            )
        };
        (hostname.to_owned(), port_number)
    } else if address.matches(':').count() == 1 {
        let (hostname, port_number) = address.split_once(':').ok_or("Invalid target")?;
        (
            hostname.to_owned(),
            Some(port_number.parse::<u16>().map_err(|_| "Invalid port")?),
        )
    } else {
        (address.to_owned(), None)
    };
    host(&hostname)?;
    if let Some(port_number) = port_number {
        port(port_number)?;
    }
    Ok((user, hostname, port_number))
}
