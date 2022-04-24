use clap::Parser;
use magick_rust::magick_wand_genesis;
use rshrink::filesystem::{create_dir_if_not_exists, list_files, parse_file};
use rshrink::imagemagick::shrink;
use rshrink::{threadpool::ThreadPool, utils::Dimensions};
use std::sync::{Arc, Once};

static DEFAULT_REGEX: &str = ".*.(jpg|png|JPG|PNG|JPEG|jpeg)";
static DEFAULT_IN_DIR: &str = ".";
static DEFAULT_OUT_DIR: &str = "_rshrinked";
static START: Once = Once::new();

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
        Some(d) => {
            Option::Some(Dimensions::parse_dimensions(&d).expect("Failed to parse dimensions!"))
        }
        None => None,
    };

    let apply_gaussian_blur = match gaussian_blur {
        Some(v) => v,
        None => false,
    };

    START.call_once(|| {
        magick_wand_genesis();
    });

    let cpu_count = num_cpus::get();
    println!("Number of cpus: {}", cpu_count);

    let in_dir = Arc::new(in_dir);
    let out_dir = Arc::new(out_dir);
    let dims = Arc::new(dims);
    let pool = ThreadPool::new(cpu_count);

    for file in selected_files.iter() {
        // https://stackoverflow.com/questions/34837011/how-to-clear-the-terminal-screen-in-rust-after-a-new-line-is-printed
        // print!("\x1B[2J\x1B[1;1H");
        // print!("{esc}c", esc = 27 as char);
        let file = file.clone();
        let in_dir = Arc::clone(&in_dir);
        let out_dir = Arc::clone(&out_dir);
        let dims = Arc::clone(&dims);

        pool.execute(move || {
            if let Some(file_name) = file.to_str() {
                match shrink(
                    file_name,
                    in_dir,
                    out_dir,
                    dims,
                    compression_quality,
                    apply_gaussian_blur.clone(),
                ) {
                    Ok(()) => (),
                    Err(err) => eprintln!("Failed to shrink file {}! : {}", file_name, err),
                };
            }
        });
    }
}
