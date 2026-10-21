use crate::model::Session;
use inquire::{InquireError, Select, Text};
use std::fmt;
pub(super) fn optional<T>(result: Result<T, InquireError>) -> Result<Option<T>, String> {
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
pub(super) fn select<'a>(message: &str, items: Vec<&'a str>) -> Result<Option<&'a str>, String> {
    optional(Select::new(message, items).prompt())
}
pub(super) fn text(message: &str, initial: &str) -> Result<Option<String>, String> {
    optional(Text::new(message).with_initial_value(initial).prompt())
}
pub(super) fn field(message: &str, initial: &str) -> Result<String, String> {
    text(message, initial)?.ok_or("Cancelled".into())
}
pub fn deletion(message: &str) -> Result<bool, String> {
    Ok(select(message, vec!["Cancel", "Delete"])? == Some("Delete"))
}
