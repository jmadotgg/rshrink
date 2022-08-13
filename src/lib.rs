pub mod filesystem;
pub mod gui;
pub mod resizer;
pub mod threadpool;
pub mod utils;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;

#[cfg(target_arch = "wasm32")]
use gui::RshrinkApp;

#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
use rfd::FileHandle;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use std::{future::Future, panic};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc,
    },
};
use utils::{Dimensions, Resize};

#[cfg(target_arch = "wasm32")]
use rayon::prelude::*;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_rayon::init_thread_pool;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures::{spawn_local, JsFuture};

#[cfg(target_arch = "wasm32")]
pub fn execute<F: Future<Output = ()> + 'static>(f: F) {
    spawn_local(f)
}

// https://rustwasm.github.io/wasm-bindgen/examples/console-log.html
// Log to browser console
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
    pub fn my_func(nums: &[u8]) -> i32;
    #[wasm_bindgen(js_namespace = ["window", "threadWorker"])]
    pub fn postMessage(message: &[u8], targetOrigin: &str);
    // #[wasm_bindgen(js_namespace = window)]
    // pub fn postToThreadWorker(message: &[u8], targetOrigin: &str);

}

// TODO: Look at reference https://github.com/GoogleChromeLabs/wasm-bindgen-rayon/issues/18
// TODO: Look at example https://github.com/GoogleChromeLabs/wasm-bindgen-rayon#usage-without-bundlers

// Create macro to use rust like syntax
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn sum(numbers: &[i32]) -> i32 {
    numbers.par_iter().sum()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run(canvas_id: String) {
    // Get console.error() for panics
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends
    tracing_wasm::set_as_global_default();

    let canvas_id = canvas_id.clone();

    spawn_local(async {
        run_async(canvas_id).await.unwrap_throw();
    });
    // let web_options = eframe::WebOptions::default();
}

#[cfg(target_arch = "wasm32")]
async fn run_async(canvas_id: String) -> Result<(), JsValue> {
    eframe::start_web(
        &canvas_id,
        // web_options,
        Box::new(|cc| Box::new(RshrinkApp::new(cc))),
    );
    Ok(())
}

const DEFAULT_OUT_DIR: &str = "_rshrinked";

#[derive(Serialize, Deserialize)]
pub struct Settings {
    dimensions: Dimensions,
    resize_method: Resize,
    dimensions_relative: u32,
    change_dimensions: bool,
    compression_quality: usize,
    output_folder_name: String,
    output_folder_parent_dir_path: Option<String>,
    output_folder_parent_dir_path_enabled: bool,
    light_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dimensions: Dimensions::default(),
            resize_method: Resize::Relative,
            dimensions_relative: 50,
            change_dimensions: true,
            compression_quality: 85,
            output_folder_name: String::from(DEFAULT_OUT_DIR),
            output_folder_parent_dir_path_enabled: false,
            output_folder_parent_dir_path: None,
            light_mode: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SelectedFile {
    #[cfg(not(target_arch = "wasm32"))]
    path: String,
    #[cfg(not(target_arch = "wasm32"))]
    parent_folder: String,
    #[cfg(target_arch = "wasm32")]
    data: Vec<u8>,
    name: String,
    size: FileSize,
    done: Arc<AtomicBool>,
}

impl SelectedFile {
    #[cfg(not(target_arch = "wasm32"))]
    fn new(file_path: &PathBuf) -> Option<SelectedFile> {
        let metadata = match fs::metadata(&file_path) {
            Ok(data) => data,
            Err(_) => return None,
        };
        let parent_folder = String::from(file_path.parent()?.to_str()?);
        let file_size = metadata.len();
        let file_name = String::from(file_path.file_name()?.to_str()?);

        Some(SelectedFile {
            path: file_path.display().to_string(),
            parent_folder,
            name: file_name,
            size: FileSize::new(file_size),
            done: Arc::new(AtomicBool::new(false)),
        })
    }
    #[cfg(target_arch = "wasm32")]
    async fn new(file_handle: FileHandle) -> SelectedFile {
        // let file_size = metadata.len();
        let file_name = file_handle.file_name();
        let file_data = file_handle.read().await;

        SelectedFile {
            name: file_name,
            size: FileSize::new(file_data.len() as u64),
            data: file_data,
            done: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Clone, Debug)]
struct FileSize {
    original: u64,
    new: Arc<AtomicU64>,
}

impl FileSize {
    fn new(original: u64) -> Self {
        Self {
            original,
            new: Arc::new(AtomicU64::new(original)),
        }
    }
}
