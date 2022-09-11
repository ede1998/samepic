use std::{fmt::Display, io::Cursor};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{NaiveDate, NaiveDateTime};
use image::io::Reader;
use image_hasher::{HasherConfig, ImageHash};
use thiserror::Error;

type HashType = [u8; 16];

pub struct ImageData {
    data: Vec<u8>,
    path: Utf8PathBuf,
    pub timestamp: NaiveDateTime,
}

impl ImageData {
    pub fn load(path: &Utf8Path) -> Result<Self, ImageLoadError> {
        let file = std::fs::read(&path)?;

        let timestamp = match parse_time_stamp(&file) {
            Some(ts) => ts,
            None => {
                let meta = std::fs::metadata(&path)?;
                let fallback: chrono::DateTime<chrono::Local> =
                    meta.created().or_else(|_| meta.accessed())?.into();
                fallback.naive_local()
            }
        };

        Ok(ImageData {
            path: path.into(),
            timestamp,
            data: file,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    path: Utf8PathBuf,
    pub timestamp: NaiveDateTime,
    pub hash: ImageHash<HashType>,
}

impl Image {
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn load(path: &Utf8Path) -> Result<Self, ImageLoadError> {
        let image_data = ImageData::load(path)?;

        let base_image = {
            let file_cursor = Cursor::new(&image_data.data);
            Reader::new(file_cursor).with_guessed_format()?.decode()?
        };

        let hasher = HasherConfig::with_bytes_type::<HashType>()
            .hash_alg(image_hasher::HashAlg::Blockhash)
            .to_hasher();
        let hash = hasher.hash_image(&base_image);

        Ok(Image {
            path: image_data.path,
            timestamp: image_data.timestamp,
            hash,
        })
    }
}

impl std::hash::Hash for Image {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Eq for Image {}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Display for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Image {} at {}",
            self.path,
            self.timestamp.format("%A, %-d %B, %C%y")
        )
    }
}

#[derive(Debug, Error)]
pub enum ImageLoadError {
    #[error("failed to read image")]
    IoError(#[from] std::io::Error),
    #[error("invalid exif data")]
    InvalidExif(#[from] exif::Error),
    #[error("invalid image")]
    InvalidImage(#[from] image::error::ImageError),
}

fn parse_time_stamp(file: &[u8]) -> Option<NaiveDateTime> {
    use exif::Reader;
    let mut file_cursor = Cursor::new(file);
    let exif = Reader::new().read_from_container(&mut file_cursor).ok()?;
    let tags = [
        exif::Tag::DateTimeOriginal,
        exif::Tag::DateTime,
        exif::Tag::DateTimeDigitized,
    ];
    let ifds = [exif::In::PRIMARY, exif::In::THUMBNAIL];
    tags.into_iter()
        .flat_map(|tag| ifds.into_iter().map(move |ifd| (tag, ifd)))
        .find_map(|(tag, ifd)| {
            let f = exif.get_field(tag, ifd)?;
            let dt: String = match f.value {
                exif::Value::Ascii(ref a) => a
                    .iter()
                    .flat_map(|c| c.iter().copied().map(char::from))
                    .collect(),
                _ => return None,
            };
            let dt = exif::DateTime::from_ascii(dt.as_bytes()).ok()?;
            Some(
                NaiveDate::from_ymd(dt.year.into(), dt.month.into(), dt.day.into()).and_hms(
                    dt.hour.into(),
                    dt.minute.into(),
                    dt.second.into(),
                ),
            )
        })
}
