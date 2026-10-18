use crate::storage::StorageManager;
use std::process::Command;

pub struct DoctorReport {
    pub ssh_available: bool,
    pub ssh_version: String,
    pub config_dir_writable: bool,
    pub config_dir_path: String,
}
pub struct DoctorService;
impl DoctorService {
    pub fn run(storage: &StorageManager) -> DoctorReport {
        let (ssh_available, ssh_version) = match Command::new("ssh").arg("-V").output() {
            Ok(output) if output.status.success() => {
                let version = if output.stderr.is_empty() {
                    &output.stdout
                } else {
                    &output.stderr
                };
                (true, String::from_utf8_lossy(version).trim().to_owned())
            }
            _ => (false, "OpenSSH is not available in PATH".into()),
        };
        DoctorReport {
            ssh_available,
            ssh_version,
            config_dir_writable: storage.load_config().is_ok() && storage.load_registry().is_ok(),
            config_dir_path: storage.config_path().display().to_string(),
        }
    }
    pub fn print_report(report: &DoctorReport) {
        println!(
            "OpenSSH: {} ({})",
            if report.ssh_available {
                "OK"
            } else {
                "MISSING"
            },
            report.ssh_version
        );
        println!(
            "Storage readable / schema valid: {}",
            report.config_dir_writable
        );
        println!("Configuration path: {}", report.config_dir_path);
        println!("Write permissions are checked when saving; this command does not create files.");
    }
}
