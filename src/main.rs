use eframe::{epaint::Vec2, NativeOptions};
use rshrink::{gui::RshrinkApp, resizer::shrink_image};

const MIN_WIN_SIZE: Option<Vec2> = Some(Vec2::new(360.0, 300.0));

fn main() {
    //    let native_options = NativeOptions {
    //        min_window_size: MIN_WIN_SIZE,
    //        ..Default::default()
    //    };
    //    eframe::run_native(
    //        "Rshrink",
    //        native_options,
    //        Box::new(|cc| Box::new(RshrinkApp::new(cc))),
    //    );
    // compress("testimage.jpg", 6000, 4000).expect("Something went wrong");
    // compress();
    shrink_image("test.jpg", "testimage3000x2000.jpg", 3000, 2000)
        .expect("Something went wrong resizing");
}
