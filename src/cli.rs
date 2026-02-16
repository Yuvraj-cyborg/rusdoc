use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "rusdoc",
    about = "Terminal-first Rust documentation query engine",
    version,
    after_help = "EXAMPLES:\n  \
        rusdoc std::vec::Vec          Look up Vec documentation\n  \
        rusdoc HashMap::insert        Look up a method\n  \
        rusdoc tokio::spawn           Look up from a crate on docs.rs\n  \
        rusdoc --plain Result | less  Pipe plain output\n  \
        rusdoc cache clear            Clear the doc cache"
)]
pub struct Cli {
    /// The item path to look up (e.g. std::vec::Vec, HashMap::insert)
    #[arg(value_name = "PATH")]
    pub query: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Rich)]
    pub format: OutputFormat,

    /// Force re-fetch, ignoring cache
    #[arg(long)]
    pub no_cache: bool,

    /// Use local project docs (runs cargo rustdoc)
    #[arg(short, long)]
    pub local: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage the documentation cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Search across all cached crate docs
    Search {
        /// Search query
        query: String,
        /// Max results to show
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Clear all cached documentation
    Clear,
    /// Show cache location and size
    Info,
    /// Update cached docs for a crate
    Update {
        /// Crate name to update
        name: String,
    },
}

#[derive(Copy, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Colored and formatted output (default)
    Rich,
    /// Plain text, no ANSI codes — suitable for piping
    Plain,
    /// JSON output for tooling
    Json,
}
