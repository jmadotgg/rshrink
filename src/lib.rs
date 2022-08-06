pub mod filesystem;
pub mod gui;
pub mod resizer;
pub mod threadpool;
pub mod utils;

//#[cfg(target_arch = "wasm32")]
//use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use gui::RshrinkApp;

#[cfg(target_arch = "wasm32")]
use console_error_panic_hook;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
use std::{future::Future, panic};

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
}

// Create macro to use rust like syntax
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
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
