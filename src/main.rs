use eframe::{epaint::Vec2, NativeOptions};
use rshrink::gui::RshrinkApp;

const MIN_WIN_SIZE: Option<Vec2> = Some(Vec2::new(360.0, 300.0));

fn main() {
    let native_options = NativeOptions {
        min_window_size: MIN_WIN_SIZE,
        ..Default::default()
    };
    eframe::run_native(
        "Rshrink",
        native_options,
        Box::new(|cc| Box::new(RshrinkApp::new(cc))),
    );
}
