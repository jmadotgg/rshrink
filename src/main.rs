use clap::Parser;
use magick_rust::{magick_wand_genesis, MagickError, MagickWand};
use regex::Regex;
use std::{ffi::OsString, fs, io, sync::Once};

static START: Once = Once::new();
static DEFAULT_REGEX: &str = ".*.(jpg|png|JPG|PNG|JPEG|jpeg)";
static DEFAULT_IN_DIR: &str = ".";
static DEFAULT_OUT_DIR: &str = "_rshrinked";

#[derive(Debug)]
struct Dimensions {
    width: usize,
    height: usize,
}
impl Dimensions {
    fn parse_dimensions(dimensions: &str) -> Result<Dimensions, &str> {
        let d: Vec<&str> = dimensions.split("x").collect();
        if let [width, height] = d[..] {
            return Ok(Dimensions {
                width: width.parse::<usize>().expect("Invalid width!"),
                height: height.parse::<usize>().expect("Invalid height!"),
            });
        }
        Err("Invalid dimensions!")
    }
}
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    dimensions: Option<String>,

    #[clap(default_value = DEFAULT_REGEX)]
    files: String,

    #[clap(default_value = DEFAULT_IN_DIR)]
    in_dir: String,

    #[clap(default_value = DEFAULT_OUT_DIR)]
    out_dir: String,
}

impl Args {
    fn get_dimensions(&self) -> Option<Result<Dimensions, &str>> {
        if let Some(d) = &self.dimensions {
            let dimensions: Vec<&str> = d.split("x").collect();
            if let [width, height] = dimensions[..] {
                return Some(Ok(Dimensions {
                    width: width.parse::<usize>().expect("Invalid width!"),
                    height: height.parse::<usize>().expect("Invalid height!"),
                }));
            }
            return Some(Err("Invalid dimensions!"));
        }
        None
    }
}

fn main() {
    let Args {
        in_dir,
        out_dir,
        files,
        dimensions,
    } = Args::parse();
    // let dimensions = Dimensions::parse_dimensions(&dimensions.expect("Invalid dimensions!")).expect(msg);

    // let in_dir = &args.in_dir;
    // let out_dir = &args.out_dir;
    // let files = &args.files;

    println!(
        "In Directory: {}\nOut Directory: {}\nDimensions: {:?}\nFiles: {}",
        in_dir, out_dir, &dimensions, files
    );

    match create_dir_if_not_exists(&out_dir) {
        Ok(()) => (),
        Err(err) => panic!("Failed to create directory! {err}"),
    }

    // TODO: Print Error to console (More elegant way than using match?)
    let all_files = list_files(&in_dir).expect("Failed to read files!");

    println!("All files {:?}", all_files);

    let selected_files = all_files
        .iter()
        .cloned()
        .filter(|f| match f.to_str() {
            Some(file_name) => parse_file(&files, file_name),
            None => false,
        })
        .collect::<Vec<_>>();

    println!("Matched files {:?}", selected_files);

    if let Some(d) = dimensions {
        let d = Dimensions::parse_dimensions(&d).expect("Failed to parse dimensions!");
        for file in selected_files {
            if let Some(f) = file.to_str() {
                resize(f, &out_dir, &d).expect(&format!("Failed to resize file! {f}"));
            }
        }
    }
}

fn create_dir_if_not_exists(dir: &str) -> io::Result<()> {
    fs::create_dir(dir)?;
    Ok(())
}

fn parse_file(file_sel: &str, file_name: &str) -> bool {
    match Regex::new(file_sel) {
        Ok(reg) => reg.is_match(&format!(r"{file_name}")),
        Err(err) => {
            eprintln!("Failed to parse regular expression! {err}");
            false
        }
    }
}
fn resize(image_path: &str, out_dir: &str, dimensions: &Dimensions) -> Result<(), MagickError> {
    START.call_once(|| {
        magick_wand_genesis();
    });

    let wand = MagickWand::new();
    wand.read_image(image_path)?;
    wand.fit(dimensions.width, dimensions.height);
    let new_file = format!("{}/sm_{}", out_dir, image_path);
    wand.write_image(new_file.as_str())
}

fn list_files(path: &str) -> io::Result<Vec<OsString>> {
    let entries = fs::read_dir(path)?
        .filter_map(|res| match res {
            Ok(e) => Some(e.file_name()),
            Err(err) => {
                eprintln!("Failed to read file! {err}");
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(entries)
}
