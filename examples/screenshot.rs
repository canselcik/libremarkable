use image::{DynamicImage, ImageOutputFormat};

use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::*;
use libremarkable::framebuffer::*;
use libremarkable::image::RgbImage;
use std::fs::OpenOptions;

fn main() {
    let fb = Framebuffer::new();
    let width = DISPLAYWIDTH as u32;
    let height = DISPLAYHEIGHT as u32;
    let contents = fb
        .dump_region(mxcfb_rect {
            top: 0,
            left: 0,
            width,
            height,
        })
        .expect("dumping image buffer with known dimensions should succeed")
        .chunks_exact(2)
        .flat_map(|c| color::NATIVE_COMPONENTS(c[0], c[1]).to_rgb8())
        .collect::<Vec<_>>();

    let image =
        RgbImage::from_raw(width, height, contents).expect("unable to construct the rgb image");

    let args = std::env::args().collect::<Vec<_>>();

    match args.get(1) {
        Some(path) => {
            let mut output_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)
                .expect("Invalid path provided as argument!");
            DynamicImage::ImageRgb8(image)
                .write_to(&mut output_file, ImageOutputFormat::Png)
                .expect("failed while writing to output file");
        }
        None => {
            let mut stdout = std::io::stdout();
            DynamicImage::ImageRgb8(image)
                .write_to(&mut stdout, ImageOutputFormat::Png)
                .expect("failed while writing to stdout");
        }
    }
}
