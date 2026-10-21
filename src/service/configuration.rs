use super::AppService;
use crate::config::IdType;
impl AppService {
    pub fn set_config(&self, key: &str, value: &str) -> Result<crate::config::AppConfig, String> {
        self.storage.update_config(|config| {
            match key.to_lowercase().as_str() {
                "id_type" => {
                    config.id_type = match value {
                        "integer" => IdType::Integer,
                        "uuid" => IdType::Uuid,
                        _ => return Err("id_type must be integer or uuid".into()),
                    }
                }
                "default_port" => {
                    config.default_port = value.parse().map_err(|_| "Invalid port")?
                }
                "confirm_delete" => {
                    config.confirm_delete = match value.to_lowercase().as_str() {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        _ => return Err("Expected true/false/1/0".into()),
                    }
                }
                "ping_timeout_seconds" => {
                    config.ping_timeout_seconds = value.parse().map_err(|_| "Invalid timeout")?
                }
                _ => return Err("Unknown configuration key".into()),
            }
            Ok(config.clone())
        })
    }
}
