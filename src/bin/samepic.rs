use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use samepic::Repository;

use color_eyre::eyre::{eyre, Context, ContextCompat, Result};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    let args = Args::parse();

    match args.command {
        Commands::Start {
            source,
            destination,
            opener,
        } => {
            let destination = destination.unwrap_or_else(|| {
                let mut dest = source.clone();
                match source.file_name() {
                    Some(folder) => dest.set_file_name(format!("{folder}-sorted")),
                    None => dest.set_file_name("sorted"),
                }
                dest
            });
            let repo = Repository::new(source);
            create_dir(&destination)?;
            repo.create_piles(&destination)?;
        }
        Commands::End => todo!(),
    }
    Ok(())
}

/// Group similar images in joint folders so duplicates are easily deleteable
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// What to do
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Starts grouping all the images in source into a destination folder
    Start {
        /// Source folder to be sorted.
        #[clap(value_parser = dir)]
        source: Utf8PathBuf,
        /// destination to sort the pictures into. If it does not exist, it will be created. Defaults to `source`-sorted.
        #[clap(short, long, value_parser)]
        destination: Option<Utf8PathBuf>,
        /// Program to open the picture folders with. Defaults to the default folder explorer.
        #[clap(short, long, value_parser)]
        opener: Option<String>,
    },
    End,
}

fn dir(s: &str) -> Result<Utf8PathBuf> {
    let meta = std::fs::metadata(s)?;
    meta.is_dir()
        .then(|| s.into())
        .wrap_err("Source is not a directory.")
}

fn create_dir(dir: &Utf8Path) -> Result<()> {
    if let Ok(mut dir) = dir.read_dir() {
        if dir.next().is_none() {
            return Err(eyre!("Non-empty directory"));
        }
    }
    std::fs::create_dir_all(dir).wrap_err_with(|| format!("Failed to create directory {}", dir))
}
