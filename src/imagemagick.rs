use std::sync::Arc;

use magick_rust::{MagickError, MagickWand};

use crate::utils::Dimensions;

//impl Executor for ImageMagick {
//    fn new(
//        self: &mut Self,
//        files: Vec<String>,
//        in_dir: Arc<String>,
//        out_dir: Arc<String>,
//    ) -> ImageMagick {
//        ImageMagick {
//            files,
//            in_dir,
//            out_dir,
//        }
//    }
//}
pub fn perform_magick(
    in_file: &str,
    out_file: &str,
    dims: Arc<Dimensions>,
    compression_quality: usize,
    apply_gaussian_blur: bool,
) -> Result<(), MagickError> {
    let mut wand = MagickWand::new();
    wand.read_image(in_file)?;
    wand.fit(dims.width, dims.height);
    wand.set_sampling_factors(&[4.0, 2.0, 0.0])?;
    wand.strip_image()?;
    wand.set_image_compression_quality(compression_quality)?;
    // 3 = Plane (build.rs)
    wand.set_interlace_scheme(3)?;
    // 26 should be RGB (have to build magick_rust myself to verify)
    // wand.set_image_colorspace(30)?;
    if apply_gaussian_blur {
        // Pretty slow
        wand.gaussian_blur_image(0.05, 1.0)?
    }

    wand.write_image(out_file)
}
