use crate::model::TunnelKind;
use clap::{Args, Parser, Subcommand, ValueEnum};

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

#[derive(Args, Debug, Clone, Default)]
pub struct AddArgs {
    pub target: Option<String>,
    #[arg(short = 'n', long)]
    pub name: Option<String>,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(short = 'H', long)]
    pub host: Option<String>,
    #[arg(short = 'u', long)]
    pub username: Option<String>,
    #[arg(short = 'p', long)]
    pub port: Option<u16>,
    #[arg(short = 'i', long = "identity")]
    pub identity_file: Option<String>,
    #[arg(short = 'J', long = "jump")]
    pub jump_host: Option<String>,
    #[arg(short = 'd', long)]
    pub description: Option<String>,
    #[arg(short = 't', long = "tag", value_delimiter = ',')]
    pub tags: Vec<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ListArgs {
    #[arg(short = 'n', long)]
    pub name: Option<String>,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(short = 'H', long)]
    pub host: Option<String>,
    #[arg(short = 'u', long)]
    pub username: Option<String>,
    #[arg(short = 'p', long)]
    pub port: Option<u16>,
    #[arg(short = 't', long = "tag", value_delimiter = ',')]
    pub tags: Vec<String>,
    #[arg(long)]
    pub favorites: bool,
    #[arg(long)]
    pub recent: bool,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub limit: Option<u32>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ShowArgs {
    pub target: Option<String>,
    #[command(flatten)]
    pub filters: ListArgs,
}

#[derive(Args, Debug, Clone, Default)]
pub struct EditArgs {
    pub target: String,
    #[arg(long)]
    pub from_group: Option<String>,
    #[arg(short = 'n', long)]
    pub name: Option<String>,
    #[arg(short = 'g', long, conflicts_with = "clear_group")]
    pub group: Option<String>,
    #[arg(short = 'H', long)]
    pub host: Option<String>,
    #[arg(short = 'u', long)]
    pub username: Option<String>,
    #[arg(short = 'p', long)]
    pub port: Option<u16>,
    #[arg(short = 'i', long = "identity", conflicts_with = "clear_identity")]
    pub identity_file: Option<String>,
    #[arg(short = 'J', long = "jump", conflicts_with = "clear_jump")]
    pub jump_host: Option<String>,
    #[arg(short = 'd', long, conflicts_with = "clear_description")]
    pub description: Option<String>,
    #[arg(
        short = 't',
        long = "tag",
        value_delimiter = ',',
        conflicts_with = "clear_tags"
    )]
    pub tags: Vec<String>,
    #[arg(long)]
    pub clear_tags: bool,
    #[arg(long)]
    pub clear_group: bool,
    #[arg(long)]
    pub clear_identity: bool,
    #[arg(long)]
    pub clear_jump: bool,
    #[arg(long)]
    pub clear_description: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct DuplicateArgs {
    pub target: String,
    #[arg(long)]
    pub from_group: Option<String>,
    #[arg(short = 'n', long)]
    pub name: String,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(short = 'H', long)]
    pub host: Option<String>,
    #[arg(short = 'u', long)]
    pub username: Option<String>,
    #[arg(short = 'p', long)]
    pub port: Option<u16>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct DeleteArgs {
    pub target: String,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct FavoriteArgs {
    pub target: String,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(long)]
    pub unset: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct QuickArgs {
    pub query: Option<String>,
    #[command(flatten)]
    pub filters: ListArgs,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ConnectArgs {
    pub target: Option<String>,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(last = true)]
    pub passthrough: Vec<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct PingArgs {
    pub target: String,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(long)]
    pub icmp: bool,
    #[arg(long)]
    pub timeout: Option<u64>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ConfigArgs {
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum TransferFormat {
    #[default]
    Ssh,
    Json,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ExportArgs {
    #[arg(short = 'o', long)]
    pub output: Option<String>,
    #[arg(long, value_enum, default_value = "ssh")]
    pub format: TransferFormat,
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ImportArgs {
    pub file: String,
    #[arg(short = 'y', long)]
    pub yes: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, value_enum, default_value = "ssh")]
    pub format: TransferFormat,
}

#[derive(Args, Debug, Clone)]
pub struct CompletionsArgs {
    pub shell: clap_complete::Shell,
}

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
