use super::SshConfigManager;
use crate::{model::Registry, validation};
use std::collections::HashSet;
use std::io::Write;
fn safe_alias(value: &str) -> String {
    let alias: String = value
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect();
    let alias = alias.trim_start_matches('-').to_owned();
    if alias.is_empty() {
        "session".into()
    } else {
        alias
    }
}

fn quote(value: &str) -> String {
    if !value
        .chars()
        .any(|c| c.is_whitespace() || c == '"' || c == '\\' || c == '#')
    {
        return value.to_owned();
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

impl SshConfigManager {
    pub fn export_to_writer<W: Write>(registry: &Registry, mut writer: W) -> Result<(), String> {
        for session in &registry.sessions {
            validation::session(&mut session.clone())?;
        }
        let natural: Vec<String> = registry
            .sessions
            .iter()
            .map(|s| safe_alias(&s.name))
            .collect();
        let mut used = HashSet::new();
        let mut output = String::new();
        for (index, session) in registry.sessions.iter().enumerate() {
            let base = &natural[index];
            let duplicated = natural
                .iter()
                .filter(|n| n.eq_ignore_ascii_case(base))
                .count()
                > 1;
            let mut alias = match (&session.group, duplicated) {
                (Some(group), true) => safe_alias(&format!("{group}-{}", session.name)),
                _ => base.clone(),
            };
            let stem = alias.clone();
            let mut suffix = 2;
            while !used.insert(alias.to_lowercase()) {
                alias = format!("{stem}-{suffix}");
                suffix += 1;
            }
            output.push_str(&format!(
                "Host {alias}\n    HostName {}\n    User {}\n",
                session.host, session.user
            ));
            if session.port != 22 {
                output.push_str(&format!("    Port {}\n", session.port));
            }
            if let Some(identity) = &session.identity_file {
                output.push_str(&format!("    IdentityFile {}\n", quote(identity)));
            }
            if let Some(jump) = &session.jump_host {
                output.push_str(&format!("    ProxyJump {jump}\n"));
            }
            output.push('\n');
        }
        writer
            .write_all(output.as_bytes())
            .map_err(|_| "Cannot write SSH config".into())
    }
}
