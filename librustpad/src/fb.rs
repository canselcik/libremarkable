#![allow(dead_code)]

extern crate libc;
use self::libc::ioctl;

extern crate mmap;
use self::mmap::MemoryMap;

extern crate image;
use image::DynamicImage;

use std;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicU32, Ordering};
use std::fs::{OpenOptions, File};

use mxc_types::{mxcfb_update_data,mxcfb_rect,VarScreeninfo,FixScreeninfo,FBIOGET_FSCREENINFO,FBIOGET_VSCREENINFO,FBIOPUT_VSCREENINFO};

extern crate rusttype;
use self::rusttype::{Font, FontCollection, Scale, point};

pub struct Framebuffer<'a> {
    pub device: File,
    pub frame: MemoryMap,
    pub marker: std::sync::atomic::AtomicU32,
    pub default_font: Font<'a>,
    pub var_screen_info: VarScreeninfo,
    pub fix_screen_info: FixScreeninfo,
}

impl<'a> Framebuffer<'a> {
    pub fn new(path_to_device: &str) -> Framebuffer {
        let device = OpenOptions::new().read(true).write(true).open(path_to_device).unwrap();

        let var_screen_info = Framebuffer::get_var_screeninfo(&device);
        let fix_screen_info = Framebuffer::get_fix_screeninfo(&device);

        let frame_length = (fix_screen_info.line_length * var_screen_info.yres) as usize;
        let mem_map = MemoryMap::new(frame_length, &[
            mmap::MapOption::MapReadable,
            mmap::MapOption::MapWritable,
            mmap::MapOption::MapFd(device.as_raw_fd()),
            mmap::MapOption::MapOffset(0),
            mmap::MapOption::MapNonStandardFlags(libc::MAP_SHARED)
        ]).unwrap();

         // Load the font
	    let font_data = include_bytes!("/usr/share/fonts/TTF/DejaVuSans.ttf");
	    let collection = FontCollection::from_bytes(font_data as &[u8]);

        return Framebuffer {
        	marker: AtomicU32::new(1),
            device: device,
            frame: mem_map,
            default_font: collection.into_font().unwrap(),
            var_screen_info: var_screen_info,
            fix_screen_info: fix_screen_info,
        };
    }

    // TODO:  waiting update etc. markers
	pub fn refresh(&mut self, y: usize, x: usize, height: usize, width: usize, update_mode: u32,
		waveform_mode: u32, temperature: i32, flags: u32, dither_mode: i32, quant_bit: i32) {
	  const SEND_UPDATE_IOCTL: u32 = iow!(b'F', 0x2E, std::mem::size_of::<mxcfb_update_data>());
	  let whole = mxcfb_update_data {
	  	update_mode: update_mode,
	  	update_marker: *self.marker.get_mut() as u32,
	  	waveform_mode: waveform_mode,
	  	temp: temperature,
	  	flags: flags,
	  	quant_bit: quant_bit,
	  	dither_mode: dither_mode,
	  	update_region: mxcfb_rect {
	  		top: y as u32,
	  		left: x as u32,
	  		height: height as u32,
	  		width: width as u32
	  	}, ..Default::default() };
	  let pt: *const mxcfb_update_data = &whole;
	  unsafe {
	    libc::ioctl(self.device.as_raw_fd(), SEND_UPDATE_IOCTL, pt);
	  }
	  // TODO: Do proper compare and swap
	  self.marker.swap(whole.update_marker + 1, Ordering::Relaxed);
	}

    pub fn draw_image(&mut self, img: &DynamicImage, top: usize, left: usize) {
    	for (x, y, pixel) in img.to_luma().enumerate_pixels() {
    		self.write_pixel(top + y as usize, left + x as usize, pixel.data[0]);
    	}
    }

	pub fn draw_text(&mut self, y: usize, x: usize, text: String, size: usize, color: u8) {
	    let scale = Scale {x: size as f32, y: size as f32};

	    // The starting positioning of the glyphs (top left corner)
	    let start = point(x as f32, y as f32);

		let dfont = &mut self.default_font.clone();
	    // Loop through the glyphs in the text, positing each one on a line
	    for glyph in dfont.layout(&text, scale, start) {
	        if let Some(bounding_box) = glyph.pixel_bounding_box() {
	            // Draw the glyph into the image per-pixel by using the draw closure


	            glyph.draw(|x, y, v| self.write_pixel(
	                // Offset the position by the glyph bounding box
		            (y + bounding_box.min.y as u32) as usize,
	                (x + bounding_box.min.x as u32) as usize,
	                // Turn the coverage into an alpha value
	                //  [colour.0, colour.1, colour.2, (v * 255.0) as u8]
					//(((0u16 | !colour.0) << 11) | ((0u16 | !colour.1) << 5) | (0u16 | !colour.2)) as u8
					//colour.0 * (v*10.0) as u8, !colour.1 * (v*10.0) as u8, !colour.2 * (v*10.0) as u8
					!((v*color as f32) as u8)
	            ));

	        }
	    }
	}

	pub fn draw_square(&mut self, y: usize, x: usize, height: usize, width: usize, color: u8) {
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

    pub fn write_frame(&mut self, frame: &[u8]) {
    	unsafe {
	    	let begin = self.frame.data() as *mut u8;
	    	for (i, elem) in frame.iter().enumerate() {
				*(begin.offset(i as isize)) = *elem;
			}
    	}
    }

    pub fn write_pixel(&mut self, y: usize, x: usize, v: u8) {
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
	    	// TODO: Figure out this packing
			*(begin.offset(curr_index)) = v;
			*(begin.offset(curr_index+1)) = v;
			*(begin.offset(curr_index+2)) = v;
	    }
    }

    pub fn read_pixel(&mut self, y: usize, x: usize) -> u8 {
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

    pub fn read_offset(&mut self, ofst: isize) -> u8 {
    	unsafe {
    		let begin = self.frame.data() as *mut u8;
    		return *(begin.offset(ofst));
    	}
    }

    ///Creates a FixScreeninfo struct and fills it using ioctl.
    pub fn get_fix_screeninfo(device: &File) -> FixScreeninfo  {
        let mut info: FixScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_FSCREENINFO, &mut info) };
        if result != 0 {
        	panic!("FBIOGET_FSCREENINFO failed");
        }
        return info;
    }

    ///Creates a VarScreeninfo struct and fills it using ioctl.
    pub fn get_var_screeninfo(device: &File) -> VarScreeninfo {
        let mut info: VarScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_VSCREENINFO, &mut info) };
        if result != 0 {
        	panic!("FBIOGET_VSCREENINFO failed");
        }
        return info;
    }

    pub fn put_var_screeninfo(&mut self) -> bool {
        let result = unsafe { ioctl(self.device.as_raw_fd(), FBIOPUT_VSCREENINFO, &mut self.var_screen_info) };
        return result == 0;
    }
}
