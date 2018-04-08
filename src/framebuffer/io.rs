#![allow(dead_code)]
use framebuffer;

impl<'a> framebuffer::FramebufferIO for framebuffer::core::Framebuffer<'a> {
    fn write_frame(&mut self, frame: &[u8]) {
        unsafe {
            let begin = self.frame.data() as *mut u8;
            for (i, elem) in frame.iter().enumerate() {
                *(begin.offset(i as isize)) = *elem;
            }
        }
    }

    fn write_pixel(&mut self, y: usize, x: usize, v: u8) {
        let w = self.var_screen_info.xres as usize;
        let h = self.var_screen_info.yres as usize;
        if y >= h || x >= w {
            return;
        }
        let line_length = self.fix_screen_info.line_length as usize;
        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as usize;
        let curr_index = (y * line_length + x * bytespp) as isize;

        let begin = self.frame.data() as *mut u8;
        unsafe {
            // TODO: Fix this packing scheme to be more flexible
            //  red:   0xF800F800
            //  green: 0x07E007E0
            //  blue:  0x001F001F
            *(begin.offset(curr_index)) = v;
            *(begin.offset(curr_index + 1)) = v;
            *(begin.offset(curr_index + 2)) = v;
            *(begin.offset(curr_index + 3)) = v;
        }
    }

    fn read_pixel(&mut self, y: usize, x: usize) -> u8 {
        let w = self.var_screen_info.xres as usize;
        let h = self.var_screen_info.yres as usize;
        if y >= h || x >= w {
            return 0;
        }
        let line_length = self.fix_screen_info.line_length as usize;
        let bytespp = (self.var_screen_info.bits_per_pixel / 8) as usize;
        let curr_index = y * line_length + x * bytespp;
        return self.read_offset(curr_index as isize);
    }

    fn read_offset(&mut self, ofst: isize) -> u8 {
        unsafe {
            let begin = self.frame.data() as *mut u8;
            return *(begin.offset(ofst));
        }
    }
}
