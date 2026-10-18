use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::IdType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    #[serde(default)]
    pub group: Option<String>,
    pub name: String,
    pub host: String,
    pub user: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub identity_file: Option<String>,
    #[serde(default)]
    pub jump_host: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub last_connect_attempt_at: Option<DateTime<Utc>>,
}

fn default_port() -> u16 {
    22
}

pub fn normalize_tags(raw_tags: &[String]) -> Vec<String> {
    let mut tags = Vec::<String>::new();
    for part in raw_tags.iter().flat_map(|item| item.split(',')) {
        let value = part.trim();
        if !value.is_empty()
            && !tags
                .iter()
                .any(|tag| tag.to_lowercase() == value.to_lowercase())
        {
            tags.push(value.to_owned());
        }
    }
    tags.sort_by_key(|tag| tag.to_lowercase());
    tags
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TunnelKind {
    Local,
    Remote,
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub session_id: String,
    pub kind: TunnelKind,
    pub bind: String,
    pub listen_port: u16,
    pub destination_host: Option<String>,
    pub destination_port: Option<u16>,
    #[serde(default)]
    pub allow_public_bind: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub version: u32,
    pub next_id: u64,
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub tunnels: Vec<Tunnel>,
    #[serde(default = "initial_id")]
    pub next_tunnel_id: u64,
}

fn initial_id() -> u64 {
    1
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            version: 2,
            next_id: 1,
            sessions: Vec::new(),
            tunnels: Vec::new(),
            next_tunnel_id: 1,
        }
    }
}

pub fn generate_id(kind: IdType, counter: &mut u64) -> Result<String, String> {
    match kind {
        IdType::Uuid => Ok(Uuid::new_v4().to_string()),
        IdType::Integer => {
            let id = *counter;
            *counter = counter.checked_add(1).ok_or("ID counter exhausted")?;
            Ok(id.to_string())
        }
    }
}
