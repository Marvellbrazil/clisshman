use clap::Args;
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
