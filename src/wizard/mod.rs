mod onboarding;
mod prompts;
mod sessions;
mod transfer;
mod tunnels;
use crate::{
    cli::{Cli, ListArgs},
    service::AppService,
    storage::StorageManager,
};
pub use onboarding::onboard;
use prompts::select;
pub use prompts::{deletion, pick_session};
pub use sessions::session_form;
pub use transfer::transfer;
pub use tunnels::{tunnel_form, tunnels};
pub fn menu(service: &AppService) -> Result<(), String> {
    match select("Clisshman", vec!["CLI", "TUI", "Help", "Exit"])? {
        Some("CLI") => Wizard::start(service, &service.storage),
        Some("TUI") => crate::tui::TuiApp::new(service.clone(), service.storage.clone())?.run(),
        Some("Help") => {
            use clap::CommandFactory;
            Cli::command()
                .print_help()
                .map_err(|_| "Cannot print help")?;
            println!();
            Ok(())
        }
        _ => Ok(()),
    }
}

pub struct Wizard;
impl Wizard {
    pub fn start(service: &AppService, storage: &StorageManager) -> Result<(), String> {
        onboard(storage)?;
        loop {
            let Some(action) = select(
                "Clisshman",
                vec![
                    "Quick connect",
                    "List",
                    "Show",
                    "Add",
                    "Edit",
                    "Duplicate",
                    "Favorite / Unfavorite",
                    "Delete",
                    "Ping",
                    "Tunnels",
                    "Transfer files (SCP)",
                    "Exit",
                ],
            )?
            else {
                return Ok(());
            };
            if action == "Exit" {
                return Ok(());
            }
            let result = (|| {
                match action {
                    "Add" => return session_form(service, None, false),
                    "List" => {
                        return crate::output::sessions(&service.list(&ListArgs::default())?, false)
                    }
                    "Tunnels" => return tunnels(service),
                    _ => {}
                }
                let sessions = crate::selection::ranked(service.list(&ListArgs::default())?, "");
                let Some(id) = pick_session(sessions)? else {
                    return Ok(());
                };
                let session = service.get(&id, None)?;
                match action {
                    "Quick connect" => {
                        println!("SSH exited with status {}", service.connect_returning(&id)?);
                    }
                    "Transfer files (SCP)" => {
                        transfer(service, &id)?;
                    }
                    "Show" => crate::output::session(&session, false)?,
                    "Edit" => session_form(service, Some(&session), false)?,
                    "Duplicate" => session_form(service, Some(&session), true)?,
                    "Favorite / Unfavorite" => {
                        service.favorite(&id, None, !session.favorite)?;
                    }
                    "Delete" => {
                        if deletion("Delete session?")? {
                            service.delete(&id, None)?;
                        }
                    }
                    "Ping" => {
                        let result = service.ping(&id, false, None)?;
                        println!("{}", result.message);
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(error) = result {
                println!("{error}");
            }
        }
    }
}
