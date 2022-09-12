use std::{
    io::Write,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use clap::Args;
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};

use crate::common::dir;

#[derive(Debug, Args)]
pub struct OpenOptions {
    /// Program to open the picture folders with. Defaults to the default folder explorer
    #[clap(short, long, value_parser = program)]
    opener: Option<PathBuf>,
    /// Skip folders with only a single image
    #[clap(short, long)]
    skip_singles: bool,
}

/// Run the opener program without rerunning the sorting process
#[derive(Debug, Args)]
pub struct Open {
    /// Source folder to be sorted
    #[clap(value_parser = dir)]
    path: Utf8PathBuf,
    /// Skip all piles until the given pile
    #[clap(short = 'a', long)]
    start_at: Option<PathBuf>,
    #[clap(flatten)]
    options: OpenOptions,
}

impl Open {
    pub fn new(path: Utf8PathBuf, options: OpenOptions) -> Self {
        Self {
            path,
            start_at: None,
            options,
        }
    }

    pub fn run(self) -> Result<()> {
        let mut entries = self
            .path
            .read_dir()?
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort_unstable_by_key(|e| e.path());

        let start = match self.start_at {
            Some(start_at) => entries
                .iter()
                .position(|entry| entry.path() == start_at)
                .ok_or_else(|| eyre!("Failed to find pile {}", start_at.display()))?,
            None => 0,
        };

        for dir in entries.into_iter().skip(start) {
            let dir = dir.path();

            if !dir.is_dir() {
                tracing::debug!("Skipping {} because it is not a directory", dir.display());
                continue;
            }

            if self.options.skip_singles && only_single_file(&dir) {
                tracing::debug!(
                    "Skipping pile {} because it only has one image",
                    dir.display()
                );
                continue;
            }

            tracing::info!("Showing pile {}", dir.display());

            match self.options.opener {
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

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually
    stdout
        .write_all(b"Press enter to continue...")
        .expect("failed to write to stdout");
    stdout.flush().expect("failed to flush stdout");

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).unwrap();
}

fn program(s: &str) -> Result<PathBuf> {
    use which::which;

    which(s).wrap_err_with(|| format!("Opener {s} is not a valid executable."))
}

fn only_single_file(dir: &Path) -> bool {
    match dir.read_dir() {
        Ok(dir) => !dir.enumerate().any(|(i, _)| i >= 1),
        Err(e) => {
            tracing::warn!("Failed read directory {}: {e}", dir.display());
            false
        }
    }
}
