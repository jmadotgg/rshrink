use std::sync::Arc;

use image::{io::Reader as ImageReader, ColorType, DynamicImage, ImageBuffer, ImageError, Rgb};
use resize::{Error, Pixel::RGB8, Type};
use rgb::FromSlice;

use crate::utils::{Dimensions, Resize};

pub fn shrink_image(
    path: &str,
    dst_path: &str,
    resize_method: Arc<Resize>,
    dims: Arc<Dimensions>,
    dims_relative: u32,
) -> Result<(), ImageError> {
    let img = read_image(path)?;
    let (sw, sh) = (img.width() as usize, img.height() as usize);

    // Don't troll me please
    if sw <= 100 || sh <= 100 {
        return Ok(());
    }

    // Rust is going to optimize this during compile time, right?
    // This looks more readable
    let (dw, dh) = match resize_method.as_ref() {
        Resize::Absolute => (dims.width, dims.height),
        Resize::Relative => (
            (sw as f32 * dims_relative as f32 / 100.) as usize,
            (sh as f32 * dims_relative as f32 / 100.) as usize,
        ),
    };

    let img = img.to_rgb8();
    let img_buf_resized = resize(sw, sh, dw, dh, &img).expect("Failed to resize image");
    save_image_buffer(dst_path, dw as u32, dh as u32, &img_buf_resized[..])
        .expect("Failed to save image buffer!");

    Ok(())
}

fn read_image(path: &str) -> Result<DynamicImage, ImageError> {
    ImageReader::open(path)?.decode()
}

pub fn save_image_buffer(
    dst_path: &str,
    width: u32,
    height: u32,
    img_buf: &[u8],
) -> Result<(), ImageError> {
    image::save_buffer(dst_path, img_buf, width, height, ColorType::Rgb8)?;

    Ok(())
}

fn resize(
    src_width: usize,
    src_height: usize,
    dst_width: usize,
    dst_height: usize,
    img_buf: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Result<Vec<u8>, Error> {
    let mut dst_buf = vec![0; dst_width * dst_height * 3];

    let mut resizer = resize::new(
        src_width,
        src_height,
        dst_width,
        dst_height,
        RGB8,
        Type::Lanczos3,
    )?;

    resizer.resize(img_buf.as_rgb(), dst_buf.as_rgb_mut())?;

    Ok(dst_buf)
}
