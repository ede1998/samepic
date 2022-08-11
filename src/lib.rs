use std::{collections::HashSet, io::Cursor};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{NaiveDate, NaiveDateTime};
use egui::ColorImage;
use egui_extras::RetainedImage;
use image::io::Reader;
use image::{DynamicImage, GenericImageView};
use image_hasher::{HasherConfig, ImageHash};
use itertools::Itertools;
use std::fmt::Display;
use thiserror::Error;

mod lru;
use crate::lru::{Key, LruCache};

pub struct Repository {
    pub piles: Vec<Pile>,
    pub images: ImageRepository,
}

pub struct ImageRepository {
    image_cache: LruCache<RetainedImage>,
    preview_cache: LruCache<RetainedImage>,
}

impl ImageRepository {
    pub fn get_image(&mut self, image: &Image) -> &RetainedImage {
        self.image_cache.get_or_insert(image.image, || {
            let inner = || {
                let file = std::fs::read(&image.path)?;
                let base_image = {
                    let file_cursor = Cursor::new(&file);
                    Reader::new(file_cursor).with_guessed_format()?.decode()?
                };

                Ok(to_retained(&base_image, &image.path))
            };
            inner()
                .map_err(|err: ImageLoadError| {
                    eprintln!("Failed to load image {}: {err}", image.path);
                })
                .unwrap_or_else(|_| {
                    RetainedImage::from_color_image(image.path.as_str(), ColorImage::example())
                })
        })
    }
}

impl Repository {
    fn load_single_image(
        &mut self,
        path: &Utf8Path,
        size: (u32, u32),
    ) -> Result<Image, ImageLoadError> {
        let file = std::fs::read(&path)?;

        let timestamp = parse_time_stamp(&file, NaiveDateTime::MAX);

        let base_image = {
            let file_cursor = Cursor::new(&file);
            Reader::new(file_cursor).with_guessed_format()?.decode()?
        };

        let hasher = HasherConfig::with_bytes_type::<[u8; 8]>().to_hasher();
        let hash = hasher.hash_image(&base_image);

        let preview = base_image.thumbnail(size.0, size.1);

        let image = to_retained(&base_image, path);
        let preview = to_retained(&preview, path);

        let preview = self.images.preview_cache.push(preview);
        let image = self.images.image_cache.push(image);

        Ok(Image {
            path: path.into(),
            image,
            preview,
            timestamp,
            hash,
        })
    }

    pub fn new(src: Utf8PathBuf) -> Self {
        use walkdir::WalkDir;

        let mut repo = Self {
            piles: vec![],
            images: ImageRepository {
                image_cache: LruCache::new(10),
                preview_cache: LruCache::new(1000),
            },
        };

        let images: Vec<_> = WalkDir::new(src)
            .into_iter()
            .filter_map(|e| {
                e.ok()
                    .filter(|e| e.file_type().is_file())
                    .and_then(|e| e.into_path().try_into().ok())
            })
            .filter_map(|path: Utf8PathBuf| {
                let image = repo
                    .load_single_image(&path, (100, 100))
                    .map_err(|err| {
                        eprintln!("Failed to load image {path}: {err}");
                        err
                    })
                    .ok()?;
                Some(image)
            })
            .collect();

        println!("Loaded {} images.", images.len());

        let piles: Vec<Pile> = images
        .iter()
        .tuple_combinations::<(_, _)>()
        .filter(|(l, r)| l.hash.dist(&r.hash) < 20)
        .chain(images.iter().map(|i| (i,i)))
        .fold(Vec::with_capacity(images.len()), |mut piles, (l, r)| {
            let left_pile = piles.iter().position(|pile| pile.pictures.contains(l));
            let right_pile = piles.iter().position(|pile| pile.pictures.contains(r));
            match (left_pile, right_pile) {
                (None, None) => {
                    let mut pile = Pile::new(r.clone());
                    pile.push(l.clone());
                    piles.push(pile);
                    let index = piles.len() - 1;
                    println!("Added picture {l} to pile {index}");
                    if l.image != r.image {
                        println!("Added picture {r} to pile {index}");
                    }
                },
                (Some(pile), None) => {
                    piles[pile].push(r.clone());
                    println!("Added picture {r} to pile {pile} because it also contains picture {l}");
                },
                (None, Some(pile)) => {
                    piles[pile].push(l.clone());
                    println!("Added picture {l} to pile {pile} because it also contains picture {r}");
                },
                (Some(i), Some(j)) => {
                    if i != j {
                        let disbanding_pile = piles.swap_remove(j);
                        piles[i].merge(disbanding_pile);
                        println!("Merged piles {i} and {j} to {i} because picture {l} belonged to pile {i} and picture {r} belonged to pile {j}");
                    }
                }
            }
            piles
        });

        println!("{piles:?}");

        repo.piles = piles;

        repo
    }
}

