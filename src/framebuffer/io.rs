use log::error;

use crate::framebuffer;
use crate::framebuffer::cgmath;
use crate::framebuffer::common;

impl<'a> framebuffer::FramebufferIO for framebuffer::core::Framebuffer<'a> {
    fn write_frame(&mut self, frame: &[u8]) {
        let begin = self.frame.as_mut_ptr();
        unsafe {
            for (i, elem) in frame.iter().enumerate() {
                begin.add(i).write_volatile(*elem);
            }
        }
    }

    #[inline]
    fn write_pixel(&mut self, pos: cgmath::Point2<i32>, col: framebuffer::common::color) {
        let w = self.var_screen_info.xres as usize;
        let h = self.var_screen_info.yres as usize;
        if pos.y < 0 || pos.x < 0 {
            return;
        }
        if pos.y as usize >= h || pos.x as usize >= w {
            return;
        }
        let line_length = self.fix_screen_info.line_length as isize;
        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as isize;
        let curr_index = pos.y as isize * line_length + pos.x as isize * bytespp;

        let begin = self.frame.as_mut_ptr();
        let components = col.as_native();
        unsafe {
            begin.offset(curr_index).write_volatile(components[0]);
            begin.offset(curr_index + 1).write_volatile(components[1]);
        }
    }

    fn read_pixel(&self, pos: cgmath::Point2<u32>) -> framebuffer::common::color {
        let w = self.var_screen_info.xres as usize;
        let h = self.var_screen_info.yres as usize;
        if pos.y as usize >= h || pos.x as usize >= w {
            error!("Attempting to read pixel out of range. Returning a white pixel.");
            return framebuffer::common::color::WHITE;
        }
        let line_length = self.fix_screen_info.line_length as usize;
        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as usize;
        let curr_index = pos.y as usize * line_length + pos.x as usize * bytespp;

        let begin = self.frame.as_mut_ptr();
        let (c1, c2) = unsafe {
            (
                begin.add(curr_index).read_volatile(),
                begin.add(curr_index + 1).read_volatile(),
            )
        };
        framebuffer::common::color::NATIVE_COMPONENTS(c1, c2)
    }

    fn read_offset(&self, ofst: isize) -> u8 {
        unsafe {
            let begin = self.frame.as_mut_ptr();
            begin.offset(ofst).read_volatile()
        }
    }

    fn dump_region(&self, rect: common::mxcfb_rect) -> Result<Vec<u8>, &'static str> {
        if rect.width == 0 || rect.height == 0 {
            return Err("Unable to dump a region with zero height/width");
        }
        if rect.top + rect.height > self.var_screen_info.height {
            return Err("Vertically out of bounds");
        }
        if rect.left + rect.width > self.var_screen_info.width {
            return Err("Horizontally out of bounds");
        }

        let line_length = self.fix_screen_info.line_length as u32;
        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as usize;
        let inbuffer = self.frame.as_ptr();
        let mut outbuffer: Vec<u8> =
            Vec::with_capacity(rect.height as usize * rect.width as usize * bytespp);
        let outbuffer_ptr = outbuffer.as_mut_ptr();

        let mut written = 0;
        let chunk_size = bytespp * rect.width as usize;
        for row in 0..rect.height {
            let curr_index = (row + rect.top) * line_length + (bytespp * rect.left as usize) as u32;
            unsafe {
                inbuffer
                    .add(curr_index as usize)
                    .copy_to_nonoverlapping(outbuffer_ptr.add(written), chunk_size);
            }
            written += chunk_size;
        }
        unsafe {
            outbuffer.set_len(written);
        }

        Ok(outbuffer)
    }

    fn restore_region(
        &mut self,
        rect: common::mxcfb_rect,
        data: &[u8],
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

        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as usize;
        if data.len() as u32 != rect.width * rect.height * bytespp as u32 {
            return Err("Cannot restore region due to mismatched size");
        }

        let line_length = self.fix_screen_info.line_length as u32;
        let chunk_size = bytespp * rect.width as usize;
        let outbuffer = self.frame.as_mut_ptr();
        let inbuffer = data.as_ptr();
        let mut written: u32 = 0;
        for y in 0..rect.height {
            let curr_index = (y + rect.top) * line_length + (bytespp * rect.left as usize) as u32;
            unsafe {
                outbuffer
                    .add(curr_index as usize)
                    .copy_from(inbuffer.add(written as usize), chunk_size);
            }
            written += chunk_size as u32;
        }
        Ok(written)
    }
}
