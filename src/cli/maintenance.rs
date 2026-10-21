use clap::{Args, ValueEnum};
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
    #[arg(long, value_parser = ["clisshman", "csm"], default_value = "clisshman")]
    pub command_name: String,
}
