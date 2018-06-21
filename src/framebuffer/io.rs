#![allow(dead_code)]
use framebuffer;
use framebuffer::common;

use image::{ImageBuffer, Rgba, RgbaImage};

impl<'a> framebuffer::FramebufferIO for framebuffer::core::Framebuffer<'a> {
    fn write_frame(&mut self, frame: &[u8]) {
        unsafe {
            let begin = self.frame.data() as *mut u8;
            for (i, elem) in frame.iter().enumerate() {
                *(begin.offset(i as isize)) = *elem;
            }
        }
    }

    fn write_pixel(&mut self, y: usize, x: usize, v: framebuffer::common::color) {
        let w = self.var_screen_info.xres as usize;
        let h = self.var_screen_info.yres as usize;
        if y >= h || x >= w {
            return;
        }
        let line_length = self.fix_screen_info.line_length as usize;
        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as usize;
        let curr_index = (y * line_length + x * bytespp) as isize;

        let begin = self.frame.data() as *mut u8;
        let components = v.as_native();
        unsafe {
            *(begin.offset(curr_index)) = components[0];
            *(begin.offset(curr_index + 1)) = components[1];
            *(begin.offset(curr_index + 2)) = components[2];
            *(begin.offset(curr_index + 3)) = components[3];
        }
    }

    fn read_pixel(&self, y: usize, x: usize) -> framebuffer::common::color {
        let w = self.var_screen_info.xres as usize;
        let h = self.var_screen_info.yres as usize;
        if y >= h || x >= w {
            error!("Attempting to read pixel out of range. Returning a white pixel.");
            return framebuffer::common::color::WHITE;
        }
        let line_length = self.fix_screen_info.line_length as usize;
        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as usize;
        let curr_index = y * line_length + x * bytespp;

        framebuffer::common::color::NATIVE_COMPONENTS(
            self.read_offset(curr_index as isize),
            self.read_offset(curr_index as isize + 1),
            self.read_offset(curr_index as isize + 2),
            self.read_offset(curr_index as isize + 3),
        )
    }

    fn read_offset(&self, ofst: isize) -> u8 {
        unsafe {
            let begin = self.frame.data() as *mut u8;
            return *(begin.offset(ofst));
        }
    }

    fn dump_region(&self, rect: common::mxcfb_rect) -> Result<RgbaImage, &'static str> {
        if rect.width == 0 || rect.height == 0 {
            return Err("Unable to dump a region with zero height/width");
        }
        if rect.top + rect.height > self.var_screen_info.height {
            return Err("Vertically out of bounds");
        }
        if rect.left + rect.width > self.var_screen_info.width {
            return Err("Horizontally out of bounds");
        }

        let mut buffer: RgbaImage = ImageBuffer::new(rect.width, rect.height);
        for y in 0..rect.height {
            for x in 0..rect.width {
                buffer.put_pixel(
                    x,
                    y,
                    Rgba {
                        data: self.read_pixel((rect.top + y) as usize, (rect.left + x) as usize)
                            .as_native(),
                    },
                );
            }
        }
        return Ok(buffer);
    }

    fn restore_region(
        &mut self,
        rect: common::mxcfb_rect,
        data: &RgbaImage,
    ) -> Result<u32, &'static str> {
        if rect.width == 0 || rect.height == 0 {
            return Err("Unable to restore a region with zero height/width");
        }
        if rect.top + rect.height > self.var_screen_info.height {
            return Err("Vertically out of bounds");
        }
        if rect.left + rect.width > self.var_screen_info.width {
            return Err("Horizontally out of bounds");
        }

        let mut written: u32 = 0;
        for y in 0..rect.height {
            for x in 0..rect.width {
                self.write_pixel(
                    (rect.top + y) as usize,
                    (rect.left + x) as usize,
                    common::color::from_native(data.get_pixel(x, y).data),
                );
                written += 1;
            }
        }
        return Ok(written);
    }
}
