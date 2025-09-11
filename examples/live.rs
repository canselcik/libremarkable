use image::codecs::{
    bmp::BmpEncoder, gif::GifEncoder, jpeg::JpegEncoder, png::PngEncoder, tga::TgaEncoder, webp::WebPEncoder,
};
use image::ImageEncoder;
use image::{ExtendedColorType::Rgb8, ImageFormat};
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

        let width = u32::from(DISPLAYWIDTH);
        let height = u32::from(DISPLAYHEIGHT);
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
            (encode(&rgb888, ImageFormat::Jpeg), "image/jpeg")
        } else if url_lc.ends_with("gif") {
            (encode(&rgb888, ImageFormat::Gif), "image/gif")
        } else if url_lc.ends_with("bmp") {
            (encode(&rgb888, ImageFormat::Bmp), "image/bmp")
        } else if url_lc.ends_with("tga") {
            (encode(&rgb888, ImageFormat::Tga), "image/x-tga")
        } else if url_lc.ends_with("png") {
            (encode(&rgb888, ImageFormat::Png), "image/png")
        } else if url_lc.ends_with("webp") {
            (encode(&rgb888, ImageFormat::WebP), "image/webp")
        }else {
            // 404
            let response_text = "404 Not found\nEither go to / or specify any path ending in a supported file extension: webp, png, jp(e)g, gif, tga or bmp)";
            let response = Response::new_empty(StatusCode(404))
                .with_data(response_text.as_bytes(), Some(response_text.as_bytes().len()))
                .with_header(format!("Content-Type: text/plain").parse::<Header>().unwrap());
            request.respond(response).unwrap();
            continue;
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
        ImageFormat::Png => PngEncoder::new(&mut writer).write_image(img_buf, width, height, Rgb8),
        ImageFormat::Tga => TgaEncoder::new(&mut writer).encode(img_buf, width, height, Rgb8),
        ImageFormat::WebP => WebPEncoder::new_lossless(&mut writer).encode(img_buf, width, height, Rgb8),
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
        <a href="/my-cool-screenshot.tga">WEBP<br><small>Lossless mode, twice as fast and half as big as png.</small></a>
        <a href="/my-cool-screenshot.png">PNG<br><small>Lossless, old and trusty. Can get big with realistic graphics.</small></a>
        <a href="/my-cool-screenshot.jpg">JPG<br><small>Lossy, slower but very small. Can add noise in UIs.</small></a>
        <p><small>You can save the image by right clicking or holding the image and selecting <i>Save image</i></small></p>
        <p><small>Any path works that ends in a supported extension (webp, png, jpg, gif, tga or bmp)</small><p>
    </body>
</html>
"#;
