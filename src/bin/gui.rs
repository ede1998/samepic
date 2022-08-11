//! Example: File Explorer
//! -------------------------
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.

// ![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{collections::HashSet, io::Cursor};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{NaiveDate, NaiveDateTime};
use egui_extras::RetainedImage;
use image::{DynamicImage, GenericImageView};
use image_hasher::{HashBytes, ImageHash};
use itertools::Itertools;
use thiserror::Error;

fn group_piles<'a>(
    piles: &'a [Pile],
) -> itertools::GroupBy<NaiveDate, std::vec::IntoIter<&'a Pile>, impl FnMut(&&'a Pile) -> NaiveDate>
{
    piles
        .iter()
        .sorted_by_key(|p| p.date())
        .group_by(|p| p.date)
}

#[derive(Debug, PartialEq)]
struct DayBatchProps {
    date: NaiveDate,
    piles: Vec<SimilarityPileProps>,
}

#[derive(Debug, PartialEq, Clone)]
struct SimilarityPileProps {
    preview_path: Utf8PathBuf,
    image_count: usize,
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
    use image_hasher::HasherConfig;
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
    retained_image: std::rc::Rc<RetainedImage>,
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
        match load().and_then(|(pixels, timestamp)| {
            let image = RetainedImage::from_image_bytes(path.as_str(), pixels.as_bytes())
                .map_err(ImageLoadError::ImageConversionError)?;
            Ok((pixels, timestamp, image))
        }) {
            Ok((pixels, timestamp, image)) => Ok(Self {
                retained_image: image.into(),
                path,
                pixels,
                timestamp,
            }),
            Err(err) => Err((path, err)),
        }
    }

    pub fn hash<B: HashBytes>(
        &self,
        hasher: &image_hasher::Hasher<B>,
    ) -> image_hasher::ImageHash<B> {
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
    #[error("failed to convert image")]
    ImageConversionError(String),
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

// When compiling natively:
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(TemplateApp::new(cc))),
    );
}
// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    images: Vec<Pile>,
}

// impl Default for TemplateApp {
//     fn default() -> Self {
//         Self {
//             data: vec![
//                 ("Wednesday, 23rd January 2022".into(), vec![6, 1, 5, 16, 1]),
//                 ("Thursday, 24th January 2022".into(), vec![9, 51]),
//                 ("Tuesday, 18th February 2022".into(), vec![88]),
//                 ("Monday, 17th July 2022".into(), vec![0, 17, 32]),
//             ],
//             image: None,
//         }
//     }
// }

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let dark_mode = cc.integration_info.prefer_dark_mode.unwrap_or(true);
        if dark_mode {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
        }

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            //return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        TemplateApp { images: init() }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (day, piles) in &group_piles(&self.images) {
                    ui.heading(format!("{}", day.format("%A, %-d %B, %C%y")));
                    ui.horizontal_wrapped(|ui| {
                        for pile in piles {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    let first = pile.pictures.iter().next().unwrap();

                                    first
                                        .retained_image
                                        .show_max_size(ui, egui::vec2(100., 100.));
                                    ui.label(format!("{}", pile.pictures.len()));
                                });
                            });
                        }
                    });
                }
                ui.set_width(ui.available_width());
            });
        });
    }
}
