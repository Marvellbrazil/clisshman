use crate::model::{Registry, Session, Tunnel};
pub fn candidates(registry: &Registry, target: &str, group: Option<&str>) -> Vec<Session> {
    let target = target.trim().to_lowercase();
    let available: Vec<_> = registry
        .sessions
        .iter()
        .filter(|s| {
            group.is_none_or(|g| {
                s.group
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(g.trim())
            })
        })
        .collect();
    let exact: Vec<_> = available
        .iter()
        .filter(|s| s.id.to_lowercase() == target)
        .map(|s| (*s).clone())
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    if target.bytes().all(|c| c.is_ascii_digit()) {
        return Vec::new();
    }
    let prefix = target.len() >= 8 && target.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-');
    available
        .into_iter()
        .filter(|s| {
            (prefix
                && uuid::Uuid::parse_str(&s.id).is_ok()
                && s.id.to_lowercase().starts_with(&target))
                || s.name.to_lowercase() == target
        })
        .cloned()
        .collect()
}

pub fn resolve_session_index(
    registry: &Registry,
    target: &str,
    group: Option<&str>,
) -> Result<usize, String> {
    let found = candidates(registry, target, group);
    if found.len() != 1 {
        return Err(if found.is_empty() {
            format!("Session '{target}' not found")
        } else {
            format!(
                "Ambiguous session '{target}': {}. Use a full ID or group.",
                found
                    .iter()
                    .map(|s| format!(
                        "{} ({}/{})",
                        s.id,
                        s.group.as_deref().unwrap_or("-"),
                        s.name
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
    }
    registry
        .sessions
        .iter()
        .position(|s| s.id == found[0].id)
        .ok_or("Session disappeared".into())
}

pub fn resolve_session<'a>(
    registry: &'a Registry,
    target: &str,
    group: Option<&str>,
) -> Result<&'a Session, String> {
    Ok(&registry.sessions[resolve_session_index(registry, target, group)?])
}

pub fn resolve_tunnel_index(registry: &Registry, target: &str) -> Result<usize, String> {
    let target = target.trim().to_lowercase();
    let target = target.as_str();
    let exact: Vec<_> = registry
        .tunnels
        .iter()
        .enumerate()
        .filter(|(_, t)| t.id == target)
        .collect();
    if exact.len() == 1 {
        return Ok(exact[0].0);
    }
    let found: Vec<(usize, &Tunnel)> = registry
        .tunnels
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.name.eq_ignore_ascii_case(target)
                || (target.len() >= 8
                    && uuid::Uuid::parse_str(&t.id).is_ok()
                    && t.id.starts_with(target))
        })
        .collect();
    match found.as_slice() {
        [(index, _)] => Ok(*index),
        [] => Err("Tunnel not found".into()),
        _ => Err("Ambiguous tunnel; use full ID".into()),
    }
}
