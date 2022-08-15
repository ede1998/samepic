use std::{
    io::Write,
    path::{Path, PathBuf},
};

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
            no_open,
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
            if !no_open {
                show_folders(&destination, opener)?;
            }
        }
        Commands::End => todo!(),
    }
    Ok(())
}

fn show_folders(path: &Utf8Path, opener: Option<PathBuf>) -> Result<()> {
    for dir in path.read_dir()? {
        let dir = dir?.path();
        match opener {
            Some(ref opener) => {
                spawn_process(opener, &dir)?;
            }
            None => {
                opener::open(dir)?;
                pause();
            }
        }
    }
    Ok(())
}

fn spawn_process(exe: &Path, arg: &Path) -> Result<()> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(exe)
        .arg(arg)
        .stderr(Stdio::inherit())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .spawn()?;

    child.wait()?;
    Ok(())
}

fn pause() {
    use std::io::Read;

    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    stdout
        .write_all(b"Press enter to continue...")
        .expect("failed to write to stdout");
    stdout.flush().expect("failed to flush stdout");

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
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
        #[clap(short, long, value_parser = program)]
        opener: Option<PathBuf>,
        /// do not attempt to open image folders after sorting.
        #[clap(short, long, value_parser)]
        no_open: bool,
    },
    End,
}

fn dir(s: &str) -> Result<Utf8PathBuf> {
    let meta = std::fs::metadata(s)?;
    meta.is_dir()
        .then(|| s.into())
        .wrap_err("Source is not a directory.")
}

fn program(s: &str) -> Result<PathBuf> {
    use which::which;

    which(s).wrap_err_with(|| format!("Opener {s} is not a valid executable."))
}

fn create_dir(dir: &Utf8Path) -> Result<()> {
    std::fs::create_dir_all(dir).wrap_err_with(|| format!("Cannot create directory {}.", dir))?;
    match std::fs::read_dir(dir)?.next() {
        Some(_) => Err(eyre!("Target directory not empty.")),
        None => Ok(()),
    }
}
