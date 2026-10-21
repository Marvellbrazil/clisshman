mod connection;
mod maintenance;
mod session;
mod tunnel;
use clap::{Parser, Subcommand};
pub use connection::{ConnectArgs, PingArgs, ScpArgs};
pub use maintenance::{CompletionsArgs, ConfigArgs, ExportArgs, ImportArgs, TransferFormat};
pub use session::{
    AddArgs, DeleteArgs, DuplicateArgs, EditArgs, FavoriteArgs, ListArgs, QuickArgs, ShowArgs,
};
pub use tunnel::{
    TunnelAddArgs, TunnelCommands, TunnelDeleteArgs, TunnelEditArgs, TunnelRunArgs,
    TunnelTargetArgs,
};
#[derive(Parser, Debug)]
#[command(
    name = "clisshman",
    about = "Manage SSH sessions and foreground tunnels",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Add(AddArgs),
    List(ListArgs),
    Show(ShowArgs),
    Edit(EditArgs),
    Delete(DeleteArgs),
    Duplicate(DuplicateArgs),
    Favorite(FavoriteArgs),
    Quick(QuickArgs),
    #[command(short_flag = 'c', long_flag = "connect")]
    Connect(ConnectArgs),
    #[command(short_flag = 'P', long_flag = "ping")]
    Ping(PingArgs),
    Scp(ScpArgs),
    Config(ConfigArgs),
    Cli,
    Tui,
    Doctor,
    Export(ExportArgs),
    Import(ImportArgs),
    Completions(CompletionsArgs),
    #[command(subcommand)]
    Tunnel(TunnelCommands),
}
