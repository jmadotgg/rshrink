// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use clap::Parser;
use eframe::egui::{
    CentralPanel, Context, DroppedFile, Grid, Layout, RichText, ScrollArea, TopBottomPanel, Ui,
};
use eframe::epaint::{Pos2, Rect, Vec2};
use eframe::{egui, epi};
use magick_rust::magick_wand_genesis;
use regex::Regex;
use rshrink::filesystem::{create_dir_if_not_exists, filter_files, list_files};
use rshrink::imagemagick::shrink;
use rshrink::{threadpool::ThreadPool, utils::Dimensions};
use std::fmt::Debug;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::{Arc, Once};

static DEFAULT_REGEX: &str = r".*.(jpg|png|jpeg|JPG|PNG|JPEG)$";
static DEFAULT_IN_DIR: &str = ".";
static DEFAULT_OUT_DIR: &str = "_rshrinked";
static START: Once = Once::new();

const PADDING: f32 = 5.0;

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

#[derive(Default)]
struct RshrinkApp {
    selected_files: Vec<String>,
}

impl RshrinkApp {
    fn render_controls(self: &mut Self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if self.selected_files.len() > 0 && ui.button("Clear files").clicked() {
                self.selected_files.clear();
            };
            if ui.button("Select files").clicked() {
                if let Some(file_paths) = rfd::FileDialog::new().pick_files() {
                    self.selected_files = file_paths
                        .iter()
                        .map(|path_buf| path_buf.display().to_string())
                        .collect::<Vec<_>>();
                }
            };
        });
    }
    fn render_main(
        self: &mut Self,
        ui: &mut Ui,
        total_file_size: &mut u64,
        last_folder: &mut String,
    ) {
        if !self.selected_files.is_empty() {
            ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let mut files_to_remove_indexes = Vec::new();
                    for (i, file_path) in self.selected_files.iter().enumerate() {
                        let remove_file = render_file(ui, &file_path, total_file_size, last_folder);
                        if remove_file {
                            files_to_remove_indexes.push(i);
                        }
                    }
                    for i in files_to_remove_indexes {
                        self.selected_files.remove(i);
                    }
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Select files or drop them here");
            });
        }
        ui.add_space(10.);
    }
}
fn render_file(
    // self: &mut Self,
    ui: &mut Ui,
    file_path: &String,
    total_file_size: &mut u64,
    _last_folder: &mut String,
) -> bool {
    let file = File::open(&file_path).unwrap();
    let file_size = File::metadata(&file).unwrap().len();
    *total_file_size += file_size;

    let file_path_vec: Vec<&str> = file_path.split("/").collect();
    let count = file_path_vec.len();

    let mut remove_file = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(file_path_vec[count - 1]).strong())
            .on_hover_text_at_pointer(file_path);
        ui.with_layout(Layout::right_to_left(), |ui| {
            if ui.button("Deselect").clicked() {
                remove_file = true
            };
            ui.label(format!("{file_size} bytes"));
        });
    });
    ui.separator();
    remove_file
}

fn render_header(ui: &mut Ui) {
    ui.vertical_centered(|ui| ui.heading("Rshrink"));
    ui.separator();
}

fn render_footer(ctx: &Context, total_file_size: u64) {
    TopBottomPanel::bottom("footer").show(ctx, |ui| {
        ui.add_space(PADDING);
        ui.label(format!("Total file size: {} Kb", total_file_size / 1024));
        ui.add_space(PADDING);
    });
}

impl epi::App for RshrinkApp {
    fn name(&self) -> &str {
        "Rshrink file compression"
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        let mut total_file_size = 0;
        let mut last_folder = String::new();
        CentralPanel::default().show(ctx, |ui| {
            // Header
            render_header(ui);
            // Controls
            self.render_controls(ui);
            ui.separator();
            // Files to shrink
            self.render_main(ui, &mut total_file_size, &mut last_folder);
            ui.add_space(10.);
            // Footer
            render_footer(ctx, total_file_size);
        });
        self.detect_files_being_dropped(ctx);
    }
}

