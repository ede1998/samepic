use clap::{Parser, Subcommand};

use color_eyre::Result;

mod collect;
mod common;
mod completions;
mod open;
mod sort;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    let args = Args::parse();

    match args.command {
        Commands::Sort(sort) => sort.run(),
        Commands::Open(open) => open.run(),
        Commands::Collect(collect) => collect.run(),
        Commands::Completions(completions) => {
            completions.run();
            Ok(())
        }
    }
}

/// Group similar images in joint folders so duplicates are easily deleteable
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Sort(sort::Sort),
    Open(open::Open),
    Collect(collect::Collect),
    Completions(completions::Completions),
}
