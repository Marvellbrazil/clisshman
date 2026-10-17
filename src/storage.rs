use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use fs2::FileExt;

use crate::config::AppConfig;
use crate::model::Registry;

#[derive(Clone)]
pub struct StorageManager {
    base_dir: PathBuf,
}

impl StorageManager {
    pub fn new() -> Result<Self, String> {
        let path = if let Ok(custom) = std::env::var("CLISSHMAN_CONFIG_DIR") {
            PathBuf::from(custom)
        } else if let Some(proj_dirs) = ProjectDirs::from("com", "clisshman", "clisshman") {
            proj_dirs.config_dir().to_path_buf()
        } else {
            PathBuf::from(".clisshman")
        };
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        Ok(Self { base_dir: path })
    }

    pub fn with_dir(path: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        Ok(Self { base_dir: path })
    }

    pub fn config_path(&self) -> PathBuf {
        self.base_dir.join("config.json")
    }

    pub fn registry_path(&self) -> PathBuf {
        self.base_dir.join("sessions.json")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.base_dir.join(".lock")
    }

    pub fn load_config(&self) -> Result<AppConfig, String> {
        let path = self.config_path();
        if !path.exists() {
            let default_cfg = AppConfig::default();
            self.save_config(&default_cfg)?;
            return Ok(default_cfg);
        }
        let mut file = File::open(&path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let serialized = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
        self.atomic_write(&self.config_path(), serialized.as_bytes())
    }

    pub fn load_registry(&self) -> Result<Registry, String> {
        let path = self.registry_path();
        if !path.exists() {
            let default_reg = Registry::default();
            self.save_registry(&default_reg)?;
            return Ok(default_reg);
        }
        let mut file = File::open(&path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub fn save_registry(&self, registry: &Registry) -> Result<(), String> {
        let _lock = self.acquire_lock()?;
        let serialized = serde_json::to_string_pretty(registry).map_err(|e| e.to_string())?;
        self.atomic_write(&self.registry_path(), serialized.as_bytes())
    }

    fn acquire_lock(&self) -> Result<File, String> {
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.lock_path())
            .map_err(|e| e.to_string())?;
        lock_file.lock_exclusive().map_err(|e| e.to_string())?;
        Ok(lock_file)
    }

    fn atomic_write(&self, target_path: &Path, data: &[u8]) -> Result<(), String> {
        let tmp_path = target_path.with_extension(format!("tmp.{}", std::process::id()));
        {
            let mut tmp_file = File::create(&tmp_path).map_err(|e| e.to_string())?;
            tmp_file.write_all(data).map_err(|e| e.to_string())?;
            tmp_file.sync_all().map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp_path, target_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            e.to_string()
        })?;
        Ok(())
    }
}
