use super::SshConfigManager;
use crate::model::Session;
use chrono::Utc;
use std::collections::HashSet;
use std::path::Path;
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
}
