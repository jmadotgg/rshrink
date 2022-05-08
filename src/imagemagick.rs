use std::sync::Arc;

use magick_rust::{MagickError, MagickWand};

use crate::utils::Dimensions;

pub fn perform_magick(
    in_file: &str,
    out_file: &str,
    dims: Arc<Option<Dimensions>>,
    compression_quality: usize,
    apply_gaussian_blur: bool,
) -> Result<(), MagickError> {
    let mut wand = MagickWand::new();
    wand.read_image(in_file)?;
    if let Some(dims) = dims.as_ref() {
        // TODO: Check if provided dimensions are actually smaller than original dimensions
        wand.fit(dims.width, dims.height);
    }
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
