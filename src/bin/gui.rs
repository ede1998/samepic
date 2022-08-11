//! Example: File Explorer
//! -------------------------
//!
//! This is a fun little desktop application that lets you explore the file system.
//!
//! This example is interesting because it's mixing filesystem operations and GUI, which is typically hard for UI to do.

// ![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use camino::Utf8PathBuf;
use chrono::NaiveDate;

use samepic::{group_piles, Repository};

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
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    repo: Repository,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            repo: Repository::new("./samples".into()),
        }
    }
}

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
        if let Some(_storage) = cc.storage {
            //return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
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
                let Repository {
                    ref mut images,
                    ref piles,
                } = self.repo;
                for (day, piles) in &group_piles(piles) {
                    ui.heading(format!("{}", day.format("%A, %-d %B, %C%y")));
                    ui.horizontal_wrapped(|ui| {
                        for pile in piles {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    let first = pile.preview();

                                    images
                                        .get_image(first)
                                        .show_max_size(ui, egui::vec2(100., 100.));
                                    ui.label(format!("{}", pile.len()));
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
