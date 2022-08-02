pub mod filesystem;
pub mod gui;
pub mod resizer;
pub mod threadpool;
pub mod utils;

#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;
#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
#[cfg(target_arch = "wasm32")]
use gui::RshrinkApp;
#[cfg(target_arch = "wasm32")]
use std::panic;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_rayon::init_thread_pool;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    // Get console.error() for panics
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends
    tracing_wasm::set_as_global_default();

    // let web_options = eframe::WebOptions::default();
    eframe::start_web(
        canvas_id,
        // web_options,
        Box::new(|cc| Box::new(RshrinkApp::new(cc))),
    )
}
