use crate::{
    cli::ListArgs,
    model::{normalize_tags, Registry, Session},
};
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
