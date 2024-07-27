use image::{DynamicImage, ImageFormat};

use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::*;
use libremarkable::framebuffer::*;
use libremarkable::image::RgbImage;
use std::fs::OpenOptions;

use rgb565::Rgb565;

fn main() {
    let fb = Framebuffer::new();
    let width = u32::from(DISPLAYWIDTH);
    let height = u32::from(DISPLAYHEIGHT);
    let contents = fb
        .dump_region(mxcfb_rect {
            top: 0,
            left: 0,
            width,
            height,
        })
        .expect("dumping image buffer with known dimensions should succeed")
        .chunks_exact(2)
        .flat_map(|c| Rgb565::from_rgb565_le([c[0], c[1]]).to_srgb888_components())
        .collect::<Vec<_>>();

    let image =
        RgbImage::from_raw(width, height, contents).expect("unable to construct the rgb image");

    let Some(path) = std::env::args().nth(1) else {
        panic!("First argument must be the path to the PNG we will be writing")
    };

    let mut output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .expect("Invalid path provided as argument!");
    DynamicImage::ImageRgb8(image)
        .write_to(&mut output_file, ImageFormat::Png)
        .expect("failed while writing to output file");
}