impl RshrinkApp {
    fn detect_files_being_dropped(&mut self, ctx: &egui::Context) {
        use egui::*;

        // Preview hovering files:
        if !ctx.input().raw.hovered_files.is_empty() {
            let mut text = "Dropping files:\n".to_owned();
            for file in &ctx.input().raw.hovered_files {
                if let Some(path) = &file.path {
                    text += &format!("\n{}", path.display());
                } else if !file.mime.is_empty() {
                    text += &format!("\n{}", file.mime);
                } else {
                    text += "\n???"
                }
            }
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.input().screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }

        // Collect dropped files:
        if !ctx.input().raw.dropped_files.is_empty() {
            self.selected_files = ctx
                .input()
                .raw
                .dropped_files
                .iter()
                .filter(|dropped_file| match &dropped_file.path {
                    Some(file) => {
                        let regex = Regex::new(DEFAULT_REGEX);
                        match regex {
                            Ok(regex) => regex.is_match(&file.display().to_string()),
                            Err(_) => false,
                        }
                    }
                    None => false,
                })
                .map(|dropped_file| match &dropped_file.path {
                    Some(file_path) => file_path.display().to_string(),
                    None => "???".to_owned(),
                })
                .collect::<_>();
        }
    }
}

fn main() {
    let app = RshrinkApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
    //    let Args {
    //        in_dir,
    //        out_dir,
    //        files,
    //        dimensions,
    //        gaussian_blur,
    //        compression_quality,
    //    } = Args::parse();
    //
    //    println!(
    //        " In Directory: {}\n Out Directory: {}\n Dimensions: {:?}\n File Regex: {}\n Compression quality: {}\n Gaussian blur: {:?}\n",
    //        in_dir, out_dir, dimensions, files, compression_quality, gaussian_blur
    //    );
    //
    //    match create_dir_if_not_exists(&out_dir) {
    //        Ok(()) => (),
    //        Err(err) => panic!("Failed to create directory! {err}"),
    //    }
    //
    //    // TODO: Print Error to console (More elegant way than using match?)
    //    let all_files = list_files(&in_dir).expect("Failed to read files!");
    //
    //    let selected_files = filter_files(all_files, &files);
    //
    //    println!("Matched files {:?}\n", selected_files);
    //
    //    let dims = match &dimensions {
    //        Some(d) => {
    //            Option::Some(Dimensions::parse_dimensions(&d).expect("Failed to parse dimensions!"))
    //        }
    //        None => None,
    //    };
    //
    //    let apply_gaussian_blur = match gaussian_blur {
    //        Some(v) => v,
    //        None => false,
    //    };
    //
    //    START.call_once(|| {
    //        magick_wand_genesis();
    //    });
    //
    //    let cpu_count = num_cpus::get();
    //    println!("Number of cpus: {}", cpu_count);
    //
    //    let in_dir = Arc::new(in_dir);
    //    let out_dir = Arc::new(out_dir);
    //    let dims = Arc::new(dims);
    //    let pool = ThreadPool::new(cpu_count);
    //
    //    for file in selected_files.iter() {
    //        // https://stackoverflow.com/questions/34837011/how-to-clear-the-terminal-screen-in-rust-after-a-new-line-is-printed
    //        // print!("\x1B[2J\x1B[1;1H");
    //        // print!("{esc}c", esc = 27 as char);
    //        let file = file.clone();
    //        let in_dir = Arc::clone(&in_dir);
    //        let out_dir = Arc::clone(&out_dir);
    //        let dims = Arc::clone(&dims);
    //
    //        pool.execute(move || {
    //            if let Some(file_name) = file.to_str() {
    //                match shrink(
    //                    file_name,
    //                    in_dir,
    //                    out_dir,
    //                    dims,
    //                    compression_quality,
    //                    apply_gaussian_blur.clone(),
    //                ) {
    //                    Ok(()) => (),
    //                    Err(err) => eprintln!("Failed to shrink file {}! : {}", file_name, err),
    //                };
    //            }
    //        });
    //    }
}
