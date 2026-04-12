use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "hashjunkie", version, about = "Multi-algorithm file hasher")]
pub struct Args {
    /// Files to hash (omit to read from stdin)
    pub files: Vec<String>,

    /// Comma-separated list of algorithms (default: all)
    #[arg(short = 'a', long = "algorithms")]
    pub algorithms: Option<String>,

    /// Output format
    #[arg(long = "format", default_value = "json")]
    pub format: Format,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Format {
    Json,
    Hex,
}
