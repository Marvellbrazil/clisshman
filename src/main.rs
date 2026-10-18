use clap::Parser;
use clisshman::{cli::Cli, service::AppService, storage::StorageManager};
fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    let result = StorageManager::new()
        .and_then(|storage| clisshman::dispatch::run(cli, &AppService::new(storage)));
    let code = match result {
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
    };
    std::process::exit(code);
}
