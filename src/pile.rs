use std::collections::HashSet;

use chrono::NaiveDate;

use crate::image::Image;

#[derive(Debug, Clone)]
pub struct Pile {
    pub pictures: HashSet<Image>,
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

    pub fn new(image: Image) -> Self {
        Pile {
            date: image.timestamp.date(),
            pictures: HashSet::from([image]),
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
