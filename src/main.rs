use clap::Parser;
use rusdoc::cli;

fn main() {
    let cli = cli::Cli::parse();
    println!("Query: {}", cli.path);
    println!("Hello World");
}

