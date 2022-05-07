use eframe::{
    egui::{
        self, menu, Button, CentralPanel, Context, Grid, Id, Label, LayerId, Layout, Order,
        RichText, ScrollArea, Slider, Spinner, TextEdit, TextStyle, TopBottomPanel, Ui, Visuals,
        Widget, Window,
    },
    emath::{Align2, Vec2},
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

struct Settings {
    dimensions: Dimensions,
    change_dimensions: bool,
    compression_quality: usize,
    output_folder_name: String,
    output_folder_parent_dir_path: Option<String>,
    output_folder_parent_dir_path_enabled: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            dimensions: Dimensions::default(),
            change_dimensions: true,
            compression_quality: 85,
            output_folder_name: String::from(DEFAULT_OUT_DIR),

            output_folder_parent_dir_path_enabled: false,
            output_folder_parent_dir_path: None,
        }
    }
}

#[derive(Clone)]
struct SelectedFile {
    path: String,
    parent_folder: String,
    name: String,
    size: u64,
    done: Arc<AtomicBool>,
}

impl SelectedFile {
    fn new(path: String) -> SelectedFile {
        let path_vec = path.split('/').collect::<Vec<_>>();
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
    thread_pool: ThreadPool,
    light_mode: bool,
    is_running: bool,
    has_run_once: bool,
    settings_dialog_opened: bool,
    settings: Settings,
}

impl App for RshrinkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut last_folder = String::new();
        // Footer (first, because of CentralPanel filling the remaininng space)
        render_footer(ctx, self.total_file_size, self.selected_files.len());
        CentralPanel::default().show(ctx, |ui| {
            // Render menu
            self.render_menu(&ctx, ui);
            // Header
            render_header(ui);
            ui.add_space(5.0);
            // Controls
            ui.group(|ui| {
                self.render_controls(ui);
            });
            ui.add_space(5.0);
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
    }
    pub fn render_menu(&mut self, ctx: &Context, ui: &mut Ui) {
        menu::bar(ui, |ui| {
            // TODO: Do something useful here
            if ui.button("Settings").clicked() {
                self.settings_dialog_opened = !self.settings_dialog_opened;
            };
            // if self.settings_dialog_opened {
            // let painter =
            // ctx.layer_painter(LayerId::new(Order::Background, Id::new("file_drop_target")));
            //
            // let screen_rect = ctx.input().screen_rect();
            // painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            // }

            // let mut ctx = ctx.clone();
            // let mut style = (*ctx.style()).clone();
            // style.visuals.popup_shadow = Shadow {
            // extrusion: 1000.0,
            // color: Color32::RED,
            // };
            // ctx.set_style(style);

            Window::new("Settings")
                .open(&mut self.settings_dialog_opened)
                .resizable(false)
                .collapsible(false)
                .title_bar(true)
                .anchor(Align2::CENTER_CENTER, Vec2::new(0., 0.))
                // .fixed_size(Vec2::new(300., 300.))
                .show(&ctx, |ui| {
                    Grid::new("Settings grid")
                        .num_columns(2)
                        .spacing([60., 10.])
                        .show(ui, |ui| {
                            ui.checkbox(
                                &mut self.settings.output_folder_parent_dir_path_enabled,
                                "Change file directory",
                            );
                            if self.settings.output_folder_parent_dir_path == None
                                && self.settings.output_folder_parent_dir_path_enabled
                            {
                                match rfd::FileDialog::new().pick_folder() {
                                    Some(folder) => match folder.to_str() {
                                        Some(f) => {
                                            self.settings.output_folder_parent_dir_path =
                                                Some(String::from(f));
                                        }
                                        None => {
                                            self.settings.output_folder_parent_dir_path_enabled =
                                                false
                                        }
                                    },
                                    None => {
                                        self.settings.output_folder_parent_dir_path_enabled = false
                                    }
                                }
                            } else if !self.settings.output_folder_parent_dir_path_enabled {
                                self.settings.output_folder_parent_dir_path = None;
                            }
                            ui.add(
                                Label::new(
                                    RichText::new(
                                        match &self.settings.output_folder_parent_dir_path {
                                            Some(path) => {
                                                format!(
                                                    "{}/{}",
                                                    path, self.settings.output_folder_name
                                                )
                                            }
                                            None => String::from(format!(
                                                "./{}",
                                                self.settings.output_folder_name
                                            )),
                                        },
                                    )
                                    .italics(),
                                )
                                .wrap(true),
                            );
                            ui.end_row();
                            ui.wrap_text();
                            ui.label(RichText::new("Output folder name"));
                            ui.add(
                                TextEdit::singleline(&mut self.settings.output_folder_name)
                                    .hint_text("Same folder with \"min-\" prefix"),
                            );
                            ui.end_row();
                        })
                });

            let theme_text = match self.light_mode {
                true => "🌙",
                false => "🔆",
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
    pub fn render_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Open file explorer
            if ui
                .add_enabled(!self.is_running, Button::new("Select files 📂"))
                .clicked()
            {
                if let Some(file_paths) = rfd::FileDialog::new().pick_files() {
                    // Manually reset old total file size
                    self.total_file_size = 0;
                    self.has_run_once = false;
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
            // Clear files
            if ui
                .add_enabled(
                    !self.is_running && !self.selected_files.is_empty(),
                    Button::new("Clear all files ❌"),
                )
                .clicked()
            {
                self.selected_files.clear();
            };
            // Run program
            if ui
                .add_enabled(
                    !self.is_running && !self.selected_files.is_empty(),
                    Button::new("Compress files 🔨"),
                )
                .clicked()
            {
                // Clean up potential previous run before initializing a new one
                for selected_file in &self.selected_files {
                    let done = Arc::clone(&selected_file.done);
                    done.store(false, Ordering::SeqCst);
                }

                self.is_running = true;

                if !self.has_run_once {
                    self.has_run_once = true;
                }

                self.run();
            }
            if self.is_running {
                Spinner::default().ui(ui);
            }
        });
        ui.separator();
        ui.collapsing("Compression settings", |ui| {
            let (mut width, mut height) = self.settings.dimensions.as_string();
            // Compression Controls
            Grid::new("compression_settings_grid")
                .num_columns(2)
                .spacing([60.0, 10.0])
                .max_col_width(100.0)
                // .striped(true)
                .show(ui, |ui| {
                    ui.label("Quality");
                    ui.add(Slider::new(&mut self.settings.compression_quality, 1..=100));
                    ui.end_row();
                    // Resize image or keep originial size
                    ui.checkbox(&mut self.settings.change_dimensions, "Fit dimensions");
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            self.settings.change_dimensions,
                            TextEdit::singleline(&mut width).desired_width(50.0),
                        );
                        ui.add_enabled(
                            self.settings.change_dimensions,
                            TextEdit::singleline(&mut height).desired_width(50.0),
                        );
                    });
                    ui.end_row();
                });
            if let Err(err) = self
                .settings
                .dimensions
                .save_dimensions_from_string((width, height))
            {
                eprintln!("Error saving dimensions! {}", err)
            }
            // });
        });
    }
    pub fn render_main(&mut self, ui: &mut Ui, last_folder: &mut str) {
        if !self.selected_files.is_empty() {
            ScrollArea::vertical().show(ui, |ui| {
                let mut files_to_remove_indexes = Vec::new();
                // Determine if compression finished
                let mut all_done = true;
                for (i, selected_file) in self.selected_files.iter().enumerate() {
                    // For coloring columns
                    // egui::Frame::window(&(*ctx.style()).clone())..show(ui, |ui| {
                    // ui.label("Label with red background");
                    // });
                    let (done, remove_file) = render_file(
                        ui,
                        selected_file,
                        self.is_running,
                        self.has_run_once,
                        last_folder,
                    );
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
            self.has_run_once = false;
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

    fn run(&self) {
        let Settings {
            output_folder_parent_dir_path_enabled: _,
            output_folder_name,
            output_folder_parent_dir_path,
            change_dimensions,
            dimensions,
            compression_quality,
        } = &self.settings;
        let dims = Arc::new(match change_dimensions {
            true => Some(dimensions.clone()),
            false => None,
        });
        let mut prev_dir = String::new();
        for selected_file in &self.selected_files {
            let selected_file = selected_file.clone();
            println!(
                "{},{:?}",
                selected_file.parent_folder, output_folder_parent_dir_path
            );
            let out_folder = match output_folder_parent_dir_path {
                Some(path) => format!("{}/{}", path, output_folder_name),
                None => format!("{}/{}", selected_file.parent_folder, output_folder_name),
            };
            if out_folder != prev_dir {
                if let Err(err) = create_dir_if_not_exists(&out_folder) {
                    eprintln!("Failed to create folder! {}", err)
                }
            }
            let out_file_path =
                // Remove "/" at the end
                match out_folder[..out_folder.len() - 1] == selected_file.parent_folder {
                    // Results in 2 "/", if output_folder_name is empty, but that shouldn't be a problem
                    true => format!("{}/min-{}", out_folder, selected_file.name),
                    false => format!("{}/{}", out_folder, selected_file.name),
                };
            let dims = Arc::clone(&dims);
            let compression_quality = compression_quality.clone();
            let done = Arc::clone(&selected_file.done);
            self.thread_pool.execute(move || {
                if let Err(err) = perform_magick(
                    &selected_file.path,
                    &out_file_path,
                    dims,
                    compression_quality,
                    false,
                ) {
                    eprintln!("Failed to shrink file {}! : {}", selected_file.path, err)
                }
                done.store(true, Ordering::Relaxed);
            });
            prev_dir = out_folder;
        }
    }
}

fn render_file(
    ui: &mut Ui,
    selected_file: &SelectedFile,
    is_running: bool,
    has_run_once: bool,
    _last_folder: &mut str,
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
            if ui.add_enabled(!is_running, Button::new("❌")).clicked() {
                remove_file = true
            }
            // Add label if file has been compressed
            if done || (has_run_once && !is_running) {
                ui.label("Done ✅");
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
