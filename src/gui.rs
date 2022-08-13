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

use regex::Regex;

use std::{
    fs::File,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use crate::{
    filesystem::create_dir_if_not_exists,
    resizer::shrink_image,
    threadpool::ThreadPool,
    utils::{round_percent, Resize},
    SelectedFile, Settings,
};

#[cfg(target_arch = "wasm32")]
use futures::future::join_all;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

#[cfg(target_arch = "wasm32")]
use crate::{console_log, execute, log, my_func};
#[cfg(target_arch = "wasm32")]
use rfd::FileHandle;
#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;

#[cfg(target_arch = "wasm32")]
use rayon::prelude::*;

const DEFAULT_REGEX: &str = r".*.(jpg|png|jpeg|JPG|PNG|JPEG)$";
const PADDING: f32 = 5.0;

#[derive(Default)]
pub struct RshrinkApp {
    #[cfg(not(target_arch = "wasm32"))]
    selected_files: Vec<SelectedFile>,
    #[cfg(target_arch = "wasm32")]
    selected_files: Arc<Mutex<Vec<SelectedFile>>>,
    total_file_size: u64,
    total_new_file_size: Arc<AtomicU64>,
    #[cfg(not(target_arch = "wasm32"))]
    thread_pool: ThreadPool,
    //#[cfg(target_arch = "wasm32")]
    //thread_pool: MyRayon,
    is_running: bool,
    has_run_once: bool,
    settings_dialog_opened: bool,
    settings: Settings,
}

impl App for RshrinkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let mut last_folder = String::new();
        // Footer (first, because of CentralPanel filling the remaininng space)

        #[cfg(not(target_arch = "wasm32"))]
        let file_count = self.selected_files.len();
        #[cfg(target_arch = "wasm32")]
        let file_count = self.selected_files.lock().unwrap().len();

        render_footer(
            ctx,
            self.total_file_size,
            Arc::clone(&self.total_new_file_size),
            self.has_run_once,
            file_count,
        );
        CentralPanel::default().show(ctx, |ui| {
            // Render menu
            self.render_menu(ctx, ui);
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

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(settings) = serde_json::to_string(&self.settings) {
            storage.set_string("settings", settings);
        }
    }
}

impl RshrinkApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        // Retrieve stored settings from file
        let mut stored_settings: Option<Settings> = None;
        if let Some(storage) = cc.storage {
            if let Some(settings) = storage.get_string("settings") {
                if let Ok(settings) = serde_json::from_str(&settings) {
                    stored_settings = Some(settings);
                }
            }
        }

        // Apply stored settings if they exist
        let settings = match stored_settings {
            Some(settings) => settings,
            None => Settings::default(),
        };

        // Set theme accordingly
        cc.egui_ctx.set_visuals(match settings.light_mode {
            false => Visuals::dark(),
            true => Visuals::light(),
        });

        // Start with saved settings if they exist
        Self {
            settings,
            ..Default::default()
        }
    }
    pub fn render_menu(&mut self, ctx: &Context, ui: &mut Ui) {
        menu::bar(ui, |ui| {
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Settings").clicked() {
                self.settings_dialog_opened = !self.settings_dialog_opened;
            };

            #[cfg(not(target_arch = "wasm32"))]
            Window::new("Settings")
                .open(&mut self.settings_dialog_opened)
                .resizable(false)
                .collapsible(false)
                .title_bar(true)
                .anchor(Align2::CENTER_CENTER, Vec2::new(0., 0.))
                .show(ctx, |ui| {
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
                                            None => {
                                                format!("./{}", self.settings.output_folder_name)
                                            }
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

            let Settings { light_mode, .. } = &mut self.settings;
            let theme_text = match light_mode {
                true => "🌙",
                false => "🔆",
            };
            if ui.button(theme_text).clicked() {
                ctx.set_visuals(match light_mode {
                    true => Visuals::dark(),
                    false => Visuals::light(),
                });
                *light_mode = !(*light_mode);
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
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(file_paths) = rfd::FileDialog::new().pick_files() {
                    // Manually reset old total file size
                    self.total_file_size = 0;
                    self.total_new_file_size
                        .store(self.total_file_size, Ordering::Relaxed);
                    self.has_run_once = false;
                    self.selected_files = file_paths
                        .iter()
                        .map(|path_buf| {
                            let selected_file = SelectedFile::new(path_buf).expect(&format!(
                                "Failed to read file {}",
                                path_buf.display().to_string()
                            ));
                            self.total_file_size += selected_file.size.original;
                            selected_file
                        })
                        .collect::<Vec<_>>();
                }

                #[cfg(target_arch = "wasm32")]
                {
                    let selected_files = Arc::clone(&self.selected_files);
                    let dialog = rfd::AsyncFileDialog::new().pick_files();
                    execute(async move {
                        let files = dialog.await;
                        if let Some(files) = files {
                            // Maybe constrain the number of files to 300
                            // in order to not crash the sandbox
                            let _selected_files = files
                                .into_iter()
                                .map(|file| SelectedFile::new(file))
                                .collect::<Vec<_>>();

                            let _selected_files = join_all(_selected_files).await;

                            let mut selected_files = selected_files.lock().unwrap();
                            *selected_files = _selected_files;
                        }
                    });
                };
            }

            // Clear files
            #[cfg(not(target_arch = "wasm32"))]
            let selected_files = &mut self.selected_files;
            #[cfg(target_arch = "wasm32")]
            let mut selected_files = self.selected_files.lock().unwrap();
            if ui
                .add_enabled(
                    !self.is_running && selected_files.is_empty(),
                    Button::new("Clear all  ❌"),
                )
                .clicked()
            {
                selected_files.clear();
                self.has_run_once = false;
                self.total_file_size = 0;
                self.total_new_file_size.store(0, Ordering::Relaxed);
            };
            // Run program
            if ui
                .add_enabled(
                    !self.is_running && !selected_files.is_empty(),
                    Button::new("Compress files 🔨"),
                )
                .clicked()
            {
                // Clean up potential previous run before initializing a new one
                for selected_file in selected_files.iter() {
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
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Resize");
                        ui.horizontal(|ui| {
                            ui.radio_value(
                                &mut self.settings.resize_method,
                                Resize::Absolute,
                                "Absolute",
                            );
                            ui.add_enabled(
                                self.settings.resize_method == Resize::Absolute,
                                TextEdit::singleline(&mut width),
                            );
                            ui.label("x");
                            ui.add_enabled(
                                self.settings.resize_method == Resize::Absolute,
                                TextEdit::singleline(&mut height),
                            );
                            ui.label("px");
                        });
                        ui.horizontal(|ui| {
                            ui.radio_value(
                                &mut self.settings.resize_method,
                                Resize::Relative,
                                "Relative",
                            );
                            ui.add_enabled(
                                self.settings.resize_method == Resize::Relative,
                                Slider::new(&mut self.settings.dimensions_relative, 1..=100)
                                    .suffix('%'),
                            )
                        });
                    });
                    ui.end_row();
                    //Resize image or keep originial size
                    ui.end_row();
                });
            if let Err(err) = self
                .settings
                .dimensions
                .save_dimensions_from_string((width, height))
            {
                eprintln!("Error saving dimensions! {}", err)
            }
        });
    }
    pub fn render_main(&mut self, ui: &mut Ui, last_folder: &mut str) {
        #[cfg(not(target_arch = "wasm32"))]
        let selected_files = &mut self.selected_files;
        #[cfg(target_arch = "wasm32")]
        let mut selected_files = self.selected_files.lock().unwrap();
        if !selected_files.is_empty() {
            ScrollArea::vertical().show(ui, |ui| {
                let mut files_to_remove_indexes = Vec::new();
                // Determine if compression finished
                let mut all_done = true;

                for (i, selected_file) in selected_files.iter().enumerate() {
                    println!("{:?}", selected_file.name);
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
                        self.total_file_size -= selected_file.size.original;
                        self.total_new_file_size.fetch_sub(
                            selected_file.size.new.load(Ordering::Relaxed),
                            Ordering::Relaxed,
                        );
                    }
                }
                if all_done {
                    self.is_running = false;
                }
                for i in files_to_remove_indexes {
                    selected_files.remove(i);
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Select files or drop them here");
            });
        }
    }

    pub fn detect_files_being_dropped(&mut self, ctx: &egui::Context) {
        if self.is_running {
            return;
        }
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
        #[cfg(not(target_arch = "wasm32"))]
        if !ctx.input().raw.dropped_files.is_empty() {
            // Manually reset old total file size
            self.total_file_size = 0;
            self.total_new_file_size
                .store(self.total_file_size, Ordering::Relaxed);
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
                .map(|dropped_file| {
                    #[cfg(target_arch = "wasm32")]
                    console_log!("{:?}", dropped_file);

                    #[cfg(not(target_arch = "wasm32"))]
                    let path_buf = dropped_file.path.clone().expect(&format!(
                        "Failed to read dropped file {}",
                        &dropped_file.name
                    ));
                    #[cfg(not(target_arch = "wasm32"))]
                    let selected_file = SelectedFile::new(&path_buf).expect(&format!(
                        "Failed to read file {}",
                        path_buf.display().to_string()
                    ));

                    self.total_file_size += selected_file.size.original;
                    selected_file
                })
                .collect::<_>();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn run(&self) {
        let nums: [u8; 3] = [1, 2, 3];
        // let x = my_func(&nums);
        // console_log!("{:?}", x);
        use crate::postMessage;
        postMessage(&nums, "http://localhost:8080")
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn run(&self) {
        let Settings {
            output_folder_parent_dir_path_enabled: _,
            output_folder_name,
            output_folder_parent_dir_path,
            dimensions,
            dimensions_relative,
            resize_method,
            ..
        } = &self.settings;
        let resize_method = Arc::new(*resize_method);
        let dims = Arc::new(dimensions.clone());
        let mut prev_dir = PathBuf::new();
        for selected_file in &self.selected_files {
            let selected_file = selected_file.clone();

            let mut out_folder = PathBuf::from(match output_folder_parent_dir_path {
                Some(path) => path,
                None => &selected_file.parent_folder,
            });
            out_folder.push(output_folder_name);

            if out_folder != prev_dir {
                if let Err(err) = create_dir_if_not_exists(&out_folder) {
                    eprintln!("Failed to create folder! {}", err)
                }
            }

            let mut out_file_path = PathBuf::from(&out_folder);
            out_file_path.push(
                match out_folder == PathBuf::from(&selected_file.parent_folder) {
                    true => format!("min-{}", selected_file.name),
                    false => selected_file.name,
                },
            );

            let resize_method = Arc::clone(&resize_method);
            let dims = Arc::clone(&dims);
            let dims_relative = *dimensions_relative;
            let done = Arc::clone(&selected_file.done);
            let new_filesize = Arc::clone(&selected_file.size.new);

            // Reset total file size
            self.total_new_file_size.store(0, Ordering::Relaxed);
            let total_new_file_size = Arc::clone(&self.total_new_file_size);

            #[cfg(not(target_arch = "wasm32"))]
            self.thread_pool.execute(move || {
                if let Err(err) = shrink_image(
                    &selected_file.path,
                    out_file_path.display().to_string().as_ref(),
                    resize_method,
                    dims,
                    dims_relative,
                ) {
                    eprintln!("Failed to shrink file {}! : {}", selected_file.path, err)
                } else {
                    //Read file metadata to determine new file size
                    match File::open(&out_file_path) {
                        Ok(file) => match File::metadata(&file) {
                            Ok(metadata) => {
                                let file_size = metadata.len();
                                //Store the indiviual files new size
                                new_filesize.store(file_size, Ordering::Relaxed);
                                //Store the overall new file size
                                total_new_file_size.fetch_add(file_size, Ordering::Relaxed);
                            }
                            Err(err) => {
                                eprintln!("Failed to read the new file's metadata! {}", err)
                            }
                        },
                        Err(err) => eprintln!("Failed to read new file size! {}", err),
                    }
                }
                //complete the job for the UI
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
        #[cfg(not(target_arch = "wasm32"))]
        ui.label(RichText::new(&selected_file.name).strong())
            .on_hover_text_at_pointer(&selected_file.path);
        #[cfg(target_arch = "wasm32")]
        ui.label(RichText::new(&selected_file.name).strong());
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
            ui.label(format!(
                "{}%",
                round_percent(
                    selected_file.size.new.load(Ordering::Relaxed),
                    selected_file.size.original,
                )
            ));
        });
    });
    ui.separator();
    (done, remove_file)
}

pub fn render_header(ui: &mut Ui) {
    ui.vertical_centered(|ui| ui.heading("Rshrink"));
}

pub fn render_footer(
    ctx: &Context,
    total_file_size: u64,
    total_new_file_size: Arc<AtomicU64>,
    has_run_once: bool,
    file_count: usize,
) {
    TopBottomPanel::bottom("footer").show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(PADDING);
            ui.label(format!(
                "Original size: {} Kb ({} files)",
                total_file_size / 1024,
                file_count
            ));

            ui.add_enabled(
                has_run_once,
                Label::new(format!(
                    "☞ New size: {} Kb ({}%)",
                    total_new_file_size.load(Ordering::Relaxed) / 1024,
                    round_percent(total_new_file_size.load(Ordering::Relaxed), total_file_size)
                )),
            );
            ui.add_space(PADDING);
        });
    });
}
