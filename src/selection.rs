use crate::{
    cli::ListArgs,
    model::{normalize_tags, Registry, Session, Tunnel},
};

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

pub fn filtered(registry: &Registry, filters: &ListArgs) -> Vec<Session> {
    let tags = normalize_tags(&filters.tags);
    let mut sessions: Vec<_> = registry
        .sessions
        .iter()
        .filter(|s| {
            filters.group.as_ref().is_none_or(|g| {
                s.group
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(g.trim())
            }) && filters
                .name
                .as_ref()
                .is_none_or(|v| s.name.to_lowercase().contains(&v.trim().to_lowercase()))
                && filters
                    .host
                    .as_ref()
                    .is_none_or(|v| s.host.to_lowercase().contains(&v.trim().to_lowercase()))
                && filters
                    .username
                    .as_ref()
                    .is_none_or(|v| s.user.to_lowercase().contains(&v.trim().to_lowercase()))
                && filters.port.is_none_or(|v| s.port == v)
                && (!filters.favorites || s.favorite)
                && (!filters.recent || s.last_connect_attempt_at.is_some())
                && tags
                    .iter()
                    .all(|v| s.tags.iter().any(|t| t.to_lowercase() == v.to_lowercase()))
        })
        .cloned()
        .collect();
    sessions.sort_by(|a, b| {
        if filters.recent {
            b.last_connect_attempt_at
                .cmp(&a.last_connect_attempt_at)
                .then_with(|| a.id.cmp(&b.id))
        } else {
            match (a.id.parse::<u64>(), b.id.parse::<u64>()) {
                (Ok(a), Ok(b)) => a.cmp(&b),
                (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                _ => a.id.cmp(&b.id),
            }
        }
    });
    if let Some(limit) = filters.limit {
        sessions.truncate(limit as usize);
    }
    sessions
}

pub fn fuzzy_score(value: &str, query: &str) -> Option<usize> {
    let value = value.to_lowercase();
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(0);
    }
    if value == query {
        return Some(0);
    }
    if let Some(position) = value.find(&query) {
        return Some(1 + position);
    }
    let mut remaining = query.chars();
    let mut wanted = remaining.next()?;
    let mut score = 100;
    for (position, character) in value.chars().enumerate() {
        if character == wanted {
            score += position;
            match remaining.next() {
                Some(next) => wanted = next,
                None => return Some(score),
            }
        }
    }
    None
}

pub fn ranked(sessions: Vec<Session>, query: &str) -> Vec<Session> {
    let mut scored: Vec<_> = sessions
        .into_iter()
        .filter_map(|s| {
            let fields = [
                s.name.clone(),
                s.group.clone().unwrap_or_default(),
                s.host.clone(),
                s.user.clone(),
                s.tags.join(" "),
            ];
            fields
                .iter()
                .filter_map(|field| fuzzy_score(field, query))
                .min()
                .map(|score| (score, s))
        })
        .collect();
    scored.sort_by(|(score_a, a), (score_b, b)| {
        score_a
            .cmp(score_b)
            .then_with(|| b.favorite.cmp(&a.favorite))
            .then_with(|| b.last_connect_attempt_at.cmp(&a.last_connect_attempt_at))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    scored.into_iter().map(|(_, s)| s).collect()
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
