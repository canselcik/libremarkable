extern crate librustpad;
extern crate image;
extern crate libc;
extern crate evdev;
extern crate chrono;

use chrono::{Local, DateTime};

use std::option::Option;
use std::time::Duration;
use std::thread::sleep;

use image::GenericImage;

use librustpad::fb;
use librustpad::mxc_types;
use mxc_types::{display_temp, waveform_mode, update_mode, dither_mode};

mod button_demo;

fn clear(framebuffer: &mut fb::Framebuffer) {
	let yres = framebuffer.var_screen_info.yres as usize;
	let xres = framebuffer.var_screen_info.xres as usize;
    framebuffer.clear();
    framebuffer.refresh(0, 0, 
				    	yres, xres,
				        update_mode::UPDATE_MODE_FULL,
				        waveform_mode::WAVEFORM_MODE_INIT,
				        display_temp::TEMP_USE_AMBIENT,
				        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
				        0, 0);
}

fn display_text(framebuffer: &mut fb::Framebuffer) {
	let draw_area: mxc_types::mxcfb_rect = framebuffer.draw_text(
        120, 120,
        "Remarkable Tablet".to_owned(),
        120,
        mxc_types::REMARKABLE_DARKEST,
    );
    let marker = framebuffer.refresh(
			        draw_area.top as usize,
			        draw_area.left as usize,
			        draw_area.height as usize,
			        draw_area.width as usize,
			        update_mode::UPDATE_MODE_PARTIAL,
			        waveform_mode::WAVEFORM_MODE_GC16_FAST,
			        display_temp::TEMP_USE_REMARKABLE_DRAW,
			        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
			        0, 0);
    framebuffer.wait_refresh_complete(marker);
}


fn loop_print_time(framebuffer: &mut fb::Framebuffer) {
	let mut draw_area: Option<mxc_types::mxcfb_rect> = None;
	loop {	
	    let dt: DateTime<Local> = Local::now();
	    match draw_area {
	    	Some(area) => framebuffer.draw_rect(area.top as usize, area.left as usize,
									            area.height as usize, area.width as usize,
										        mxc_types::REMARKABLE_BRIGHTEST),
	    	_ => {} 
	    }
	    
        draw_area = Some(framebuffer.draw_text(320, 120, format!("{}", dt.format("%F %r")),
        		100, mxc_types::REMARKABLE_DARKEST));
        match draw_area {
	    	Some(area) => {
			    let marker = framebuffer.refresh(
						        area.top as usize,
						        area.left as usize,
						        area.height as usize,
						        area.width as usize,
						        update_mode::UPDATE_MODE_PARTIAL,
						        waveform_mode::WAVEFORM_MODE_DU,
						        display_temp::TEMP_USE_REMARKABLE_DRAW,
						        dither_mode::EPDC_FLAG_USE_DITHERING_Y1,
					            0, 0);
			    framebuffer.wait_refresh_complete(marker);		
	    	},
	    	_ => {}
        }
	    sleep(Duration::from_millis(400));
	}
}

fn show_rust_logo(framebuffer: &mut fb::Framebuffer) {
	let img = image::load_from_memory(include_bytes!("../rustlang.bmp")).unwrap();
    framebuffer.draw_image(&img, 350, 110);
    let marker = framebuffer.refresh(
			        350, 110,
			        img.height() as usize,
			        img.width() as usize,
			        update_mode::UPDATE_MODE_PARTIAL,
			        waveform_mode::WAVEFORM_MODE_GC16_FAST,
			        display_temp::TEMP_USE_REMARKABLE_DRAW,
			        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
			        0, 0);
    framebuffer.wait_refresh_complete(marker);
}

fn main() {
    let mut framebuffer = fb::Framebuffer::new("/dev/fb0");
    
    clear(&mut framebuffer);

	display_text(&mut framebuffer);
	show_rust_logo(&mut framebuffer);

	// TODO: Maybe actually try to reason with the borrow checker here 
	unsafe {
	    let pointer = std::mem::transmute::<&mut fb::Framebuffer, usize>(&mut framebuffer);
	
		let et_t = std::thread::spawn(move|| {
			let ptr = std::mem::transmute::<usize, &mut fb::Framebuffer>(pointer);
			loop_print_time(ptr);       
	    });
	    let et_btn = std::thread::spawn(move || {
    		let ptr = std::mem::transmute::<usize, &mut fb::Framebuffer>(pointer);
    		librustpad::ev::start_evdev("/dev/input/event2".to_owned(), button_demo::DemoButtonHandler {
				framebuffer: ptr,
				name: "Physical Buttons".to_owned(),
				states: [false;3],
				last_trigger: std::time::SystemTime::now(),
    		});
	    });
	    et_t.join().unwrap();
	    et_btn.join().unwrap();
//	    let et_wacom = std::thread::spawn(move || {
//    		librustpad::ev::start_evdev("/dev/input/event0".to_owned(), librustpad::ev_debug::EvDeviceDebugHandler {
//				name: "Wacom".to_owned(),
//    		});
//	    });
//	    let et_mt = std::thread::spawn(move || {
//    		librustpad::ev::start_evdev("/dev/input/event1".to_owned(), librustpad::ev_debug::EvDeviceDebugHandler {
//				name: "MT".to_owned(),
//    		});
//	    });
//	    et_mt.join().unwrap();
//	    et_wacom.join().unwrap();
	}
    
}

