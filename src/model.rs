use std::collections::BTreeSet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}

fn default_port() -> u16 {
    22
}

pub fn normalize_tags(raw_tags: &[String]) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for item in raw_tags {
        for part in item.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                set.insert(trimmed.to_string());
            }
        }
    }
    set.into_iter().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub version: u32,
    pub next_id: u64,
    pub sessions: Vec<Session>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            sessions: Vec::new(),
        }
    }
}
