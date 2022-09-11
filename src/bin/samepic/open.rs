use std::{
    io::Write,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use clap::Args;
use color_eyre::Result;

use crate::common::{dir, program};

/// Run the opener program without rerunning the sorting process
#[derive(Debug, Args)]
pub struct Open {
    /// Source folder to be sorted
    #[clap(value_parser = dir)]
    path: Utf8PathBuf,
    /// Program to open the picture folders with. Defaults to the default folder explorer
    #[clap(short, long, value_parser = program)]
    opener: Option<PathBuf>,
}

impl Open {
    pub fn new(path: Utf8PathBuf, opener: Option<PathBuf>) -> Self {
        Self { path, opener }
    }

    pub fn run(self) -> Result<()> {
        let mut entries = self
            .path
            .read_dir()?
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort_unstable_by_key(|e| e.path());

        for dir in entries {
            let dir = dir.path();
            tracing::info!("Showing pile {}", dir.display());
            match self.opener {
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
