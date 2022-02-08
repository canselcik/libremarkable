use image::{
    bmp::BmpEncoder, gif::GifEncoder, jpeg::JpegEncoder, png::PngEncoder, tga::TgaEncoder,
    ColorType::Rgb8, ImageFormat,
};
use libremarkable::framebuffer;
use libremarkable::framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH};
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::FramebufferIO;
use libremarkable::image;
use rgb565::Rgb565;
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
        if request.url() == "/" {
            let response = Response::from_string(INDEX_PAGE)
                .with_header("Content-Type: text/html".parse::<Header>().unwrap());
            request.respond(response).unwrap();
            continue;
        }

        let width = DISPLAYWIDTH as u32;
        let height = DISPLAYHEIGHT as u32;
        let contents = fb
            .dump_region(framebuffer::common::mxcfb_rect {
                top: 0,
                left: 0,
                width,
                height,
            })
            .expect("dumping image buffer with known dimensions should succeed")
            .chunks_exact(2)
            .flat_map(|c| Rgb565::from_rgb565_le([c[0], c[1]]).to_srgb888_components())
            .collect::<Vec<_>>();

        let rgb888 = image::RgbImage::from_raw(width, height, contents)
            .expect("unable to construct the rgb image");

        let url_lc = request.url().to_lowercase();
        let (data, mime) = if url_lc.ends_with("jpg") || url_lc.ends_with("jpeg") {
            (encode(&*&rgb888, ImageFormat::Jpeg), "image/jpeg")
        } else if url_lc.ends_with("gif") {
            (encode(&*&rgb888, ImageFormat::Gif), "image/gif")
        } else if url_lc.ends_with("bmp") {
            (encode(&*&rgb888, ImageFormat::Bmp), "image/bmp")
        } else if url_lc.ends_with("tga") {
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
    let encoded = writer.into_inner();
    println!(
        "Encoded screenshot as {:?} in {:?} resulting in a file of {} KiB",
        format,
        start.elapsed(),
        encoded.len() / 1024
    );
    encoded
}

const INDEX_PAGE: &str = r#"<!DOCTYPE html>
<html>
    <head>
        <title>libremarkable example: live</title>
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            body {
                background-color: #f0f0f0;
                display: flex;
                flex-direction: column;
                align-items: center;
            }
            body > p {
                text-align: center;
            }
            a {
                text-decoration: inherit;
                color: inherit;
                border: 2px solid gray;
                margin-bottom: 5px;
                padding: 3px 3px;
                transition-duration: 0.2s;
            }
            a:hover {
                box-shadow: 0px 0px 2px rgba(0, 0, 0, 0.8);
                transition-duration: 0.2s;
            }
            body > * {
                width: 100%;
                max-width: 440px;
            }
        </style>
    </head>
        <p><b>Create screenshot as</b></p>
        <a href="/png">PNG<br><small>Lossless, fast and small when few realistic graphics included.</small></a>
        <a href="/jpg">JPG<br><small>Lossy, slower and usually smaller, can add noise in UIs.</small></a>
        <a href="/gif">GIF<br><small>Lossless, very small and slow. Reduced colour pallete.</small></a>
        <a href="/tga">TGA<br><small>Lossless, huge and fast. Usually slowest due to speed, but raw.</small></a>
        <small>You can save the image by right clicking or holding the image and selecting <i>Save image</i></small>
    </body>
</html>
"#;
