mod connections;
mod exchange;
mod interaction;
mod sessions;
mod tunnels;
use crate::{
    cli::{Cli, Commands},
    output,
    service::AppService,
};
use clap::CommandFactory;
use exchange::{export, import};
use interaction::require_terminal;
pub use interaction::{confirm, interactive, target};
use std::io;
use tunnels::tunnels;
pub fn run(cli: Cli, service: &AppService) -> Result<i32, String> {
    let json = cli.json;
    match cli.command {
        Some(Commands::Add(args)) => return sessions::add(args, service, json),
        Some(Commands::Edit(args)) => return sessions::edit(args, service, json),
        Some(Commands::Duplicate(args)) => return sessions::duplicate(args, service, json),
        Some(Commands::List(args)) => return sessions::list(args, service, json),
        Some(Commands::Show(args)) => return sessions::show(args, service, json),
        Some(Commands::Delete(args)) => return sessions::delete(args, service, json),
        Some(Commands::Favorite(args)) => return sessions::favorite(args, service, json),
        Some(Commands::Quick(args)) => return connections::quick(args, service, json),
        Some(Commands::Connect(args)) => return connections::connect(args, service, json),
        Some(Commands::Ping(args)) => return connections::ping(args, service, json),
        Some(Commands::Scp(args)) => return connections::scp(args, service, json),
        Some(Commands::Config(args)) => return sessions::config(args, service, json),
        Some(Commands::Tunnel(command)) => return tunnels(command, service, json),
        Some(Commands::Import(args)) => import(args, service, json)?,
        Some(Commands::Export(args)) => export(args, service, json)?,
        Some(Commands::Completions(args)) => {
            let mut command = Cli::command();
            clap_complete::generate(
                args.shell,
                &mut command,
                args.command_name,
                &mut io::stdout(),
            );
        }
        Some(Commands::Doctor) => {
            let report = crate::doctor::DoctorService::run(&service.storage);
            if json {
                output::json(
                    &serde_json::json!({"ssh_available":report.ssh_available,"ssh_version":report.ssh_version,"config_path":report.config_dir_path,"storage_readable":report.config_dir_writable}),
                )?;
            } else {
                crate::doctor::DoctorService::print_report(&report);
            }
            return Ok(if report.ssh_available && report.config_dir_writable {
                0
            } else {
                1
            });
        }
        Some(Commands::Cli) => {
            require_terminal(json)?;
            crate::wizard::Wizard::start(service, &service.storage)?;
        }
        Some(Commands::Tui) => {
            require_terminal(json)?;
            crate::wizard::onboard(&service.storage)?;
            crate::tui::TuiApp::new(service.clone(), service.storage.clone())?.run()?;
        }
        None => {
            if !interactive() || json {
                Cli::command()
                    .print_help()
                    .map_err(|_| "Cannot print help")?;
                println!();
                return Ok(1);
            }
            crate::wizard::onboard(&service.storage)?;
            crate::wizard::menu(service)?;
        }
    }
    Ok(0)
}
