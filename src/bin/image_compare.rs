use std::path::{Path, PathBuf};

use image::ImageError;
use image_hasher::{HasherConfig, ImageHash};
use tabled::{Table, Tabled};

#[derive(Tabled)]
struct HashedImage {
    #[tabled(display_with = "display_path")]
    pub path: PathBuf,
    #[tabled(display_with = "ImageHash::to_base64")]
    pub hash: ImageHash,
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use walkdir::WalkDir;

    let args: Vec<_> = std::env::args().collect();

    let hasher = HasherConfig::new().to_hasher();

    let images = WalkDir::new(&args[1])
        .into_iter()
        .filter_map(|e| {
            e.ok()
                .filter(|e| e.file_type().is_file())
                .map(|e| e.into_path())
        })
        .inspect(|e| println!("{}", e.display()))
        .map(|path| {
            let image = image::open(&path)?;
            let hash = hasher.hash_image(&image);
            Ok(HashedImage { path, hash })
        })
        .collect::<Result<Vec<_>, ImageError>>()?;

    println!("{}", Table::new(&images));

    println!("Hamming Distance");

    use tabled::builder::Builder;

    let mut builder = Builder::default();
    builder.set_columns(
        std::iter::once(String::new()).chain(images.iter().map(|i| i.path.display().to_string())),
    );
    for image1 in &images {
        let dists = images.iter().map(|i| i.hash.dist(&image1.hash).to_string());
        builder.add_record(std::iter::once(image1.path.display().to_string()).chain(dists));
    }
    let mut builder = builder.index();
    builder.set_index(0);
    let table = builder.build();
    println!("{}", table);

    Ok(())
}
