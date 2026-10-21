use crate::model::TunnelKind;
use clap::{Args, Subcommand};
#[derive(Subcommand, Debug)]
pub enum TunnelCommands {
    Add(TunnelAddArgs),
    List,
    Show(TunnelTargetArgs),
    Edit(TunnelEditArgs),
    Delete(TunnelDeleteArgs),
    Run(TunnelRunArgs),
}

#[derive(Args, Debug, Clone)]
pub struct TunnelAddArgs {
    #[arg(short = 'n', long)]
    pub name: String,
    #[arg(long)]
    pub session: String,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(long, value_enum)]
    pub kind: TunnelKind,
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,
    #[arg(long)]
    pub listen_port: u16,
    #[arg(long)]
    pub destination_host: Option<String>,
    #[arg(long)]
    pub destination_port: Option<u16>,
    #[arg(long)]
    pub allow_public_bind: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct TunnelTargetArgs {
    pub target: String,
}
#[derive(Args, Debug, Clone, Default)]
pub struct TunnelDeleteArgs {
    pub target: String,
    #[arg(short = 'y', long)]
    pub yes: bool,
}
#[derive(Args, Debug, Clone, Default)]
pub struct TunnelRunArgs {
    pub target: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct TunnelEditArgs {
    pub target: String,
    #[arg(short = 'n', long)]
    pub name: Option<String>,
    #[arg(long)]
    pub session: Option<String>,
    #[arg(short = 'g', long, requires = "session")]
    pub group: Option<String>,
    #[arg(long, value_enum)]
    pub kind: Option<TunnelKind>,
    #[arg(long)]
    pub bind: Option<String>,
    #[arg(long)]
    pub listen_port: Option<u16>,
    #[arg(long)]
    pub destination_host: Option<String>,
    #[arg(long)]
    pub destination_port: Option<u16>,
    #[arg(long)]
    pub allow_public_bind: bool,
}
