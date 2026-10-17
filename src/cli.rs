use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "clisshman",
    about = "A fast and robust CLI & TUI SSH connection manager",
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
}

#[derive(Args, Debug, Clone)]
pub struct AddArgs {
    pub target: Option<String>,

    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    #[arg(short = 'g', long = "group")]
    pub group: Option<String>,

    #[arg(short = 'H', long = "host")]
    pub host: Option<String>,

    #[arg(short = 'u', long = "username")]
    pub username: Option<String>,

    #[arg(short = 'p', long = "port")]
    pub port: Option<u16>,

    #[arg(short = 'i', long = "identity")]
    pub identity_file: Option<String>,

    #[arg(short = 'J', long = "jump")]
    pub jump_host: Option<String>,

    #[arg(short = 'd', long = "description")]
    pub description: Option<String>,

    #[arg(short = 't', long = "tag", value_delimiter = ',')]
    pub tags: Vec<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ListArgs {
    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    #[arg(short = 'g', long = "group")]
    pub group: Option<String>,

    #[arg(short = 'H', long = "host")]
    pub host: Option<String>,

    #[arg(short = 'u', long = "username")]
    pub username: Option<String>,

    #[arg(short = 'p', long = "port")]
    pub port: Option<u16>,

    #[arg(short = 't', long = "tag", value_delimiter = ',')]
    pub tags: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ShowArgs {
    pub target: String,

    #[arg(short = 'g', long = "group")]
    pub group: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct EditArgs {
    pub target: String,

    #[arg(short = 'n', long = "name")]
    pub name: Option<String>,

    #[arg(short = 'g', long = "group")]
    pub group: Option<String>,

    #[arg(short = 'H', long = "host")]
    pub host: Option<String>,

    #[arg(short = 'u', long = "username")]
    pub username: Option<String>,

    #[arg(short = 'p', long = "port")]
    pub port: Option<u16>,

    #[arg(short = 'i', long = "identity")]
    pub identity_file: Option<String>,

    #[arg(short = 'J', long = "jump")]
    pub jump_host: Option<String>,

    #[arg(short = 'd', long = "description")]
    pub description: Option<String>,

    #[arg(short = 't', long = "tag", value_delimiter = ',')]
    pub tags: Vec<String>,

    #[arg(long = "clear-tags")]
    pub clear_tags: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DeleteArgs {
    pub target: String,

    #[arg(short = 'g', long = "group")]
    pub group: Option<String>,

    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ConnectArgs {
    pub target: String,

    #[arg(short = 'g', long = "group")]
    pub group: Option<String>,

    #[arg(long = "dry-run")]
    pub dry_run: bool,

    #[arg(last = true)]
    pub passthrough: Vec<String>,
}

#[derive(Args, Debug, Clone)]
pub struct PingArgs {
    pub target: String,

    #[arg(short = 'g', long = "group")]
    pub group: Option<String>,

    #[arg(long = "icmp")]
    pub icmp: bool,

    #[arg(long = "timeout")]
    pub timeout: Option<u64>,
}

#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    #[arg(short = 'o', long = "output")]
    pub output: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct ImportArgs {
    pub file: String,

    #[arg(short = 'y', long = "yes")]
    pub yes: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CompletionsArgs {
    pub shell: clap_complete::Shell,
}
