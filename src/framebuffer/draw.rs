use std;

use image::RgbImage;
use libc;
use line_drawing;
use rusttype::{point, Scale};

use framebuffer;
use framebuffer::cgmath::*;
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
fn sample_bezier(
    startpt: Point2<f32>,
    ctrlpt: Point2<f32>,
    endpt: Point2<f32>,
) -> Vec<Point2<f32>> {
    let mut points = Vec::new();
    let mut lastpt = (-100, -100);
    for i in 0..1000 {
        let t = (i as f32) / 1000.0;
        let precisept = Point2 {
            x: (1.0 - t).powf(2.0) * startpt.x
                + 2.0 * (1.0 - t) * t * ctrlpt.x
                + t.powf(2.0) * endpt.x,
            y: (1.0 - t).powf(2.0) * startpt.y
                + 2.0 * (1.0 - t) * t * ctrlpt.y
                + t.powf(2.0) * endpt.y,
        };
        let pt = (precisept.x as i32, precisept.y as i32);
        // prevent oversampling
        if pt != lastpt {
            points.push(precisept);
            lastpt = pt;
        }
    }
    points
}

impl<'a> framebuffer::FramebufferDraw for core::Framebuffer<'a> {
    fn draw_image(&mut self, img: &RgbImage, pos: Point2<i32>) -> mxcfb_rect {
        for (x, y, pixel) in img.enumerate_pixels() {
            let pixel_pos = pos + vec2(x as i32, y as i32);
            self.write_pixel(
                pixel_pos.cast().unwrap(),
                color::RGB(pixel.data[0], pixel.data[1], pixel.data[2]),
            );
        }
        mxcfb_rect {
            top: pos.y as u32,
            left: pos.x as u32,
            width: img.width(),
            height: img.height(),
        }
    }

