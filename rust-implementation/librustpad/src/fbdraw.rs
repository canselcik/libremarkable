use fb;
use image::DynamicImage;
use mxc_types::mxcfb_rect;
use rusttype::{Scale, point};
use libc;
use line_drawing;

use image::GenericImage;

macro_rules! min {
        ($x: expr) => ($x);
        ($x: expr, $($z: expr),+) => (::std::cmp::min($x, min!($($z),*)));
}

macro_rules! max {
        ($x: expr) => ($x);
        ($x: expr, $($z: expr),+) => (::std::cmp::max($x, max!($($z),*)));
}

impl<'a> fb::Framebuffer<'a> {
    pub fn draw_image(&mut self, img: &DynamicImage, top: usize, left: usize) -> mxcfb_rect {
        for (x, y, pixel) in img.to_luma().enumerate_pixels() {
            self.write_pixel(top + y as usize, left + x as usize, pixel.data[0]);
        }
        return mxcfb_rect {
            top: top as u32,
            left: left as u32,
            width: img.width(),
            height: img.height(),
        };
    }

    pub fn draw_line(&mut self, y0: i32, x0: i32, y1: i32, x1: i32, color: u8) -> mxcfb_rect {
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

        let mut minx = 0;
        let mut miny = 0;
        let mut maxx = 0;
        let mut maxy = 0;
        loop {
            // Set pixel
            self.write_pixel(y0 as usize, x0 as usize, color);
            maxy = max!(maxy, y0);
            miny = min!(miny, y0);
            minx = min!(minx, x0);
            maxx = max!(maxx, x0);

            // Check end condition
            if x0 == x1 && y0 == y1 { break; };

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

        return mxcfb_rect {
            top: miny as u32,
            left: minx as u32,
            width: (maxx - minx) as u32,
            height: (maxy - miny) as u32,
        };
    }

    pub fn draw_circle(&mut self, y: usize, x: usize, rad: usize, color: u8) -> mxcfb_rect {
        for (x, y) in line_drawing::BresenhamCircle::new(x as i32, y as i32, rad as i32) {
            self.write_pixel(y as usize, x as usize, color);
        }
        return mxcfb_rect {
            top: y as u32 - rad as u32,
            left: x as u32 - rad as u32,
            width: 2 * rad as u32,
            height: 2 * rad as u32,
        };
    }

    pub fn fill_circle(&mut self, y: usize, x: usize, rad: usize, color: u8) -> mxcfb_rect {
        for current in { 1..rad + 1 } {
            for (x, y) in line_drawing::BresenhamCircle::new(x as i32, y as i32, current as i32) {
                self.write_pixel(y as usize, x as usize, color);
            }
        }
        return mxcfb_rect {
            top: y as u32 - rad as u32,
            left: x as u32 - rad as u32,
            width: 2 * rad as u32,
            height: 2 * rad as u32,
        };
    }

    pub fn draw_bezier(&mut self, startpt: (f32, f32), ctrlpt: (f32, f32), endpt: (f32, f32), color: u8) -> mxcfb_rect {
        let mut upperleft: (usize, usize) = (0, 0);
        let mut lowerright: (usize, usize) = (0, 0);
        for pt in self.sample_bezier(startpt, ctrlpt, endpt) {
            let approx = (pt.0 as usize, pt.1 as usize);
            upperleft.1 = min!(upperleft.1, approx.1);
            upperleft.0 = min!(upperleft.0, approx.0);
            lowerright.1 = max!(lowerright.1, approx.1);
            lowerright.0 = max!(lowerright.0, approx.0);
            self.write_pixel(approx.1, approx.0, color);
        }
        return mxcfb_rect {
            top: upperleft.1 as u32,
            left: upperleft.0 as u32,
            width: (lowerright.0 - upperleft.0) as u32,
            height: (lowerright.1 - upperleft.1) as u32,
        };
    }

    fn sample_bezier(&mut self, startpt: (f32, f32), ctrlpt: (f32, f32), endpt: (f32, f32)) -> Vec<(f32, f32)> {
        let mut points = Vec::new();
        let mut lastpt = (-100, -100);
        for i in 0..1000 {
            let t = (i as f32) / 1000.0;
            let precisept = (
                (1.0 - t).powf(2.0) * startpt.0 + 2.0 * (1.0 - t) * t * ctrlpt.0
                    + t.powf(2.0) * endpt.0,
                (1.0 - t).powf(2.0) * startpt.1 + 2.0 * (1.0 - t) * t * ctrlpt.1
                    + t.powf(2.0) * endpt.1,
            );
            let pt = (precisept.0 as i32, precisept.1 as i32);
            // prevent oversampling
            if pt != lastpt {
                points.push(precisept);
                lastpt = pt;
            }
        }
        return points;
    }

    pub fn draw_text(
        &mut self,
        y: usize,
        x: usize,
        text: String,
        size: usize,
        color: u8,
    ) -> mxcfb_rect {
        let scale = Scale {
            x: size as f32,
            y: size as f32,
        };

        // The starting positioning of the glyphs (top left corner)
        let start = point(x as f32, y as f32);

        let dfont = &mut self.default_font.clone();

        let mut miny = y;
        let mut maxy = y;
        let mut minx = x;
        let mut maxx = x;

        // Loop through the glyphs in the text, positing each one on a line
        for glyph in dfont.layout(&text, scale, start) {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                // Draw the glyph into the image per-pixel by using the draw closure
                let bbmaxy = bounding_box.max.y as usize;
                let bbmaxx = bounding_box.max.x as usize;
                let bbminy = bounding_box.min.y as usize;
                let bbminx = bounding_box.min.x as usize;
                if bbmaxy > maxy {
                    maxy = bbmaxy;
                }
                if bbmaxx > maxx {
                    maxx = bbmaxx;
                }
                if bbminy < miny {
                    miny = bbminy;
                }
                if bbminx < minx {
                    minx = bbminx;
                }
                glyph.draw(|x, y, v| {
                    self.write_pixel(
                        (y + bounding_box.min.y as u32) as usize,
                        (x + bounding_box.min.x as u32) as usize,
                        !((v * !color as f32) as u8),
                    )
                });
            }
        }
        // return the height and width of the drawn text so that refresh can be called on it
        return mxcfb_rect {
            top: miny as u32,
            left: minx as u32,
            height: (maxy - miny) as u32,
            width: (maxx - minx) as u32,
        };
    }

    pub fn fill_rect(&mut self, y: usize, x: usize, height: usize, width: usize, color: u8) {
        for ypos in y..y + height {
            for xpos in x..x + width {
                self.write_pixel(ypos, xpos, color);
            }
        }
    }

    pub fn clear(&mut self) {
        let h = self.var_screen_info.yres as usize;
        let line_length = self.fix_screen_info.line_length as usize;
        unsafe {
            libc::memset(self.frame.data() as *mut libc::c_void, 255, line_length * h);
        }
    }
}
