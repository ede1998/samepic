//! Example: File Explorer
//! -------------------------
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.

use std::{collections::HashSet, io::Cursor};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{NaiveDate, NaiveDateTime};
use dioxus::prelude::*;
use image::{DynamicImage, GenericImageView};
use img_hash::HashBytes;
use itertools::Itertools;
use thiserror::Error;

fn main() {
    // simple_logger::init_with_level(log::Level::Debug).unwrap();
    dioxus::desktop::launch_cfg(APP, |c| c.with_window(|w| w.with_resizable(true)));
}

static APP: Component<()> = |cx| {
    let piles = use_ref(&cx, init);

    fn group_piles<'a>(
        piles: &'a [Pile],
    ) -> itertools::GroupBy<
        NaiveDate,
        std::vec::IntoIter<&'a Pile>,
        impl FnMut(&&'a Pile) -> NaiveDate,
    > {
        piles
            .iter()
            .sorted_by_key(|p| p.date())
            .group_by(|p| p.date)
    }

    rsx!(cx, div {
        link { href:"https://fonts.googleapis.com/icon?family=Material+Icons", rel:"stylesheet" }
        style { [include_str!("./style.css")] }
        header {
            i { class: "material-icons icon-menu", "menu" }
            span { }
            //i { class: "material-icons", onclick: move |_| files.write().go_up(), "logout" }
        }
        main {
            {
                group_piles(&piles.read()).into_iter().map(|(date, piles)| {
                    rsx!(crate::day_batch {
                            date: date,
                            piles: piles.map(|p| SimilarityPileProps { preview_path: p.preview().into(), image_count: p.len()}).collect(),
                        })
                })
            }
            // files.read().err.as_ref().map(|err| {
            //     rsx! (
            //         div {
            //             code { "{err}" }
            //             button { onclick: move |_| files.write().clear_err(), "x" }
            //         }
            //     )
            // })
        }
    })
};

#[derive(Debug, PartialEq, Props)]
struct DayBatchProps {
    date: NaiveDate,
    piles: Vec<SimilarityPileProps>,
}

fn day_batch(cx: Scope<DayBatchProps>) -> Element {
    let date = cx.props.date.format("%A, %-d. %B, %C%y");
    cx.render(rsx!(
        div {
            h1 { class: "batch-heading", "{date}" }
            ul {
                class: "flex-container",
                cx.props.piles.iter().cloned().map(|p| rsx!(li {
                    //key: "{&p.preview_picture}",
                    class: "flex-item",
                    crate::similarity_pile{..p }}))
            }
        }
    ))
}

#[derive(Debug, PartialEq, Props, Clone)]
struct SimilarityPileProps {
    preview_path: Utf8PathBuf,
    image_count: usize,
}

fn similarity_pile(cx: Scope<SimilarityPileProps>) -> Element {
    cx.render(rsx!(
    div {
        img {
            //onclick: move |_| files.write().enter_dir(dir_id),
            max_width: "100%",
            max_height: "100%",
            src: "{cx.props.preview_path}"
        }
        p { "{cx.props.image_count}" }
    }))
}

#[derive(Debug, Clone)]
struct Pile {
    pictures: HashSet<Image>,
    date: NaiveDate,
}

