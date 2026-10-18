use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdType {
    #[default]
    Integer,
    Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub id_type: IdType,
    pub default_port: u16,
    pub confirm_delete: bool,
    pub ping_timeout_seconds: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            id_type: IdType::Integer,
            default_port: 22,
            confirm_delete: true,
            ping_timeout_seconds: 3,
        }
    }
}
