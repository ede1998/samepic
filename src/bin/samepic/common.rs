use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::{
    eyre::{eyre, Context},
    Help, Result,
};

pub fn create_dir_from_ref_name(
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

pub fn dir(s: &str) -> Result<Utf8PathBuf> {
    let meta = std::fs::metadata(s)?;
    meta.is_dir()
        .then(|| s.into())
        .ok_or_else(|| eyre!("Source is not a directory."))
}
