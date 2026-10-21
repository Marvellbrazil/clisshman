use clap::Args;
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

#[derive(Args, Debug, Clone)]
pub struct ScpArgs {
    pub target: String,
    #[arg(short = 'g', long)]
    pub group: Option<String>,
    #[arg(short = 'f', long = "file", required = true)]
    pub files: Vec<std::path::PathBuf>,
    #[arg(long)]
    pub to: std::path::PathBuf,
    #[arg(long)]
    pub download: bool,
    #[arg(short = 'r', long)]
    pub recursive: bool,
    #[arg(short = 'y', long)]
    pub yes: bool,
    #[arg(long)]
    pub dry_run: bool,
}
