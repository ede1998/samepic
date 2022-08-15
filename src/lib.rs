use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{ContextCompat, Result};
use itertools::Itertools;

mod image;
mod pile;

use pile::Pile;
use rayon::prelude::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};

use crate::image::Image;

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
        .par_bridge()
        .filter(|(l, r)| l.hash.dist(&r.hash) < 20)
        .chain(images.par_iter().map(|i| (i,i)))
        .fold(|| Vec::with_capacity(images.len()), |mut piles: Vec<Pile>, (l, r)| {
            let left_pile = piles.iter().position(|pile| pile.pictures.contains(l));
            let right_pile = piles.iter().position(|pile| pile.pictures.contains(r));
            match (left_pile, right_pile) {
                (None, None) => {
                    let mut pile = Pile::new(r.clone());
                    pile.push(l.clone());
                    piles.push(pile);
                    let index = piles.len() - 1;
                    println!("Added picture {l} to pile {index}");
                    if l != r {
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
        }).flatten().collect();

        println!("{piles:?}");

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
