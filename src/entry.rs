use crate::{cli::Cli, service::AppService, storage::StorageManager};
use clap::Parser;

pub fn run() -> i32 {
    let cli = Cli::parse();
    let json = cli.json;
    let result = StorageManager::new()
        .and_then(|storage| crate::dispatch::run(cli, &AppService::new(storage)));
    match result {
        Ok(code) => code,
        Err(message) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({"success":false,"error":{"code":"COMMAND_FAILED","message":message}})
                );
            } else {
                eprintln!("Error: {message}");
            }
            1
        }
    }
}
