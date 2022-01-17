use image::{
    bmp::BmpEncoder, gif::GifEncoder, jpeg::JpegEncoder, png::PngEncoder, tga::TgaEncoder,
    ColorType::Rgb8, ImageFormat,
};
use libremarkable::framebuffer;
use libremarkable::framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH};
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::FramebufferIO;
use libremarkable::image;
use std::io::Cursor;
use tiny_http::{Header, Response, Server, StatusCode};

/// An HTTP server that listens on :8000 and responds to all incoming requests
/// with the full contents of the framebuffer properly exported as a JPEG.
fn main() {
    let fb = Framebuffer::new();
    println!("libremarkable Framebuffer device initialized");

    let server = Server::http("0.0.0.0:8000").unwrap();
    println!("Listening on 0.0.0.0:8000...");

    for request in server.incoming_requests() {
        if request.url() == "/favicon.ico" {
            request.respond(Response::empty(404)).unwrap();
            continue;
        }

        let rgb565 = fb
            .dump_region(framebuffer::common::mxcfb_rect {
                top: 0,
                left: 0,
                width: DISPLAYWIDTH.into(),
                height: DISPLAYHEIGHT.into(),
            })
            .unwrap();

        let rgb888 = framebuffer::storage::rgbimage_from_u8_slice(
            DISPLAYWIDTH.into(),
            DISPLAYHEIGHT.into(),
            &rgb565,
        )
        .unwrap();
        let url = request.url().to_lowercase();
        let (data, mime) = if url.ends_with("jpg") || url.ends_with("jpeg") {
            (encode(&*&rgb888, ImageFormat::Jpeg), "image/jpeg")
        } else if url.ends_with("gif") {
            (encode(&*&rgb888, ImageFormat::Gif), "image/gif")
        } else if url.ends_with("bmp") {
            (encode(&*&rgb888, ImageFormat::Bmp), "image/bmp")
        } else if url.ends_with("tga") {
            (encode(&*&rgb888, ImageFormat::Tga), "image/x-tga")
        } else {
            (encode(&*&rgb888, ImageFormat::Png), "image/png")
        };
        let response = Response::new_empty(StatusCode(200))
            .with_data(&*data, Some(data.len()))
            .with_header(format!("Content-Type: {}", mime).parse::<Header>().unwrap());
        request.respond(response).unwrap();
    }
}

fn encode(img_buf: &[u8], format: ImageFormat) -> Vec<u8> {
    let start = std::time::Instant::now();
    let (width, height) = (DISPLAYWIDTH.into(), DISPLAYHEIGHT.into());
    let mut writer = Cursor::new(Vec::new());
    match format {
        ImageFormat::Bmp => BmpEncoder::new(&mut writer).encode(img_buf, width, height, Rgb8),
        ImageFormat::Gif => GifEncoder::new(&mut writer).encode(img_buf, width, height, Rgb8),
        ImageFormat::Jpeg => JpegEncoder::new(&mut writer).encode(img_buf, width, height, Rgb8),
        ImageFormat::Png => PngEncoder::new(&mut writer).encode(img_buf, width, height, Rgb8),
        ImageFormat::Tga => TgaEncoder::new(&mut writer).encode(img_buf, width, height, Rgb8),
        _ => unimplemented!(),
    }
    .unwrap();
    let res = writer.into_inner();
    println!(
        "Encoded screenshot as {:?} in {:?} resulting in a file of {} KiB",
        format,
        start.elapsed(),
        res.len() / 1024
    );
    res
}
