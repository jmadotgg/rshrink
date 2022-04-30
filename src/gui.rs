use eframe::{
    egui::{
        self, CentralPanel, Context, Id, LayerId, Layout, Order, RichText, ScrollArea, TextStyle,
        TopBottomPanel, Ui,
    },
    emath::Align2,
    epaint::Color32,
    epi,
};
use magick_rust::magick_wand_genesis;
use regex::Regex;
use std::{
    fs::File,
    sync::{
        mpsc::{self, Receiver},
        Arc, Once,
    },
};

use crate::{
    filesystem::create_dir_if_not_exists, imagemagick::perform_magick, threadpool::ThreadPool,
    utils::Dimensions,
};

static START: Once = Once::new();
const DEFAULT_OUT_DIR: &str = "_rshrinked";
const DEFAULT_REGEX: &str = r".*.(jpg|png|jpeg|JPG|PNG|JPEG)$";
const PADDING: f32 = 5.0;

#[derive(Clone)]
struct SelectedFile {
    path: String,
    parent_folder: String,
    name: String,
    size: u64,
    done: bool,
}

impl SelectedFile {
    fn new(path: String) -> SelectedFile {
        let path_vec = path.split("/").collect::<Vec<_>>();
        let count = path_vec.len();

        let file = File::open(&path).expect("Failed to open file");
        let file_size = File::metadata(&file).unwrap().len();

        SelectedFile {
            path: path.clone(),
            parent_folder: path_vec[0..count - 1].join("/"),
            name: path_vec[count - 1].to_string(),
            size: file_size,
            done: false,
        }
    }
}
pub struct RshrinkApp {
    selected_files: Vec<SelectedFile>,
    file_dimensions: Dimensions,
    thread_pool: ThreadPool,
    receiver: Option<Receiver<usize>>,
}

impl epi::App for RshrinkApp {
    fn name(&self) -> &str {
        "Rshrink file compression"
    }

    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Fill thread pool
        let pool = ThreadPool::new(num_cpus::get());
        self.thread_pool = pool;
        // Init imagemagick
        START.call_once(|| {
            magick_wand_genesis();
        });
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        let mut total_file_size = 0;
        let mut last_folder = String::new();
        if let Some(receiver) = &self.receiver {
            if let Ok(i) = receiver.recv() {
                self.selected_files[i].done = true;
            }
        }
        // Footer (first, because of CentralPanel filling the remaininng space)
        render_footer(ctx, total_file_size, self.selected_files.len());
        CentralPanel::default().show(ctx, |ui| {
            // Header
            render_header(ui);
            // Controls
            self.render_controls(ui);
            ui.separator();
            // Files to shrink
            self.render_main(ui, &mut total_file_size, &mut last_folder);
        });
        self.detect_files_being_dropped(ctx);
    }
}
impl RshrinkApp {
    pub fn new() -> Self {
        Self {
            selected_files: Default::default(),
            file_dimensions: Default::default(),
            thread_pool: Default::default(),
            receiver: None,
        }
    }
    pub fn render_controls(self: &mut Self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if self.selected_files.len() > 0 && ui.button("Clear files").clicked() {
                self.selected_files.clear();
            };
            if ui.button("Select files").clicked() {
                if let Some(file_paths) = rfd::FileDialog::new().pick_files() {
                    self.selected_files = file_paths
                        .iter()
                        .map(|path_buf| SelectedFile::new(path_buf.display().to_string()))
                        .collect::<Vec<_>>();
                }
            };
            if self.selected_files.len() > 0 && ui.button("Compress files").clicked() {
                self.receiver = Some(self.run());
            }
        });
    }
    pub fn render_main(
        self: &mut Self,
        ui: &mut Ui,
        total_file_size: &mut u64,
        last_folder: &mut String,
    ) {
        if !self.selected_files.is_empty() {
            ScrollArea::vertical().show(ui, |ui| {
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
    }

    pub fn detect_files_being_dropped(&mut self, ctx: &egui::Context) {
        // Preview hovering files:
        if !ctx.input().raw.hovered_files.is_empty() {
            let mut text = "Dropping files:\n".to_owned();
            for file in &ctx.input().raw.hovered_files {
                if let Some(path) = &file.path {
                    text += &format!("\n{}", path.display());
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
                    Some(file_path) => SelectedFile::new(file_path.display().to_string()),
                    None => SelectedFile::new("???".to_owned()),
                })
                .collect::<_>();
        }
    }

    fn run(self: &Self) -> Receiver<usize> {
        let dims = Arc::new(self.file_dimensions.clone());
        let mut prev_dir = String::new();
        let (sender, receiver) = mpsc::channel();
        for (i, selected_file) in self.selected_files.iter().enumerate() {
            let selected_file = selected_file.clone();
            let out_dir = format!("{}/{}", selected_file.parent_folder, DEFAULT_OUT_DIR);
            if selected_file.parent_folder != prev_dir {
                if let Err(err) = create_dir_if_not_exists(&out_dir) {
                    eprintln!("Failed to create directory! {}", err)
                }
            }
            let out_file_path = format!("{}/{}", out_dir, selected_file.name);
            let dims = Arc::clone(&dims);
            let sender = sender.clone();
            self.thread_pool.execute(move || {
                match perform_magick(&selected_file.path, &out_file_path, dims, 85, false) {
                    Ok(()) => (),
                    Err(err) => {
                        eprintln!("Failed to shrink file {}! : {}", selected_file.path, err)
                    }
                }
                sender.send(i).unwrap();
            });
            prev_dir = selected_file.parent_folder;
        }
        drop(sender);
        receiver
    }
}

fn render_file(
    ui: &mut Ui,
    selected_file: &SelectedFile,
    total_file_size: &mut u64,
    _last_folder: &mut String,
) -> bool {
    *total_file_size += selected_file.size;

    let mut remove_file = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(&selected_file.name).strong())
            .on_hover_text_at_pointer(&selected_file.path);
        ui.with_layout(Layout::right_to_left(), |ui| {
            if ui.button("Deselect").clicked() {
                remove_file = true
            };
            if selected_file.done {
                ui.label("Finished");
                ui.add_space(5.);
            }
            ui.label(format!("{} bytes", selected_file.size));
        });
    });
    ui.separator();
    remove_file
}

pub fn render_header(ui: &mut Ui) {
    ui.vertical_centered(|ui| ui.heading("Rshrink"));
    ui.separator();
}

pub fn render_footer(ctx: &Context, total_file_size: u64, file_count: usize) {
    TopBottomPanel::bottom("footer").show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(PADDING);
            ui.label(format!(
                "Total file size: {} Kb ({} files)",
                total_file_size / 1024,
                file_count
            ));
            ui.add_space(PADDING);
        });
    });
}
