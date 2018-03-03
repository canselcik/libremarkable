#![feature(integer_atomics)]
#![feature(const_size_of)]
mod mxc_types;
mod fb;

#[macro_use]
extern crate ioctl_gen;

extern crate num_complex;
extern crate image;


#[allow(dead_code)]
fn main() {
    let mut framebuffer = fb::Framebuffer::new("/dev/fb0");
    framebuffer.var_screen_info.xres = 1872;
	framebuffer.var_screen_info.yres = 1404;
	framebuffer.var_screen_info.rotate = 1;
	framebuffer.var_screen_info.width = framebuffer.var_screen_info.xres;
	framebuffer.var_screen_info.height = framebuffer.var_screen_info.yres;
	framebuffer.var_screen_info.pixclock = 160000000;
	framebuffer.var_screen_info.left_margin = 32;
	framebuffer.var_screen_info.right_margin = 326;
	framebuffer.var_screen_info.upper_margin = 4;
	framebuffer.var_screen_info.lower_margin = 12;
	framebuffer.var_screen_info.hsync_len = 44;
	framebuffer.var_screen_info.vsync_len = 1;
	framebuffer.var_screen_info.sync = 0;
	framebuffer.var_screen_info.vmode = 0; // FB_VMODE_NONINTERLACED
	framebuffer.var_screen_info.accel_flags = 0;
	if !framebuffer.put_var_screeninfo() {
		panic!("FBIOPUT_VSCREENINFO failed");
	}

	framebuffer.clear();
	
    let imgx = framebuffer.var_screen_info.xres;
    let imgy = framebuffer.var_screen_info.yres;
      
	framebuffer.draw_text(630, 350, "Testing...".to_owned(), 120, 255);
	framebuffer.draw_square(550, 200, 120, 120, 0);

    let img = image::open("/home/root/lowbattery.bmp").unwrap().rotate270();
    framebuffer.draw_image(&img, 1050, 220);
	
    framebuffer.refresh(0, 0, imgy as usize, imgx as usize, 1, 1, 0, 0, 0, 0);
}