impl Pile {
    pub fn len(&self) -> usize {
        self.pictures.len()
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn preview(&self) -> &Utf8Path {
        &self.pictures.iter().next().expect("empty pile").path
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

fn init() -> Vec<Pile> {
    use img_hash::{HasherConfig, ImageHash};
    use walkdir::WalkDir;

    #[derive(Debug, Clone)]
    struct HashedImage {
        pub image: Image,
        pub hash: ImageHash<[u8; 8]>,
    }

    impl std::fmt::Display for HashedImage {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.image.path)
        }
    }

    let images: Vec<_> = WalkDir::new("samples")
        .into_iter()
        .filter_map(|e| {
            e.ok()
                .filter(|e| e.file_type().is_file())
                .and_then(|e| e.into_path().try_into().ok())
        })
        .filter_map(|path| {
            let hasher = HasherConfig::with_bytes_type::<[u8; 8]>().to_hasher();
            let image = Image::load(path)
                .map_err(|(path, err)| {
                    eprintln!("Failed to load image {path}: {err}");
                    err
                })
                .ok()?;
            let hash = image.hash(&hasher);
            Some(HashedImage { image, hash })
        })
        .collect();

    println!("Loaded {} images.", images.len());

    let piles: Vec<Pile> = images
        .iter()
        .tuple_combinations::<(_, _)>()
        .filter(|(l, r)| l.hash.dist(&r.hash) < 20)
        .chain(images.iter().map(|i| (i,i)))
        .fold(Vec::with_capacity(images.len()), |mut piles, (l, r)| {
            let left_pile = piles.iter().position(|pile| pile.pictures.contains(&l.image));
            let right_pile = piles.iter().position(|pile| pile.pictures.contains(&r.image));
            match (left_pile, right_pile) {
                (None, None) => {
                    let mut pile = Pile::new(r.image.clone());
                    pile.push(l.image.clone());
                    piles.push(pile);
                    let index = piles.len() - 1;
                    println!("Added picture {l} to pile {index}");
                    if l.image != r.image {
                        println!("Added picture {r} to pile {index}");
                    }
                },
                (Some(pile), None) => {
                    piles[pile].push(r.image.clone());
                    println!("Added picture {r} to pile {pile} because it also contains picture {l}");
                },
                (None, Some(pile)) => {
                    piles[pile].push(l.image.clone());
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

    piles
}

#[derive(Clone)]
struct Image {
    path: Utf8PathBuf,
    pixels: DynamicImage,
    timestamp: NaiveDateTime,
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

impl Image {
    pub fn load(path: Utf8PathBuf) -> Result<Self, (Utf8PathBuf, ImageLoadError)> {
        let load = || {
            use image::io::Reader;
            let file = std::fs::read(&path)?;
            let pixels = {
                let file_cursor = Cursor::new(&file);
                Reader::new(file_cursor).with_guessed_format()?.decode()?
            };
            let exif = {
                let mut file_cursor = Cursor::new(&file);
                exif::Reader::new().read_from_container(&mut file_cursor)?
            };
            let tags = [
                exif::Tag::DateTimeOriginal,
                exif::Tag::DateTime,
                exif::Tag::DateTimeDigitized,
            ];
            let ifds = [exif::In::PRIMARY, exif::In::THUMBNAIL];
            let timestamp = tags
                .into_iter()
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
                        NaiveDate::from_ymd(dt.year.into(), dt.month.into(), dt.day.into())
                            .and_hms(dt.hour.into(), dt.minute.into(), dt.second.into()),
                    )
                })
                .unwrap_or(NaiveDateTime::MAX);

            Ok((pixels, timestamp))
        };
        match load() {
            Ok((pixels, timestamp)) => Ok(Self {
                path,
                pixels,
                timestamp,
            }),
            Err(err) => Err((path, err)),
        }
    }

    pub fn hash<B: HashBytes>(&self, hasher: &img_hash::Hasher<B>) -> img_hash::ImageHash<B> {
        hasher.hash_image(&self.pixels)
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

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        struct Helper<'a>(&'a exif::Exif);
        impl<'a> std::fmt::Debug for Helper<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let mut list = f.debug_list();
                list.entries(self.0.fields().take(5));
                if self.0.fields().len() > 5 {
                    list.entry(&"...");
                }
                list.finish()
            }
        }
        struct HelperPixels<'a>(&'a DynamicImage);
        impl<'a> std::fmt::Debug for HelperPixels<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    f,
                    "DynamicImage {{ color: {:?}, height: {}, width: {} }}",
                    self.0.color(),
                    self.0.width(),
                    self.0.height()
                )
            }
        }
        f.debug_struct("Image")
            .field("path", &self.path)
            .field("pixels", &HelperPixels(&self.pixels))
            .field("timestamp", &self.timestamp)
            .finish()
    }
}
