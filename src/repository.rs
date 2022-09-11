use camino::{Utf8Path, Utf8PathBuf};
use chrono::Duration;
use color_eyre::eyre::{ContextCompat, Result};
use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};

use crate::image::Image;
use crate::pile::Pile;

pub struct Repository {
    pub piles: Vec<Pile>,
}

impl Repository {
    pub fn new(src: Utf8PathBuf) -> Self {
        use walkdir::WalkDir;

        let mut repo = Self { piles: vec![] };

        let images: Vec<_> = WalkDir::new(src)
            .into_iter()
            .par_bridge()
            .filter_map(|e| {
                e.ok()
                    .filter(|e| e.file_type().is_file())
                    .and_then(|e| e.into_path().try_into().ok())
            })
            .filter_map(|path: Utf8PathBuf| {
                let image = Image::load(&path)
                    .map_err(|err| {
                        tracing::error!("Failed to load image {path}: {err}");
                        err
                    })
                    .ok()?;
                Some(image)
            })
            .collect();

        tracing::info!("Loaded {} images.", images.len());

        let piles: Vec<_> = images
            .iter()
            .tuple_combinations::<(_, _)>()
            .par_bridge()
            .filter(|(l, r)| {
                let time_delta = abs(l.timestamp - r.timestamp);
                time_delta < chrono::Duration::minutes(30) && l.hash.dist(&r.hash) < 10
            })
            .chain(images.par_iter().map(|i| (i, i)))
            .collect();

        let piles = piles.into_iter().fold(Vec::with_capacity(images.len()), |mut piles: Vec<Pile>, (l, r)| {
            let left_pile = piles.iter().position(|pile| pile.pictures.contains(l));
            let right_pile = piles.iter().position(|pile| pile.pictures.contains(r));
            match (left_pile, right_pile) {
                (None, None) => {
                    let mut pile = Pile::new(r.clone());
                    pile.push(l.clone());
                    piles.push(pile);
                    let index = piles.len() - 1;
                    tracing::debug!("Added picture {l} to pile {index}");
                    if l != r {
                        tracing::debug!("Added picture {r} to pile {index}");
                    }
                },
                (Some(pile), None) => {
                    piles[pile].push(r.clone());
                    tracing::debug!("Added picture {r} to pile {pile} because it also contains picture {l}");
                },
                (None, Some(pile)) => {
                    piles[pile].push(l.clone());
                    tracing::debug!("Added picture {l} to pile {pile} because it also contains picture {r}");
                },
                (Some(i), Some(j)) => {
                    if i != j {
                        let disbanding_pile = piles.swap_remove(j);
                        // swap_remove moved the pile we want to retain if i == max index -> now it is at index j
                        let pile_index = if i == piles.len() { j } else { i };
                        piles[pile_index].merge(disbanding_pile);
                        tracing::debug!("Merged piles {i} and {j} to {i} because picture {l} belonged to pile {i} and picture {r} belonged to pile {j}");
                    }
                }
            }
            piles
        });

        tracing::trace!("{piles:#?}");

        print_stats(&piles);

        repo.piles = piles;

        repo
    }

    pub fn create_piles(&self, dest: &Utf8Path) -> Result<()> {
        use std::collections::HashMap;
        use std::fs;
        let mut dates_counts = HashMap::with_capacity(self.piles.len());
        for pile in &self.piles {
            let n: usize = *dates_counts
                .entry(pile.date())
                .and_modify(|e| *e += 1)
                .or_default();
            let timestamp = format!("{}_{n:04}", pile.date());
            let dir = dest.join(timestamp);
            fs::create_dir(&dir)?;
            for image in &pile.pictures {
                let mut link = dir.clone();
                link.push(image.path().file_name().wrap_err_with(|| {
                    format!("Invalid image file name for path {}", image.path())
                })?);
                fs::hard_link(image.path(), link)?;
            }
        }
        Ok(())
    }
}

fn abs(duration: Duration) -> Duration {
    if duration < Duration::zero() {
        -duration
    } else {
        duration
    }
}

fn print_stats(piles: &[Pile]) {
    let longest_time_delta = piles
        .iter()
        .map(|p| {
            use itertools::MinMaxResult;
            match p.pictures.iter().map(|image| image.timestamp).minmax() {
                MinMaxResult::NoElements | MinMaxResult::OneElement(_) => chrono::Duration::zero(),
                MinMaxResult::MinMax(min, max) => max - min,
            }
        })
        .max()
        .unwrap_or_else(Duration::zero)
        .num_minutes();
    let total_pics: usize = piles.iter().map(|p| p.pictures.len()).sum();
    let total_piles = piles.len();
    let max_pile_size = piles
        .iter()
        .map(|p| p.pictures.len())
        .max()
        .unwrap_or_default();
    let avg_pile_size = total_pics as f32 / total_piles as f32;
    let sorted_piles: Vec<_> = piles
        .iter()
        .map(|p| p.pictures.len())
        .sorted_unstable()
        .collect();
    let median_pile_size = sorted_piles[sorted_piles.len() / 2];
    tracing::info!("===== PILE STATS =====");
    tracing::info!("Image count: {total_pics}");
    tracing::info!("Pile count: {total_piles}");
    tracing::info!("Pile size (Avg/Med/Max): {avg_pile_size}/{median_pile_size}/{max_pile_size}");
    tracing::info!("Longest time delta: {longest_time_delta}min");
}
