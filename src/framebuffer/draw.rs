use std;

use image::RgbImage;
use libc;
use line_drawing;
use rusttype::{point, Scale};

use framebuffer;
use framebuffer::common::*;
use framebuffer::core;
use framebuffer::FramebufferIO;

macro_rules! min {
        ($x: expr) => ($x);
        ($x: expr, $($z: expr),+) => (::std::cmp::min($x, min!($($z),*)));
}

macro_rules! max {
        ($x: expr) => ($x);
        ($x: expr, $($z: expr),+) => (::std::cmp::max($x, max!($($z),*)));
}

/// Helper function to sample pixels on the bezier curve.
fn sample_bezier(startpt: (f32, f32), ctrlpt: (f32, f32), endpt: (f32, f32)) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    let mut lastpt = (-100, -100);
    for i in 0..1000 {
        let t = (i as f32) / 1000.0;
        let precisept = (
            (1.0 - t).powf(2.0) * startpt.0
                + 2.0 * (1.0 - t) * t * ctrlpt.0
                + t.powf(2.0) * endpt.0,
            (1.0 - t).powf(2.0) * startpt.1
                + 2.0 * (1.0 - t) * t * ctrlpt.1
                + t.powf(2.0) * endpt.1,
        );
        let pt = (precisept.0 as i32, precisept.1 as i32);
        // prevent oversampling
        if pt != lastpt {
            points.push(precisept);
            lastpt = pt;
        }
    }
    points
}

impl<'a> framebuffer::FramebufferDraw for core::Framebuffer<'a> {
    fn draw_image(&mut self, img: &RgbImage, top: usize, left: usize) -> mxcfb_rect {
        for (x, y, pixel) in img.enumerate_pixels() {
            self.write_pixel(
                top + y as usize,
                left + x as usize,
                color::RGB(pixel.data[0], pixel.data[1], pixel.data[2]),
            );
        }
        mxcfb_rect {
            top: top as u32,
            left: left as u32,
            width: img.width(),
            height: img.height(),
        }
    }

    fn draw_line(
        &mut self,
        y0: i32,
        x0: i32,
        y1: i32,
        x1: i32,
        width: usize,
        v: color,
    ) -> mxcfb_rect {
        // Create local variables for moving start point
        let mut x0 = x0;
        let mut y0 = y0;

        // Get absolute x/y offset
        let dx = if x0 > x1 { x0 - x1 } else { x1 - x0 };
        let dy = if y0 > y1 { y0 - y1 } else { y1 - y0 };

        // Get slopes
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        // Initialize error
        let mut err = if dx > dy { dx } else { -dy } / 2;
        let mut err2;

        let (mut min_x, mut max_x, mut min_y, mut max_y) = (x0, x0, y0, y0);
        loop {
            // Set pixel
            match width {
                1 => self.write_pixel(y0 as usize, x0 as usize, v),
                _ => self.fill_rect(
                    (y0 - (width / 2) as i32) as usize,
                    (x0 - (width / 2) as i32) as usize,
                    width,
                    width,
                    v,
                ),
            }

            max_y = max!(max_y, y0);
            min_y = min!(min_y, y0);
            min_x = min!(min_x, x0);
            max_x = max!(max_x, x0);

            // Check end condition
            if x0 == x1 && y0 == y1 {
                break;
            };

            // Store old error
            err2 = 2 * err;

            // Adjust error and start position
            if err2 > -dx {
                err -= dy;
                x0 += sx;
            }
            if err2 < dy {
                err += dx;
                y0 += sy;
            }
        }

        let margin = ((width + 1) / 2) as i32;
        mxcfb_rect {
            top: (min_y - margin) as u32,
            left: (min_x - margin) as u32,
            width: (max_x - min_x + margin * 2) as u32,
            height: (max_y - min_y + margin * 2) as u32,
        }
    }

    fn draw_circle(&mut self, y: usize, x: usize, rad: usize, v: color) -> mxcfb_rect {
        for (x, y) in line_drawing::BresenhamCircle::new(x as i32, y as i32, rad as i32) {
            self.write_pixel(y as usize, x as usize, v);
        }
        mxcfb_rect {
            top: y as u32 - rad as u32,
            left: x as u32 - rad as u32,
            width: 2 * rad as u32,
            height: 2 * rad as u32,
        }
    }

    fn fill_circle(&mut self, y: usize, x: usize, rad: usize, v: color) -> mxcfb_rect {
        for current in { 1..rad + 1 } {
            for (x, y) in line_drawing::BresenhamCircle::new(x as i32, y as i32, current as i32) {
                self.write_pixel(y as usize, x as usize, v);
            }
        }
        mxcfb_rect {
            top: y as u32 - rad as u32,
            left: x as u32 - rad as u32,
            width: 2 * rad as u32,
            height: 2 * rad as u32,
        }
    }

