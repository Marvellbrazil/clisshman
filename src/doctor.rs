use std::process::Command;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, ContentArrangement, Table};

use crate::storage::StorageManager;

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
            Ok(output) => {
                let err_out = String::from_utf8_lossy(&output.stderr).to_string();
                let std_out = String::from_utf8_lossy(&output.stdout).to_string();
                let ver = if !err_out.trim().is_empty() {
                    err_out.trim().to_string()
                } else if !std_out.trim().is_empty() {
                    std_out.trim().to_string()
                } else {
                    "OpenSSH detected".to_string()
                };
                (true, ver)
            }
            Err(_) => (false, "Not found in PATH".to_string()),
        };

        let config_dir_path = storage.registry_path().parent().unwrap().display().to_string();
        let config_dir_writable = storage.load_config().is_ok();

        DoctorReport {
            ssh_available,
            ssh_version,
            config_dir_writable,
            config_dir_path,
        }
    }

    pub fn print_report(report: &DoctorReport) {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic);

        table.set_header(vec![
            Cell::new("Component").fg(Color::Cyan),
            Cell::new("Status").fg(Color::Cyan),
            Cell::new("Details").fg(Color::Cyan),
        ]);

        let ssh_status = if report.ssh_available {
            Cell::new("OK").fg(Color::Green)
        } else {
            Cell::new("MISSING").fg(Color::Red)
        };
        table.add_row(vec![
            Cell::new("OpenSSH Client"),
            ssh_status,
            Cell::new(&report.ssh_version),
        ]);

        let dir_status = if report.config_dir_writable {
            Cell::new("OK").fg(Color::Green)
        } else {
            Cell::new("ERROR").fg(Color::Red)
        };
        table.add_row(vec![
            Cell::new("Storage Directory"),
            dir_status,
            Cell::new(&report.config_dir_path),
        ]);

        println!("{table}");
    }
}
