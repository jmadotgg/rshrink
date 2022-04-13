use magick_rust::{magick_wand_genesis, MagickError, MagickWand};
use std::{ffi::OsString, fs, io, sync::Once};
static START: Once = Once::new();
fn main() {
    let mut dir = String::new();
    let mut files = String::new();

    println!("Enter folder (Default is current directory):");
    io::stdin()
        .read_line(&mut dir)
        .expect("Failed to read directory");

    println!("Enter files to resize: (Default *.jpg)");
    io::stdin()
        .read_line(&mut files)
        .expect("Failed to read files");

    let mut dir = dir.trim();
    if dir == "" {
        dir = ".";
    }

    let dest_dir = format!("{}/rshrinked", dir);
    fs::create_dir(&dest_dir).expect("Failed to create directory!");

    let mut files = files.trim();
    if files == "" {
        files = ".jpg";
    }

    let all_files = list_files(dir).expect("Failed to list files!");

    let selected_files = all_files
        .iter()
        .cloned()
        .filter(|f| match f.to_str() {
            Some(file) => file.contains(files),
            None => false,
        })
        .collect::<Vec<_>>();

    for file in selected_files {
        if let Some(f) = file.to_str() {
            resize(f, &dest_dir).expect("Failed to resize file");
        }
    }
}

fn resize(image_path: &str, dest_dir: &str) -> Result<(), MagickError> {
    START.call_once(|| {
        magick_wand_genesis();
    });

    let wand = MagickWand::new();
    wand.read_image("flower.jpg")?;
    wand.fit(1280, 720);
    let new_file = format!("{}/sm_{}", dest_dir, image_path);
    wand.write_image(new_file.as_str())
}

fn list_files(path: &str) -> io::Result<Vec<OsString>> {
    let entries = fs::read_dir(path)?
        .filter_map(|res| match res {
            Ok(e) => Some(e.file_name()),
            Err(_) => None,
        })
        .collect::<Vec<_>>();

    println!("listing files {:?}", entries);

    Ok(entries)
}
