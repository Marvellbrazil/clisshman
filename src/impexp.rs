use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use chrono::Utc;

use crate::config::IdType;
use crate::model::{generate_id, Registry, Session};
use crate::validation;

pub struct SshConfigManager;

#[derive(Default)]
struct Block {
    aliases: Vec<String>,
    line: usize,
    host: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity: Option<String>,
    jump: Option<String>,
}

fn tokens(line: &str, number: usize) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted && matches!(chars.peek(), Some('"') | Some('\\')) => {
                current.extend(chars.next());
                started = true;
            }
            '#' if !quoted && !started => break,
            c if c.is_whitespace() && !quoted => {
                if started {
                    result.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return Err(format!("Line {number}: unterminated quote"));
    }
    if started {
        result.push(current);
    }
    Ok(result)
}

fn split_directive(line: &str, number: usize) -> Result<Option<(String, Vec<String>)>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    let end = trimmed
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(trimmed.len());
    let key = trimmed[..end].to_lowercase();
    let mut rest = trimmed[end..].trim_start();
    if let Some(value) = rest.strip_prefix('=') {
        rest = value.trim_start();
    }
    let values = tokens(rest, number)?;
    if values.is_empty() {
        return Err(format!("Line {number}: '{key}' requires a value"));
    }
    if values
        .iter()
        .any(|v| v.contains('%') || v.contains("${") || v.chars().any(char::is_control))
    {
        return Err(format!(
            "Line {number}: tokens and environment expansion are not supported"
        ));
    }
    Ok(Some((key, values)))
}

fn single(values: Vec<String>, key: &str, number: usize) -> Result<String, String> {
    match <[String; 1]>::try_from(values) {
        Ok([value]) => Ok(value),
        Err(_) => Err(format!("Line {number}: '{key}' accepts exactly one value")),
    }
}

fn set<T>(slot: &mut Option<T>, value: T, key: &str, number: usize) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!(
            "Line {number}: repeated '{key}' cannot be represented"
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn to_sessions(block: Block) -> Result<Vec<Session>, String> {
    let user = block.user.ok_or(format!(
        "Host block at line {}: User is required",
        block.line
    ))?;
    let now = Utc::now();
    Ok(block
        .aliases
        .into_iter()
        .map(|alias| Session {
            id: String::new(),
            group: Some("imported".into()),
            host: block.host.clone().unwrap_or_else(|| alias.clone()),
            name: alias,
            user: user.clone(),
            port: block.port.unwrap_or(22),
            identity_file: block.identity.clone(),
            jump_host: block.jump.clone(),
            description: Some("Imported from ssh_config".into()),
            tags: vec!["imported".into()],
            created_at: now,
            updated_at: now,
            favorite: false,
            last_connect_attempt_at: None,
        })
        .collect())
}

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
    pub fn parse_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Session>, String> {
        let content = std::fs::read_to_string(path).map_err(|_| "Cannot read SSH config file")?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Vec<Session>, String> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut aliases = HashSet::new();
        for (index, line) in content.lines().enumerate() {
            let number = index + 1;
            let Some((key, values)) = split_directive(line, number)? else {
                continue;
            };
            if key == "host" {
                for alias in &values {
                    if alias.contains(['*', '?', '!']) {
                        return Err(format!(
                            "Line {number}: Host patterns are not supported; use concrete aliases"
                        ));
                    }
                    if !aliases.insert(alias.to_lowercase()) {
                        return Err(format!(
                            "Line {number}: alias '{alias}' appears in more than one Host block"
                        ));
                    }
                }
                blocks.push(Block {
                    aliases: values,
                    line: number,
                    ..Default::default()
                });
                continue;
            }
            if matches!(key.as_str(), "include" | "match") {
                return Err(format!("Line {number}: '{key}' is not supported"));
            }
            let block = blocks.last_mut().ok_or(format!(
                "Line {number}: global '{key}' defaults are not supported"
            ))?;
            match key.as_str() {
                "hostname" => set(&mut block.host, single(values, &key, number)?, &key, number)?,
                "user" => set(&mut block.user, single(values, &key, number)?, &key, number)?,
                "port" => {
                    let port = single(values, &key, number)?
                        .parse::<u16>()
                        .ok()
                        .filter(|p| *p > 0)
                        .ok_or(format!("Line {number}: invalid Port"))?;
                    set(&mut block.port, port, &key, number)?
                }
                "identityfile" => set(
                    &mut block.identity,
                    single(values, &key, number)?,
                    &key,
                    number,
                )?,
                "proxyjump" => set(&mut block.jump, single(values, &key, number)?, &key, number)?,
                other => {
                    return Err(format!(
                        "Line {number}: directive '{other}' is not supported"
                    ))
                }
            }
        }
        let mut sessions = Vec::new();
        for block in blocks {
            sessions.extend(to_sessions(block)?);
        }
        Ok(sessions)
    }

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

    pub fn import_from_file<P: AsRef<Path>>(
        path: P,
        registry: &mut Registry,
        id_type: IdType,
    ) -> Result<usize, String> {
        let mut sessions = Self::parse_from_file(path)?;
        for session in &mut sessions {
            validation::session(session)?;
            if registry
                .sessions
                .iter()
                .any(|existing| validation::same_profile(existing, session))
            {
                return Err(format!("Session '{}' already exists", session.name));
            }
        }
        let count = sessions.len();
        for mut session in sessions {
            session.id = generate_id(id_type, &mut registry.next_id)?;
            registry.sessions.push(session);
        }
        Ok(count)
    }
}
