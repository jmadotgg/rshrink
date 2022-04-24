use std::sync::Arc;

use magick_rust::{MagickError, MagickWand};

use crate::utils::Dimensions;

pub fn shrink(
    file_name: &str,
    in_dir: Arc<String>,
    out_dir: Arc<String>,
    dims: Arc<Option<Dimensions>>,
    compression_quality: usize,
    apply_gaussian_blur: bool,
) -> Result<(), MagickError> {
    let mut wand = MagickWand::new();
    wand.read_image(format!("{in_dir}/{file_name}").as_str())?;

    // Credit: https://stackoverflow.com/questions/48471607/how-to-match-on-an-option-inside-an-arc
    // if let Some(ref d) = *dims {
    // wand.fit(d.width, d.height);
    // }
    if let Some(d) = Option::as_ref(&dims) {
        wand.fit(d.width, d.height);
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

    let new_file = format!("{}/sm_{}", out_dir, file_name);
    wand.write_image(new_file.as_str())
}
