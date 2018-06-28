extern crate libremarkable;
extern crate tiny_http;

use libremarkable::framebuffer;
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::{FramebufferBase, FramebufferIO};
use libremarkable::image;
use std::io::BufWriter;
use tiny_http::{Response, Server};

use libremarkable::framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH};

/// An HTTP server that listens on :8000 and responds to all incoming requests
/// with the full contents of the framebuffer properly exported as a JPEG.
fn main() {
    let fb = Framebuffer::new("/dev/fb0");
    println!("libremarkable Framebuffer device initialized");

    let server = Server::http("0.0.0.0:8000").unwrap();
    println!("Listening on 0.0.0.0:8000...");

    for request in server.incoming_requests() {
        if request.url() == "/favicon.ico" {
            request.respond(Response::empty(404)).unwrap();
            continue;
        }

        let rgb565 =
            fb.dump_region(framebuffer::common::mxcfb_rect {
                top: 0,
                left: 0,
                width: DISPLAYWIDTH as u32,
                height: DISPLAYHEIGHT as u32,
            }).unwrap();

        let rgb888 = framebuffer::storage::rgbimage_from_u8_slice(
            DISPLAYWIDTH.into(),
            DISPLAYHEIGHT.into(),
            &rgb565,
        ).unwrap();
        let mut writer = BufWriter::new(Vec::new());
        image::jpeg::JPEGEncoder::new(&mut writer)
            .encode(
                &*rgb888,
                DISPLAYWIDTH.into(),
                DISPLAYHEIGHT.into(),
                image::ColorType::RGB(8),
            )
            .unwrap();

        let jpg = writer.into_inner().unwrap();
        let mut response = Response::new_empty(tiny_http::StatusCode(200))
            .with_data(&*jpg, Some(jpg.len()))
            .with_header(
                "Content-Type: image/jpeg"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            );
        request.respond(response).unwrap();
    }
}