    fn draw_bezier(
        &mut self,
        startpt: (f32, f32),
        ctrlpt: (f32, f32),
        endpt: (f32, f32),
        width: usize,
        v: color,
    ) -> mxcfb_rect {
        let mut upperleft: (usize, usize) = (startpt.0 as usize, startpt.1 as usize);
        let mut lowerright: (usize, usize) = (endpt.0 as usize, endpt.1 as usize);
        for pt in sample_bezier(startpt, ctrlpt, endpt) {
            let approx = (pt.0 as usize, pt.1 as usize);
            upperleft.1 = min!(upperleft.1, approx.1);
            upperleft.0 = min!(upperleft.0, approx.0);
            lowerright.1 = max!(lowerright.1, approx.1);
            lowerright.0 = max!(lowerright.0, approx.0);

            // Set pixel
            match width {
                1 => self.write_pixel(approx.1, approx.0, v),
                _ => self.fill_rect(
                    (approx.1 - (width / 2)) as usize,
                    (approx.0 - (width / 2)) as usize,
                    width,
                    width,
                    v,
                ),
            };
        }
        let margin = ((width + 1) / 2) as usize;
        mxcfb_rect {
            top: (upperleft.1 - margin) as u32,
            left: (upperleft.0 - margin) as u32,
            width: (lowerright.0 - upperleft.0 + margin * 2) as u32,
            height: (lowerright.1 - upperleft.1 + margin * 2) as u32,
        }
    }

    fn draw_text(
        &mut self,
        y: usize,
        x: usize,
        text: String,
        size: usize,
        col: color,
        dryrun: bool,
    ) -> mxcfb_rect {
        let scale = Scale {
            x: size as f32,
            y: size as f32,
        };

        // The starting positioning of the glyphs (top left corner)
        let start = point(x as f32, y as f32);

        let dfont = &mut self.default_font.clone();

        let mut min_y = y;
        let mut max_y = y;
        let mut min_x = x;
        let mut max_x = x;

        let components = col.to_rgb8();
        let c1 = f32::from(255 - components[0]);
        let c2 = f32::from(255 - components[1]);
        let c3 = f32::from(255 - components[2]);

        // Loop through the glyphs in the text, positing each one on a line
        for glyph in dfont.layout(&text, scale, start) {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                // Draw the glyph into the image per-pixel by using the draw closure
                let bbmax_y = bounding_box.max.y as usize;
                let bbmax_x = bounding_box.max.x as usize;
                let bbmin_y = bounding_box.min.y as usize;
                let bbmin_x = bounding_box.min.x as usize;
                if bbmax_y > max_y {
                    max_y = bbmax_y;
                }
                if bbmax_x > max_x {
                    max_x = bbmax_x;
                }
                if bbmin_y < min_y {
                    min_y = bbmin_y;
                }
                if bbmin_x < min_x {
                    min_x = bbmin_x;
                }

                if dryrun {
                    continue;
                }

                glyph.draw(|x, y, v| {
                    let mult = (1.0 - v).min(1.0);
                    self.write_pixel(
                        (y + bounding_box.min.y as u32) as usize,
                        (x + bounding_box.min.x as u32) as usize,
                        color::RGB((c1 * mult) as u8, (c2 * mult) as u8, (c3 * mult) as u8),
                    )
                });
            }
        }
        // return the height and width of the drawn text so that refresh can be called on it
        mxcfb_rect {
            top: min_y as u32,
            left: min_x as u32,
            height: (max_y - min_y) as u32,
            width: (max_x - min_x) as u32,
        }
    }

    fn draw_rect(
        &mut self,
        y: usize,
        x: usize,
        height: usize,
        width: usize,
        border_px: usize,
        c: color,
    ) {
        // top horizontal
        self.draw_line(
            y as i32,
            x as i32,
            y as i32,
            (x + width) as i32,
            border_px,
            c,
        );

        // left vertical
        self.draw_line(
            y as i32,
            x as i32,
            (y + height) as i32,
            x as i32,
            border_px,
            c,
        );

        // bottom horizontal
        self.draw_line(
            (y + height) as i32,
            x as i32,
            (y + height) as i32,
            (x + width) as i32,
            border_px,
            c,
        );

        // right vertical
        self.draw_line(
            y as i32,
            (x + width) as i32,
            (y + height) as i32,
            (x + width) as i32,
            border_px,
            c,
        );
    }

    fn fill_rect(&mut self, y: usize, x: usize, height: usize, width: usize, c: color) {
        for ypos in y..y + height {
            for xpos in x..x + width {
                self.write_pixel(ypos, xpos, c);
            }
        }
    }

    fn clear(&mut self) {
        let h = self.var_screen_info.yres as usize;
        let line_length = self.fix_screen_info.line_length as usize;
        unsafe {
            libc::memset(
                self.frame.data() as *mut libc::c_void,
                std::i32::MAX,
                line_length * h,
            );
        }
    }
}
