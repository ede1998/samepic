use camino::{Utf8Path, Utf8PathBuf};
use clap::Args;
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};
use samepic::{ImageData, DATETIME_FORMATTER};

use crate::common::{create_dir_from_ref_name, dir};

/// Collects all remaining images back into one folder after manual sorting is finished
#[derive(Debug, Args)]
pub struct Collect {
    /// Source folder with the sorted images
    #[clap(value_parser = dir)]
    source: Utf8PathBuf,
    /// Destination to collect the pictures into. If it does not exist, it will be created. Defaults to `source`-final
    #[clap(short, long, value_parser)]
    destination: Option<Utf8PathBuf>,
    /// Do not delete the source image folders after collection
    #[clap(short, long, value_parser)]
    no_delete: bool,
    /// Do not rename the source images during collection
    #[clap(short, long, value_parser)]
    keep_names: bool,
}

impl Collect {
    pub fn run(self) -> Result<()> {
        let destination = create_dir_from_ref_name(self.destination, &self.source, "final")?;
        collect(&self.source, &destination, self.keep_names)?;
        if !self.no_delete {
            std::fs::remove_dir_all(&self.source)?;
        };
        Ok(())
    }
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
            .ok_or_else(|| eyre!("Invalid file stem for path {}", original))?
            .into()
    } else {
        let img = ImageData::load(original)?;
        img.timestamp.format(DATETIME_FORMATTER).to_string().into()
    };

    let extension = original
        .extension()
        .ok_or_else(|| eyre!("Missing file extension for path {}", original))?;

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
