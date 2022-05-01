use eframe::{
    egui::{
        self, CentralPanel, Context, Id, LayerId, Layout, Order, RichText, ScrollArea, Spinner,
        TextStyle, TopBottomPanel, Ui, Visuals, Widget,
    },
    emath::Align2,
    epaint::Color32,
    App, CreationContext, Frame,
};
use magick_rust::magick_wand_genesis;
use regex::Regex;
use std::{
    fs::File,
    sync::{
        atomic::{AtomicBool, Ordering},
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
    done: Arc<AtomicBool>,
}

enum Run {
    Initial(bool),
    Subsequent(bool),
}
impl Default for Run {
    fn default() -> Run {
        Run::Initial(false)
    }
}
impl Run {
    fn run(self: &Self) -> Run {
        match self {
            Run::Initial(_) => Run::Initial(true),
            Run::Subsequent(_) => Run::Subsequent(true),
        }
    }
    fn finish(self: &Self) -> Run {
        match self {
            Run::Initial(true) => Run::Subsequent(false),
            Run::Initial(false) => Run::Initial(false),
            Run::Subsequent(_) => Run::Subsequent(false),
        }
    }
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
            done: Arc::new(AtomicBool::new(false)),
        }
    }
}
#[derive(Default)]
pub struct RshrinkApp {
    selected_files: Vec<SelectedFile>,
    total_file_size: u64,
    file_dimensions: Dimensions,
    thread_pool: ThreadPool,
    light_mode: bool,
    is_running: bool,
    // TODO: Implement everywhere
    // is_running: Run,
}

impl App for RshrinkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut last_folder = String::new();
        // Footer (first, because of CentralPanel filling the remaininng space)
        render_footer(ctx, self.total_file_size, self.selected_files.len());
        CentralPanel::default().show(ctx, |ui| {
            // Header
            render_header(ui);
            // Controls
            self.render_controls(ctx, ui);
            ui.separator();
            // Files to shrink
            self.render_main(ui, &mut last_folder);
        });
        self.detect_files_being_dropped(ctx);
    }
}
impl RshrinkApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        // Init imagemagick
        START.call_once(|| {
            magick_wand_genesis();
        });
        cc.egui_ctx.set_visuals(Visuals::dark());
        Self::default()
        // Self {
        // selected_files: Vec::new(),
        // total_file_size: 0,
        // file_dimensions: Dimensions::default(),
        //    // Fill thread pool
        // thread_pool: ThreadPool::new(num_cpus::get()),
        // receiver: None,
        // }
    }
    pub fn render_controls(self: &mut Self, ctx: &Context, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if self.selected_files.len() > 0 && ui.button("Clear files").clicked() {
                self.selected_files.clear();
            };
            if ui.button("Select files").clicked() {
                if let Some(file_paths) = rfd::FileDialog::new().pick_files() {
                    // Manually reset old total file size
                    self.total_file_size = 0;
                    self.selected_files = file_paths
                        .iter()
                        .map(|path_buf| {
                            let selected_file = SelectedFile::new(path_buf.display().to_string());
                            self.total_file_size += selected_file.size;
                            selected_file
                        })
                        .collect::<Vec<_>>();
                }
            };
            if self.selected_files.len() > 0 && ui.button("Compress files").clicked() {
                // Clean up potential previous run before initializing a new one
                for selected_file in &self.selected_files {
                    let done = Arc::clone(&selected_file.done);
                    done.store(false, Ordering::SeqCst);
                }

                self.is_running = true;
                self.run();
            }
            let theme_text = match self.light_mode {
                true => "Theme dark",
                false => "Theme light",
            };
            if ui.button(theme_text).clicked() {
                ctx.set_visuals(match self.light_mode {
                    true => Visuals::dark(),
                    false => Visuals::light(),
                });
                self.light_mode = !self.light_mode;
            }
        });
    }
    pub fn render_main(self: &mut Self, ui: &mut Ui, last_folder: &mut String) {
        if !self.selected_files.is_empty() {
            ScrollArea::vertical().show(ui, |ui| {
                let mut files_to_remove_indexes = Vec::new();
                // Determine if compression finished
                let mut all_done = true;
                for (i, selected_file) in self.selected_files.iter().enumerate() {
                    let (done, remove_file) =
                        render_file(ui, selected_file, self.is_running, last_folder);
                    // If one file hasn't finished compressing, we don't care anymore
                    if all_done && !done {
                        all_done = false
                    }
                    if !self.is_running && remove_file {
                        files_to_remove_indexes.push(i);
                        // Decrease total file size manually
                        self.total_file_size -= selected_file.size;
                    }
                }
                if all_done {
                    self.is_running = false;
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
        if !ctx.input().raw.hovered_files.is_empty() {
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.input().screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                format!("Drop {} files here", &ctx.input().raw.hovered_files.len()),
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }
        // Collect dropped files
        if !ctx.input().raw.dropped_files.is_empty() {
            // Manually reset old total file size
            self.total_file_size = 0;
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
                    Some(file_path) => {
                        let selected_file = SelectedFile::new(file_path.display().to_string());
                        self.total_file_size += selected_file.size;
                        selected_file
                    }
                    None => SelectedFile::new("???".to_owned()),
                })
                .collect::<_>();
        }
    }

    fn run(self: &Self) {
        let dims = Arc::new(self.file_dimensions.clone());
        let mut prev_dir = String::new();
        for selected_file in &self.selected_files {
            let selected_file = selected_file.clone();
            let out_dir = format!("{}/{}", selected_file.parent_folder, DEFAULT_OUT_DIR);
            if selected_file.parent_folder != prev_dir {
                if let Err(err) = create_dir_if_not_exists(&out_dir) {
                    eprintln!("Failed to create directory! {}", err)
                }
            }
            let out_file_path = format!("{}/{}", out_dir, selected_file.name);
            let dims = Arc::clone(&dims);
            let done = Arc::clone(&selected_file.done);
            self.thread_pool.execute(move || {
                if let Err(err) =
                    perform_magick(&selected_file.path, &out_file_path, dims, 85, false)
                {
                    eprintln!("Failed to shrink file {}! : {}", selected_file.path, err)
                }
                done.store(true, Ordering::Relaxed);
            });
            prev_dir = selected_file.parent_folder;
        }
    }
}

fn render_file(
    ui: &mut Ui,
    selected_file: &SelectedFile,
    is_running: bool,
    _last_folder: &mut String,
) -> (bool, bool) {
    let mut remove_file = false;
    let done = match is_running {
        true => selected_file.done.load(Ordering::Relaxed),
        false => false,
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(&selected_file.name).strong())
            .on_hover_text_at_pointer(&selected_file.path);
        ui.with_layout(Layout::right_to_left(), |ui| {
            if ui.button("Deselect").clicked() && !is_running {
                remove_file = true
            };
            // Add label if file has been compressed
            if done {
                ui.label("Finished");
                ui.add_space(5.);
            } else if is_running {
                Spinner::default().ui(ui);
            }
            ui.label(format!("{} bytes", selected_file.size));
        });
    });
    ui.separator();
    (done, remove_file)
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
