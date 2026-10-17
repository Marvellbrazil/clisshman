use clap::Parser;
use clisshman::cli::{Cli, Commands};
use clisshman::service::AppService;
use clisshman::storage::StorageManager;

fn main() {
    let cli = Cli::parse();
    let as_json = cli.json;

    let storage = match StorageManager::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Storage initialization error: {e}");
            std::process::exit(1);
        }
    };

    let service = AppService::new(storage.clone());

    let result = match cli.command {
        Some(Commands::Add(args)) => service.handle_add(args, as_json),
        Some(Commands::List(args)) => service.handle_list(args, as_json),
        Some(Commands::Show(args)) => service.handle_show(args, as_json),
        Some(Commands::Edit(args)) => service.handle_edit(args, as_json),
        Some(Commands::Delete(args)) => service.handle_delete(args, as_json),
        Some(Commands::Config(args)) => service.handle_config(args, as_json),
        Some(Commands::Connect(args)) => service.handle_connect(args),
        Some(Commands::Ping(args)) => service.handle_ping(args, as_json),
        Some(Commands::Cli) => clisshman::wizard::Wizard::start(&service, &storage),
        Some(Commands::Tui) => match clisshman::tui::TuiApp::new(service, storage) {
            Ok(mut app) => app.run(),
            Err(e) => Err(e),
        },
        Some(Commands::Doctor) => {
            let report = clisshman::doctor::DoctorService::run(&storage);
            clisshman::doctor::DoctorService::print_report(&report);
            Ok(())
        }
        Some(Commands::Export(args)) => service.handle_export(args),
        Some(Commands::Import(args)) => service.handle_import(args),
        Some(Commands::Completions(args)) => service.handle_completions(args),
        None => {
            if crossterm::tty::IsTty::is_tty(&std::io::stdout()) {
                match clisshman::tui::TuiApp::new(service, storage) {
                    Ok(mut app) => app.run(),
                    Err(e) => Err(e),
                }
            } else {
                use clap::CommandFactory;
                let mut cmd = Cli::command();
                let _ = cmd.print_help();
                println!();
                Ok(())
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
