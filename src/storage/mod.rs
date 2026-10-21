mod atomic;
use crate::{config::AppConfig, model::Registry, validation};
use directories::ProjectDirs;
use fs2::FileExt;
use serde::{de::DeserializeOwned, Serialize};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct StorageManager {
    base_dir: PathBuf,
}

impl StorageManager {
    pub fn new() -> Result<Self, String> {
        let path = if let Some(custom) = std::env::var_os("CLISSHMAN_CONFIG_DIR") {
            if custom.is_empty() {
                return Err("CLISSHMAN_CONFIG_DIR must not be empty".into());
            }
            PathBuf::from(custom)
        } else {
            ProjectDirs::from("com", "clisshman", "clisshman")
                .ok_or("Cannot locate configuration directory")?
                .config_dir()
                .to_path_buf()
        };
        Self::with_dir(path)
    }
    pub fn with_dir(path: PathBuf) -> Result<Self, String> {
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

    fn read<T: DeserializeOwned + Default>(&self, path: &Path) -> Result<T, String> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
                format!(
                    "Invalid data in {}. Restore a backup; the file was not reset.",
                    path.display()
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
            Err(_) => Err(format!("Cannot read {}", path.display())),
        }
    }
    pub fn load_config(&self) -> Result<AppConfig, String> {
        let config: AppConfig = self.read(&self.config_path())?;
        Self::validate_config(&config)?;
        Ok(config)
    }
    fn validate_config(config: &AppConfig) -> Result<(), String> {
        if config.version != 1 {
            return Err("Unsupported config schema version".into());
        }
        validation::port(config.default_port)?;
        validation::timeout(config.ping_timeout_seconds)
    }
    pub fn update_config<T>(
        &self,
        update: impl FnOnce(&mut AppConfig) -> Result<T, String>,
    ) -> Result<T, String> {
        let _lock = self.acquire_lock()?;
        let mut config = self.load_config()?;
        let result = update(&mut config)?;
        Self::validate_config(&config)?;
        self.write_json(&self.config_path(), &config)?;
        Ok(result)
    }
    pub fn load_registry(&self) -> Result<Registry, String> {
        let mut registry: Registry = self.read(&self.registry_path())?;
        if registry.version == 0 || registry.version > 2 {
            return Err("Unsupported registry schema version".into());
        }
        if registry.version == 1 {
            let highest = registry
                .sessions
                .iter()
                .filter_map(|s| s.id.parse::<u64>().ok())
                .max()
                .unwrap_or(0);
            registry.next_id = registry
                .next_id
                .max(highest.checked_add(1).ok_or("ID counter exhausted")?);
        }
        validation::registry(&mut registry)?;
        Ok(registry)
    }
    pub fn update_registry<T>(
        &self,
        update: impl FnOnce(&mut Registry) -> Result<T, String>,
    ) -> Result<T, String> {
        let _lock = self.acquire_lock()?;
        let mut registry = self.load_registry()?;
        let migrating = registry.version == 1;
        let result = update(&mut registry)?;
        registry.version = 2;
        validation::registry(&mut registry)?;
        if migrating {
            let backup = self.base_dir.join("sessions.v1.backup.json");
            if !backup.exists() {
                let bytes =
                    fs::read(self.registry_path()).map_err(|_| "Cannot read migration source")?;
                Self::atomic_write(&backup, &bytes, false)?;
            }
        }
        self.write_json(&self.registry_path(), &registry)?;
        Ok(result)
    }
    fn acquire_lock(&self) -> Result<File, String> {
        fs::create_dir_all(&self.base_dir).map_err(|_| "Cannot create configuration directory")?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(self.lock_path())
            .map_err(|_| "Cannot open storage lock")?;
        file.lock_exclusive()
            .map_err(|_| "Cannot acquire storage lock")?;
        Ok(file)
    }
    fn write_json(&self, path: &Path, value: &impl Serialize) -> Result<(), String> {
        let bytes =
            serde_json::to_vec_pretty(value).map_err(|_| "Cannot serialize application data")?;
        Self::atomic_write(path, &bytes, true)
    }
}
