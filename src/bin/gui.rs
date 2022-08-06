//! Example: File Explorer
//! -------------------------
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.

use std::{
    collections::HashSet,
    io::Cursor,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use dioxus::prelude::*;
use image::{DynamicImage, GenericImageView};
use itertools::Itertools;
use thiserror::Error;

fn main() {
    // simple_logger::init_with_level(log::Level::Debug).unwrap();
    dioxus::desktop::launch_cfg(APP, |c| c.with_window(|w| w.with_resizable(true)));
}

static APP: Component<()> = |cx| {
    let piles = use_ref(&cx, init);

    rsx!(cx, div {
        link { href:"https://fonts.googleapis.com/icon?family=Material+Icons", rel:"stylesheet" }
        style { [include_str!("./style.css")] }
        header {
            i { class: "material-icons icon-menu", "menu" }
            span { }
            //i { class: "material-icons", onclick: move |_| files.write().go_up(), "logout" }
        }
        main {
            crate::day_batch {
                date: NaiveDate::from_ymd(2016, 7, 21),
                piles: piles.read().iter().map(|p| SimilarityPileProps { preview_path: p.preview().to_string_lossy().into(), image_count: p.len()}).collect(),
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
        h1 { "{date}" }
        ul {
            class: "flex-container",
        cx.props.piles.iter().cloned().map(|p| rsx!(li {
            //key: "{&p.preview_picture}",
            class: "flex-item",
            crate::similarity_pile{..p }}))
        }
    ))
}

#[derive(Debug, PartialEq, Props, Clone)]
struct SimilarityPileProps {
    preview_path: String,
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

#[derive(Debug)]
struct Pile {
    pictures: HashSet<Image>,
    date: NaiveDate,
}

impl Pile {
    pub fn len(&self) -> usize {
        self.pictures.len()
    }

    pub fn preview(&self) -> &Path {
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
    }

    pub fn merge(&mut self, other: Pile) {
        self.pictures.extend(other.pictures);
    }
}

fn init() -> Vec<Pile> {
    use img_hash::{HasherConfig, ImageHash};
    use walkdir::WalkDir;
    let hasher = HasherConfig::new().to_hasher();

    #[derive(Debug, Clone)]
    struct HashedImage {
        pub image: Image,
        pub hash: ImageHash,
    }

    let images: Vec<_> = WalkDir::new("samples")
        .into_iter()
        .filter_map(|e| {
            e.ok()
                .filter(|e| e.file_type().is_file())
                .map(|e| e.into_path())
        })
        .filter_map(|path| {
            let image = Image::load(path)
                .map_err(|(path, err)| {
                    eprintln!("Failed to load image {}: {err}", path.display());
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
                    println!("Added picture {} to pile {}", path(l), piles.len() - 1);
                    if l.image != r.image {
                        println!("Added picture {} to pile {}", path(r), piles.len() - 1);
                    }
                },
                (Some(pile), None) => {
                    piles[pile].push(r.image.clone());
                    println!("Added picture {} to pile {pile} because it also contains picture {}", path(r), path(l));
                },
                (None, Some(pile)) => {
                    piles[pile].push(l.image.clone());
                    println!("Added picture {} to pile {pile} because it also contains picture {}", path(l), path(r));
                },
                (Some(left_pile), Some(right_pile)) => {
                    if left_pile == right_pile {
                        return piles;
                    }
                    let disbanding_pile = piles.swap_remove(right_pile);
                    piles[left_pile].merge(disbanding_pile);
                    println!("Merged piles {left_pile} and {right_pile} to {left_pile} because picture {} belonged to pile {left_pile} and picture {} belonged to pile {right_pile}", path(l), path(r));
                }
            }

            fn path(image: &HashedImage) -> std::path::Display {
                image.image.path.display()
            }

            piles
        });

    println!("{piles:?}");

    piles
}

#[derive(Clone)]
struct Image {
    path: PathBuf,
    pixels: DynamicImage,
    exif: std::rc::Rc<exif::Exif>,
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
    pub fn load(path: PathBuf) -> Result<Self, (PathBuf, ImageLoadError)> {
        let load = || {
            use image::io::Reader;
            let file = std::fs::read(&path)?;
            let pixels = {
                let file_cursor = Cursor::new(&file);
                Reader::new(file_cursor).with_guessed_format()?.decode()?
            };
            let exif = {
                let mut file_cursor = Cursor::new(&file);
                exif::Reader::new()
                    .read_from_container(&mut file_cursor)?
                    .into()
            };
            Ok((pixels, exif))
        };
        match load() {
            Ok((pixels, exif)) => Ok(Self { path, pixels, exif }),
            Err(err) => Err((path, err)),
        }
    }

    pub fn hash(&self, hasher: &img_hash::Hasher) -> img_hash::ImageHash {
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
            .field("exif", &Helper(&self.exif))
            .finish()
    }
}
