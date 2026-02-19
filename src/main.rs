use clap::Parser;
use owo_colors::OwoColorize;

use rusdoc::cache::Cache;
use rusdoc::cli::{CacheAction, Cli, Commands};
use rusdoc::render::render_item;
use rusdoc::resolver::{ResolveResult, format_disambiguation, resolve};
use rusdoc::source::DocSource;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(command) = &cli.command {
        return handle_command(command);
    }

    let query = match &cli.query {
        Some(q) => q.as_str(),
        None => {
            eprintln!(
                "{} no query provided. Try: {} std::vec::Vec",
                "error:".red().bold(),
                "rusdoc".cyan(),
            );
            std::process::exit(1);
        }
    };

    let source = DocSource::from_query(query, cli.local);

    eprint!("{}", "loading docs...".dimmed());
    let doc = source.load()?;
    eprint!("\r{}\r", " ".repeat(20));

    match resolve(&doc, query)? {
        ResolveResult::Found(item) => {
            print!("{}", render_item(&item, cli.format));
        }
        ResolveResult::Multiple(items) => {
            let count = items.len();
            eprintln!(
                "{} `{}` matched {} items:\n",
                "ambiguous:".yellow().bold(),
                query,
                count,
            );
            eprintln!("{}", format_disambiguation(&items));

            if let Some(first) = items.first() {
                eprintln!(
                    "{}\n",
                    format!("showing first match: {}", first.path.join("::")).dimmed(),
                );
                print!("{}", render_item(first, cli.format));
            }
        }
    }

    Ok(())
}

fn handle_command(command: &Commands) -> anyhow::Result<()> {
    match command {
        Commands::Cache { action } => {
            let cache = Cache::new()?;
            match action {
                CacheAction::Clear => {
                    cache.clear()?;
                    println!("{}", "cache cleared.".green());
                }
                CacheAction::Info => {
                    let info = cache.info()?;
                    println!("{info}");
                }
                CacheAction::Update { name } => {
                    let source = DocSource::from_query(name, false);
                    cache.evict(&source)?;
                    eprint!("{}", "re-fetching...".dimmed());
                    source.load()?;
                    eprint!("\r{}\r", " ".repeat(20));
                    println!("{}", format!("updated docs for `{name}`.").green());
                }
            }
        }
        Commands::Search { query, limit } => {
            eprintln!(
                "{} global search is not yet implemented. Use: {} <crate>::<item>",
                "note:".yellow().bold(),
                "rusdoc".cyan(),
            );
            eprintln!("  query={query}, limit={limit}");
        }
    }
    Ok(())
}
