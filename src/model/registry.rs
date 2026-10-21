use super::{Session, Tunnel};
use crate::config::IdType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
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
