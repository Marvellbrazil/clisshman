use super::prompts::select;
use crate::storage::StorageManager;
pub fn onboard(storage: &StorageManager) -> Result<(), String> {
    if storage.config_path().exists() {
        return Ok(());
    }
    let Some(choice) = select(
        "Session IDs:",
        vec!["Incremental Integer (default)", "UUID v4"],
    )?
    else {
        return Ok(());
    };
    storage.update_config(|config| {
        config.id_type = if choice.starts_with("UUID") {
            crate::config::IdType::Uuid
        } else {
            crate::config::IdType::Integer
        };
        Ok(())
    })?;
    storage.update_registry(|_| Ok(()))?;
    let report = crate::doctor::DoctorService::run(storage);
    if !report.ssh_available {
        println!("OpenSSH client not found. Install it before connecting; profiles remain usable.");
    }
    Ok(())
}
