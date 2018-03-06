use fb;
use image::DynamicImage;
use mxc_types::mxcfb_rect;
use rusttype::{Scale, point};
use libc;
use line_drawing;

use image::GenericImage;

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
