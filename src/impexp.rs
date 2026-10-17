use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use chrono::Utc;
use uuid::Uuid;

use crate::config::IdType;
use crate::model::{Registry, Session};

pub struct SshConfigManager;

impl SshConfigManager {
    pub fn export_to_writer<W: Write>(registry: &Registry, mut writer: W) -> Result<(), String> {
        for s in &registry.sessions {
            writeln!(writer, "Host {}", s.name).map_err(|e| e.to_string())?;
            writeln!(writer, "    HostName {}", s.host).map_err(|e| e.to_string())?;
            writeln!(writer, "    User {}", s.user).map_err(|e| e.to_string())?;
            if s.port != 22 {
                writeln!(writer, "    Port {}", s.port).map_err(|e| e.to_string())?;
            }
            if let Some(ref id) = s.identity_file {
                writeln!(writer, "    IdentityFile {}", id).map_err(|e| e.to_string())?;
            }
            if let Some(ref jump) = s.jump_host {
                writeln!(writer, "    ProxyJump {}", jump).map_err(|e| e.to_string())?;
            }
            writeln!(writer).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn import_from_file<P: AsRef<Path>>(
        path: P,
        registry: &mut Registry,
        id_type: IdType,
    ) -> Result<usize, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);

        let mut current_name: Option<String> = None;
        let mut current_host: Option<String> = None;
        let mut current_user: Option<String> = None;
        let mut current_port: u16 = 22;
        let mut current_identity: Option<String> = None;
        let mut current_jump: Option<String> = None;

        let mut imported_count = 0;

        let save_current = |name: Option<String>,
                                host: Option<String>,
                                user: Option<String>,
                                port: u16,
                                identity: Option<String>,
                                jump: Option<String>,
                                reg: &mut Registry|
         -> Result<bool, String> {
            if let (Some(n), Some(h)) = (name, host) {
                let u = user.unwrap_or_else(|| "root".to_string());
                let id = match id_type {
                    IdType::Integer => {
                        let id_str = reg.next_id.to_string();
                        reg.next_id += 1;
                        id_str
                    }
                    IdType::Uuid => Uuid::new_v4().to_string(),
                };
                let now = Utc::now();
                reg.sessions.push(Session {
                    id,
                    group: Some("imported".to_string()),
                    name: n,
                    host: h,
                    user: u,
                    port,
                    identity_file: identity,
                    jump_host: jump,
                    description: Some("Imported from ssh_config".to_string()),
                    tags: vec!["imported".to_string()],
                    created_at: now,
                    updated_at: now,
                });
                Ok(true)
            } else {
                Ok(false)
            }
        };

        for line_res in reader.lines() {
            let line = line_res.map_err(|e| e.to_string())?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let key = parts[0].to_lowercase();
            let val = if parts.len() > 1 { parts[1] } else { "" };

            if key == "host" {
                if save_current(
                    current_name.take(),
                    current_host.take(),
                    current_user.take(),
                    current_port,
                    current_identity.take(),
                    current_jump.take(),
                    registry,
                )? {
                    imported_count += 1;
                }
                current_name = Some(val.to_string());
                current_port = 22;
            } else if key == "hostname" {
                current_host = Some(val.to_string());
            } else if key == "user" {
                current_user = Some(val.to_string());
            } else if key == "port" {
                if let Ok(p) = val.parse::<u16>() {
                    current_port = p;
                }
            } else if key == "identityfile" {
                current_identity = Some(val.to_string());
            } else if key == "proxyjump" {
                current_jump = Some(val.to_string());
            }
        }

        if save_current(
            current_name,
            current_host,
            current_user,
            current_port,
            current_identity,
            current_jump,
            registry,
        )? {
            imported_count += 1;
        }

        Ok(imported_count)
    }
}
