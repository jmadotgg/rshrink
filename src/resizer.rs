use image::{io::Reader as ImageReader, ColorType, DynamicImage, ImageBuffer, ImageError, Rgb};
use resize::{Error, Pixel::RGB8, Type};
use rgb::FromSlice;

pub fn shrink_image(
    path: &str,
    dst_path: &str,
    width: usize,
    height: usize,
) -> Result<(), ImageError> {
    let img = read_image(path)?;
    let (sw, sh, dw, dh) = (img.width() as usize, img.height() as usize, width, height);
    if let Some(img_buf) = img.as_rgb8() {
        let img_buf_resized = resize(sw, sh, dw, dh, img_buf).expect("Failed to resize image");
        save_image_buffer(dst_path, dw as u32, dh as u32, &img_buf_resized[..])
            .expect("Failed to save image buffer!");
    } else {
        eprintln!("Failed to read image as rgb8!")
    }

    Ok(())
}

fn read_image(path: &str) -> Result<DynamicImage, ImageError> {
    Ok(ImageReader::open(path)?.decode()?)
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
