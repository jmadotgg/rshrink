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
    fn new(width: usize, height: usize) -> Dimensions {
        Dimensions { width, height }
    }
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

// Targeted terminal command;
//magick \
//        $fullFileName \
//        -sampling-factor 4:2:0 \
//        -strip \
//        -quality 85 \
//        -interlace Plane \
//        -gaussian-blur 0.05 \
//        -colorspace RGB \
//        $newFilePath

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    dimensions: Option<String>,

    #[clap(short, long)]
    gaussian_blur: Option<bool>,

    #[clap(short, long, default_value = "85")]
    compression_quality: usize,

    #[clap(default_value = DEFAULT_REGEX)]
    files: String,

    #[clap(default_value = DEFAULT_IN_DIR)]
    in_dir: String,

    #[clap(default_value = DEFAULT_OUT_DIR)]
    out_dir: String,
}

fn main() {
    let Args {
        in_dir,
        out_dir,
        files,
        dimensions,
        gaussian_blur,
        compression_quality,
    } = Args::parse();

    println!(
        " In Directory: {}\n Out Directory: {}\n Dimensions: {:?}\n File Regex: {}\n Compression quality: {}\n Gaussian blur: {:?}\n",
        in_dir, out_dir, dimensions, files, compression_quality, gaussian_blur
    );

    match create_dir_if_not_exists(&out_dir) {
        Ok(()) => (),
        Err(err) => panic!("Failed to create directory! {err}"),
    }

    // TODO: Print Error to console (More elegant way than using match?)
    let all_files = list_files(&in_dir).expect("Failed to read files!");

    let selected_files = all_files
        .iter()
        .cloned()
        .filter(|f| match f.to_str() {
            Some(file_name) => parse_file(&files, file_name),
            None => false,
        })
        .collect::<Vec<_>>();

    println!("Matched files {:?}\n", selected_files);

    let dims = match &dimensions {
        Some(d) => Dimensions::parse_dimensions(&d).expect("Failed to parse dimensions!"),
        None => Dimensions::new(1920, 1080),
    };

    let apply_gaussian_blur = match gaussian_blur {
        Some(v) => v,
        None => false,
    };

    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut wand = MagickWand::new();

    println!("Progress:");

    let file_count = selected_files.len();
    for (i, file) in selected_files.iter().enumerate() {
        println!(
            "=> {}% [{:?}]",
            ((i as f32 / file_count as f32) * 100.0).floor(),
            file
        );

        // https://stackoverflow.com/questions/34837011/how-to-clear-the-terminal-screen-in-rust-after-a-new-line-is-printed
        // print!("\x1B[2J\x1B[1;1H");
        // print!("{esc}c", esc = 27 as char);

        if let Some(file_name) = file.to_str() {
            match shrink(
                &mut wand,
                file_name,
                &out_dir,
                &dims,
                compression_quality,
                apply_gaussian_blur,
            ) {
                Ok(()) => (),
                Err(err) => eprintln!("Failed to shrink file {}! : {}", file_name, err),
            };
        }
    }
}

fn shrink(
    wand: &mut MagickWand,
    file_name: &str,
    out_dir: &str,
    dims: &Dimensions,
    compression_quality: usize,
    apply_gaussian_blur: bool,
) -> Result<(), MagickError> {
    wand.read_image(&file_name)?;
    wand.fit(dims.width, dims.height);

    wand.set_sampling_factors(&[4.0, 2.0, 0.0])?;
    wand.strip_image()?;
    wand.set_image_compression_quality(compression_quality)?;
    // 3 = Plane (build.rs)
    wand.set_interlace_scheme(3)?;

    if apply_gaussian_blur {
        // Pretty slow
        wand.gaussian_blur_image(0.05, 1.0)?
    }

    let new_file = format!("{}/sm_{}", out_dir, file_name);
    wand.write_image(new_file.as_str())
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
