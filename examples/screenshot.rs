use image::{DynamicImage, ImageOutputFormat};

use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::*;
use libremarkable::framebuffer::*;
use libremarkable::image::RgbImage;
use std::fs::OpenOptions;

fn to_srgb([r, g, b]: [u8; 3]) -> [u8; 3] {
    // This is a mapping from Linear RGB values to sRGB values, required for output to normal images
    // The calculation performed was: ((i as f32 / 255.0).powf(1.0 / 2.2) * 255.0) as u8
    const GAMMA: [u8; 256] = [
        0, 20, 28, 33, 38, 42, 46, 49, 52, 55, 58, 61, 63, 65, 68, 70, 72, 74, 76, 78, 80, 81, 83,
        85, 87, 88, 90, 91, 93, 94, 96, 97, 99, 100, 102, 103, 104, 106, 107, 108, 109, 111, 112,
        113, 114, 115, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 128, 129, 130, 131, 132,
        133, 134, 135, 136, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 147, 148,
        149, 150, 151, 152, 153, 153, 154, 155, 156, 157, 158, 158, 159, 160, 161, 162, 162, 163,
        164, 165, 165, 166, 167, 168, 168, 169, 170, 171, 171, 172, 173, 174, 174, 175, 176, 176,
        177, 178, 178, 179, 180, 181, 181, 182, 183, 183, 184, 185, 185, 186, 187, 187, 188, 189,
        189, 190, 190, 191, 192, 192, 193, 194, 194, 195, 196, 196, 197, 197, 198, 199, 199, 200,
        200, 201, 202, 202, 203, 203, 204, 205, 205, 206, 206, 207, 208, 208, 209, 209, 210, 210,
        211, 212, 212, 213, 213, 214, 214, 215, 216, 216, 217, 217, 218, 218, 219, 219, 220, 220,
        221, 222, 222, 223, 223, 224, 224, 225, 225, 226, 226, 227, 227, 228, 228, 229, 229, 230,
        230, 231, 231, 232, 232, 233, 233, 234, 234, 235, 235, 236, 236, 237, 237, 238, 238, 239,
        239, 240, 240, 241, 241, 242, 242, 243, 243, 244, 244, 245, 245, 246, 246, 247, 247, 248,
        248, 249, 249, 249, 250, 250, 251, 251, 252, 252, 253, 253, 254, 254, 255,
    ];

    [GAMMA[r as usize], GAMMA[g as usize], GAMMA[b as usize]]
}

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
        .flat_map(|c| to_srgb(color::NATIVE_COMPONENTS(c[0], c[1]).to_rgb8()))
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