    fn draw_line(
        &mut self,
        start: Point2<i32>,
        end: Point2<i32>,
        width: u32,
        v: color,
    ) -> mxcfb_rect {
        // Create local variables for moving start point
        let mut x0 = start.x;
        let mut y0 = start.y;

        // Get absolute x/y offset
        let dx = if x0 > end.x { x0 - end.x } else { end.x - x0 };
        let dy = if y0 > end.y { y0 - end.y } else { end.y - y0 };

        // Get slopes
        let sx = if x0 < end.x { 1 } else { -1 };
        let sy = if y0 < end.y { 1 } else { -1 };

        // Initialize error
        let mut err = if dx > dy { dx } else { -dy } / 2;
        let mut err2;

        let (mut min_x, mut max_x, mut min_y, mut max_y) = (x0, x0, y0, y0);
        loop {
            // Set pixel
            match width {
                1 => self.write_pixel(
                    Point2 {
                        x: x0 as i32,
                        y: y0 as i32,
                    },
                    v,
                ),
                _ => self.fill_rect(
                    Point2 {
                        x: (x0 - (width / 2) as i32),
                        y: (y0 - (width / 2) as i32),
                    },
                    Vector2 { x: width, y: width },
                    v,
                ),
            }

            max_y = max!(max_y, y0);
            min_y = min!(min_y, y0);
            min_x = min!(min_x, x0);
            max_x = max!(max_x, x0);

            // Check end condition
            if x0 == end.x && y0 == end.y {
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

    fn draw_circle(&mut self, pos: Point2<i32>, rad: u32, v: color) -> mxcfb_rect {
        for (x, y) in line_drawing::BresenhamCircle::new(pos.x as i32, pos.y as i32, rad as i32) {
            self.write_pixel(
                Point2 {
                    x: x as i32,
                    y: y as i32,
                },
                v,
            );
        }
        mxcfb_rect {
            top: pos.y as u32 - rad as u32,
            left: pos.x as u32 - rad as u32,
            width: 2 * rad as u32,
            height: 2 * rad as u32,
        }
    }

    fn fill_circle(&mut self, pos: Point2<i32>, rad: u32, v: color) -> mxcfb_rect {
        for current in { 1..rad + 1 } {
            for (x, y) in
                line_drawing::BresenhamCircle::new(pos.x as i32, pos.y as i32, current as i32)
            {
                self.write_pixel(
                    Point2 {
                        x: x as i32,
                        y: y as i32,
                    },
                    v,
                );
            }
        }
        mxcfb_rect {
            top: pos.y as u32 - rad as u32,
            left: pos.x as u32 - rad as u32,
            width: 2 * rad as u32,
            height: 2 * rad as u32,
        }
    }

    fn draw_bezier(
        &mut self,
        startpt: Point2<f32>,
        ctrlpt: Point2<f32>,
        endpt: Point2<f32>,
        width: f32,
        v: color,
    ) -> mxcfb_rect {
        let mut bbox = mxcfb_rect {
            top: startpt.y.max(0.0) as u32,
            left: startpt.x.max(0.0) as u32,
            width: 0,
            height: 0,
        };
        for pt in sample_bezier(startpt, ctrlpt, endpt) {
            let approx = pt.cast().unwrap();
            bbox = bbox.merge_pixel(&pt.cast().unwrap());

            // Set pixel
            match width as u32 {
                1 => self.write_pixel(approx, v),
                _ => self.fill_rect(
                    approx
                        .sub_element_wise((width / 2.0) as i32)
                        .cast()
                        .unwrap(),
                    Vector2 {
                        x: width as u32,
                        y: width as u32,
                    },
                    v,
                ),
            };
        }
        let margin = ((width + 1.0) / 2.0) as u32;
        bbox.expand(margin)
    }

    fn draw_text(
        &mut self,
        pos: Point2<f32>,
        text: String,
        size: f32,
        col: color,
        dryrun: bool,
    ) -> mxcfb_rect {
        let scale = Scale {
            x: size as f32,
            y: size as f32,
        };

        // The starting positioning of the glyphs (top left corner)
        let start = point(pos.x, pos.y);

        let dfont = &mut self.default_font.clone();

        let mut min_y = pos.y.floor().max(0.0) as u32;
        let mut max_y = pos.y.ceil().max(0.0) as u32;
        let mut min_x = pos.x.floor().max(0.0) as u32;
        let mut max_x = pos.x.ceil().max(0.0) as u32;

        let components = col.to_rgb8();
        let c1 = f32::from(255 - components[0]);
        let c2 = f32::from(255 - components[1]);
        let c3 = f32::from(255 - components[2]);

        // Loop through the glyphs in the text, positing each one on a line
        for glyph in dfont.layout(&text, scale, start) {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                // Draw the glyph into the image per-pixel by using the draw closure
                let bbmax_y = bounding_box.max.y as u32;
                let bbmax_x = bounding_box.max.x as u32;
                let bbmin_y = bounding_box.min.y as u32;
                let bbmin_x = bounding_box.min.x as u32;
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
                        Point2 {
                            x: (x + bounding_box.min.x as u32) as i32,
                            y: (y + bounding_box.min.y as u32) as i32,
                        },
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

    fn draw_rect(&mut self, pos: Point2<i32>, size: Vector2<u32>, border_px: u32, c: color) {
        let top_left = pos;
        let top_right = pos + vec2(size.x as i32, 0);
        let bottom_left = pos + vec2(0, size.y as i32);
        let bottom_right = pos + size.cast().unwrap();

        // top horizontal
        self.draw_line(top_left, top_right, border_px, c);

        // left vertical
        self.draw_line(top_left, bottom_left, border_px, c);

        // bottom horizontal
        self.draw_line(top_right, bottom_right, border_px, c);

        // right vertical
        self.draw_line(bottom_left, bottom_right, border_px, c);
    }

    fn fill_rect(&mut self, pos: Point2<i32>, size: Vector2<u32>, c: color) {
        for ypos in pos.y..pos.y + size.y as i32 {
            for xpos in pos.x..pos.x + size.x as i32 {
                self.write_pixel(
                    Point2 {
                        x: xpos as i32,
                        y: ypos as i32,
                    },
                    c,
                );
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
