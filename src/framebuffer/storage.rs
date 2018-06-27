use std::sync::Arc;
use zstd;

#[derive(Clone)]
pub struct CompressedCanvasState {
    data: Arc<[u8]>,
    height: u32,
    width: u32,
}

/// For reference, a rectangle with height=1050 and width=1404
/// will have the following size at rest:
///
/// (notice better compression at first due to relatively low-entropy canvas
///  compression plateuaus around 93% as the entropy peaks)
///
///    raw: 5896.8 kB -- zstd: 7.875 kB  (99.86645% compression)
///    raw: 5896.8 kB -- zstd: 25.405 kB  (99.569176% compression)
///    raw: 5896.8 kB -- zstd: 210.628 kB  (96.4281% compression)
///    raw: 5896.8 kB -- zstd: 367.217 kB  (93.7726% compression)
///    raw: 5896.8 kB -- zstd: 367.217 kB  (93.7726% compression)
///    raw: 5896.8 kB -- zstd: 356.432 kB  (93.9555% compression)
///    raw: 5896.8 kB -- zstd: 361.935 kB  (93.86218% compression)
impl CompressedCanvasState {
    /// Creates a CompressedCanvasState from the output of FramebufferIO::dump_region(..)
    /// Consumes the RgbaImage that's provided to it.
    pub fn new(img: &[u8], height: u32, width: u32) -> CompressedCanvasState {
        CompressedCanvasState {
            data: zstd::encode_all(img, 0).unwrap().into(),
            height,
            width,
        }
    }

    /// Returns an ImageBuffer which can be used to restore the contents of a screen
    /// region using the FramebufferIO::restore_region(..)
    pub fn decompress(&self) -> Vec<u8> {
        zstd::decode_all(&*self.data).unwrap()
    }
}

use framebuffer::common;
use image;

pub fn rgbimage_from_u8_slice(w: u32, h: u32, buff: &[u8]) -> Option<image::RgbImage> {
    // rgb565 is the input so it is 16bits (2 bytes) per pixel
    let input_bytespp = 2;
    let input_line_len = w * input_bytespp;
    if h * input_line_len != buff.len() as u32 {
        return None;
    }
    Some(image::ImageBuffer::from_fn(w, h, |x, y| {
        let in_index: usize = ((y * input_line_len) + ((input_bytespp * x) as u32)) as usize;
        let data = common::color::NATIVE_COMPONENTS(buff[in_index], buff[in_index + 1]).to_rgb8();
        image::Rgb(data)
    }))
}
