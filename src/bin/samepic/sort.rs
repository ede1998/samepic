use std::path::PathBuf;

use camino::Utf8PathBuf;
use clap::Args;
use color_eyre::Result;
use samepic::Repository;

use crate::common::{create_dir_from_ref_name, dir, program};
use crate::open::Open;

/// Starts grouping all the images in source into a destination folder
#[derive(Debug, Args)]
pub struct Sort {
    /// Source folder to be sorted
    #[clap(value_parser = dir)]
    source: Utf8PathBuf,
    /// Destination to sort the pictures into. If it does not exist, it will be created. Defaults to `source`-sorted
    #[clap(short, long, value_parser)]
    destination: Option<Utf8PathBuf>,
    /// Program to open the picture folders with. Defaults to the default folder explorer
    #[clap(short, long, value_parser = program)]
    opener: Option<PathBuf>,
    /// Do not attempt to open image folders after sorting
    #[clap(short, long, value_parser)]
    no_open: bool,
}

impl Sort {
    pub fn run(self) -> Result<()> {
        let destination = create_dir_from_ref_name(self.destination, &self.source, "sorted")?;
        let repo = Repository::new(self.source);
        repo.create_piles(&destination)?;
        if !self.no_open {
            Open::new(destination, self.opener).run()?;
        };
        Ok(())
    }
}
