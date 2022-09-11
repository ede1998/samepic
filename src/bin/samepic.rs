use std::{
    io::Write,
    path::{Path, PathBuf},
};

use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};
use samepic::Repository;

use color_eyre::{
    eyre::{eyre, Context, ContextCompat, Result},
    Help,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    let args = Args::parse();

    match args.command {
        Commands::Sort {
            source,
            destination,
            opener,
            no_open,
        } => {
            let destination = create_dir_from_ref_name(destination, &source, "sorted")?;
            let repo = Repository::new(source);
            repo.create_piles(&destination)?;
            if !no_open {
                show_folders(&destination, opener)?;
            }
        }
        Commands::Open { path, opener } => {
            show_folders(&path, opener)?;
        }
        Commands::Collect {
            source,
            destination,
            no_delete,
            keep_names,
        } => {
            let destination = create_dir_from_ref_name(destination, &source, "final")?;
            collect(&source, &destination, keep_names)?;
            if !no_delete {
                std::fs::remove_dir_all(&source)?;
            }
        }
    }
    Ok(())
}

fn show_folders(path: &Utf8Path, opener: Option<PathBuf>) -> Result<()> {
    let mut entries = path
        .read_dir()?
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    entries.sort_unstable_by_key(|e| e.path());

    for dir in entries {
        let dir = dir.path();
        tracing::info!("Showing pile {}", dir.display());
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

fn collect(source: &Utf8Path, destination: &Utf8Path, keep_names: bool) -> Result<()> {
    for dir in source.read_dir_utf8()? {
        let dir = dir?;
        if !dir.metadata()?.is_dir() {
            tracing::info!("Skipping {} because it is not a directory.", dir.path());
            continue;
        }

        tracing::info!("Disassembling pile {}", dir.path());
        for image in dir.path().read_dir_utf8()? {
            let image = image?;

            let link = generate_file_name(image.path(), destination, keep_names)?;
            std::fs::hard_link(image.path(), &link)
                .wrap_err_with(|| format!("Failed to create file {link}"))?;
        }
    }
    Ok(())
}

fn generate_file_name(
    original: &Utf8Path,
    target_dir: &Utf8Path,
    keep_names: bool,
) -> Result<Utf8PathBuf> {
    let new_stem: std::borrow::Cow<_> = if keep_names {
        original
            .file_stem()
            .with_context(|| format!("Invalid file stem for path {}", original))?
            .into()
    } else {
        let img = samepic::image::ImageData::load(original)?;
        img.timestamp.format("%FT%H-%M-%S").to_string().into()
    };

    let extension = original
        .extension()
        .with_context(|| format!("Missing file extension for path {}", original))?;

    let mut new_name = format!("{new_stem}.{extension}");
    let mut same_name_count = 0;

    let mut link = target_dir.to_owned();
    link.push(&new_name);

    while link.exists() {
        same_name_count += 1;
        new_name = format!("{new_stem}-{same_name_count}.{extension}");
        link.set_file_name(new_name);
    }

    Ok(link)
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
    Sort {
        /// Source folder to be sorted.
        #[clap(value_parser = dir)]
        source: Utf8PathBuf,
        /// Destination to sort the pictures into. If it does not exist, it will be created. Defaults to `source`-sorted.
        #[clap(short, long, value_parser)]
        destination: Option<Utf8PathBuf>,
        /// Program to open the picture folders with. Defaults to the default folder explorer.
        #[clap(short, long, value_parser = program)]
        opener: Option<PathBuf>,
        /// Do not attempt to open image folders after sorting.
        #[clap(short, long, value_parser)]
        no_open: bool,
    },
    /// Run the opener program without rerunning the sorting process.
    Open {
        /// Source folder to be sorted.
        #[clap(value_parser = dir)]
        path: Utf8PathBuf,
        /// Program to open the picture folders with. Defaults to the default folder explorer.
        #[clap(short, long, value_parser = program)]
        opener: Option<PathBuf>,
    },
    /// Collects all remaining images back into one folder after manual sorting is finished
    Collect {
        /// Source folder with the sorted images.
        #[clap(value_parser = dir)]
        source: Utf8PathBuf,
        /// Destination to collect the pictures into. If it does not exist, it will be created. Defaults to `source`-final.
        #[clap(short, long, value_parser)]
        destination: Option<Utf8PathBuf>,
        /// Do not delete the source image folders after collection.
        #[clap(short, long, value_parser)]
        no_delete: bool,
        /// Do not rename the source images during collection.
        #[clap(short, long, value_parser)]
        keep_names: bool,
    },
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

fn create_dir_from_ref_name(
    dir: Option<Utf8PathBuf>,
    base: &Utf8Path,
    name_suffix: &str,
) -> Result<Utf8PathBuf> {
    let dir = dir.unwrap_or_else(|| {
        let mut dest = base.to_owned();
        match base.file_name() {
            Some(folder) => dest.set_file_name(format!("{folder}-{name_suffix}")),
            None => dest.set_file_name(name_suffix),
        }
        dest
    });
    std::fs::create_dir_all(&dir).wrap_err_with(|| format!("Cannot create directory {}.", dir))?;
    match std::fs::read_dir(&dir)?.next() {
        Some(_) => Err(eyre!("Target directory not empty."))
            .suggestion("Pass an empty or non-existent target directory."),
        None => Ok(dir),
    }
}
