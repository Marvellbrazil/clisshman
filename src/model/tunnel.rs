use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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
