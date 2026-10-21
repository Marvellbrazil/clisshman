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