pub fn group_piles<'a>(
    piles: &'a [Pile],
) -> itertools::GroupBy<NaiveDate, std::vec::IntoIter<&'a Pile>, impl FnMut(&&'a Pile) -> NaiveDate>
{
    piles
        .iter()
        .sorted_by_key(|p| p.date())
        .group_by(|p| p.date)
}

fn to_retained(image: &DynamicImage, path: &Utf8Path) -> RetainedImage {
    use egui::Color32;
    use image::Rgba;
    fn color(rgba: Rgba<u8>) -> Color32 {
        egui::Color32::from_rgb(rgba.0[0], rgba.0[1], rgba.0[2])
    }

    let (w, h) = image.dimensions();
    let w = w.try_into().unwrap_or(usize::MAX);
    let h = h.try_into().unwrap_or(usize::MAX);

    let pixels = image
        .pixels()
        .take(w * h)
        .map(|(_x, _y, rgba)| color(rgba))
        .collect();
    let image = ColorImage {
        size: [w, h],
        pixels,
    };

    RetainedImage::from_color_image(path.as_str(), image)
}

fn parse_time_stamp(file: &[u8], fallback: NaiveDateTime) -> NaiveDateTime {
    use exif::Reader;
    let mut file_cursor = Cursor::new(file);
    let exif = match Reader::new().read_from_container(&mut file_cursor) {
        Ok(exif) => exif,
        Err(_) => return fallback,
    };
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
        .unwrap_or(fallback)
}

#[derive(Debug, Clone)]
pub struct Image {
    path: Utf8PathBuf,
    image: Key<RetainedImage>,
    #[allow(dead_code)] // TODO
    preview: Key<RetainedImage>,
    timestamp: NaiveDateTime,
    hash: ImageHash<[u8; 8]>,
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
enum ImageLoadError {
    #[error("failed to read image")]
    IoError(#[from] std::io::Error),
    #[error("invalid exif data")]
    InvalidExif(#[from] exif::Error),
    #[error("invalid image")]
    InvalidImage(#[from] image::error::ImageError),
}

#[derive(Debug, Clone)]
pub struct Pile {
    pictures: HashSet<Image>,
    date: NaiveDate,
}

impl Pile {
    #[allow(clippy::len_without_is_empty)] // piles may never be empty
    pub fn len(&self) -> usize {
        self.pictures.len()
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn preview(&self) -> &Image {
        self.pictures.iter().next().expect("empty pile")
    }

    pub fn new(image: Image) -> Self {
        Pile {
            pictures: HashSet::from([image]),
            date: NaiveDate::from_ymd(1998, 7, 21), // TODO
        }
    }

    pub fn push(&mut self, image: Image) {
        self.pictures.insert(image);
        self.update_date();
    }

    pub fn merge(&mut self, other: Pile) {
        self.pictures.extend(other.pictures);
        self.update_date();
    }

    fn update_date(&mut self) {
        self.date = self
            .pictures
            .iter()
            .map(|p| p.timestamp.date())
            .min()
            // any pile has at least one element
            .expect("computing pile date");
    }
}
