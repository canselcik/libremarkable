use evdev::Device;
use evdev::raw::input_event;

use std;
use std::time::SystemTime;

use librustpad::fb;
use librustpad::ev;
use librustpad::mxc_types;
use librustpad::mxc_types::{update_mode,waveform_mode,display_temp,dither_mode};

pub struct DemoButtonHandler {
	pub framebuffer: &'static mut fb::Framebuffer<'static>,
	pub name: String,
	pub states: [bool;3],
	pub last_trigger: SystemTime,
	pub xochitl_child: Option<std::process::Child>,
}

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

impl ev::EvdevHandler for DemoButtonHandler {
	fn on_init(&mut self, name: String, _device: &mut Device) {
		println!("INFO: '{0}' input device EPOLL initialized", name);
		self.name = name;
		self.states = [false; 3];
		self.last_trigger = SystemTime::now();
	}
	
	fn on_event(&mut self, ev: input_event) {
		match ev._type {
			0 => { /* safely ignored. only sent after the meaningful events to sync */ },
			1 => { 
				let color = match ev.value {
					0 => mxc_types::REMARKABLE_BRIGHTEST,
					_ => mxc_types::REMARKABLE_DARKEST,					
				};
				let x_offset = match ev.code {
					105 => {
						self.states[0] = ev.value == 1;
						50
					},
					102 => {
						self.states[1] = ev.value == 1;
						640
					},
					106 => {
						self.states[2] = ev.value == 1;
						1250
					}
					_ => 9999, /* out of bounds so no artifacts */
				};
				             
				self.framebuffer.draw_rect(1500, x_offset, 125, 125, color);
				self.framebuffer.refresh(1500, x_offset, 125, 125,
										 update_mode::UPDATE_MODE_PARTIAL,
										 waveform_mode::WAVEFORM_MODE_DU,
										 display_temp::TEMP_USE_PAPYRUS,
									     dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
									     0, 0);
			}
			_ => {
				// Shouldn't happen
				println!("[WARN] Unknown event on PhysicalButtonHandler (type: {0})", ev._type);
			}
		}
		self.eval_key_combo();
	}
}  
    
impl DemoButtonHandler {
	pub fn get_instance(ptr: &'static mut fb::Framebuffer<'static>) -> DemoButtonHandler {
		return DemoButtonHandler {
			framebuffer: ptr as &'static mut fb::Framebuffer<'static>,
			name: "Physical Buttons".to_owned(),
			states: [false;3],
			xochitl_child: None,
			last_trigger: std::time::SystemTime::now(),
		};
	}
	
	fn eval_key_combo(&mut self) {
		if self.states[0] && !self.states[1] && self.states[2] {
			let now = SystemTime::now();
			if now.duration_since(self.last_trigger).unwrap().as_secs() > 1 {
				std::process::Command::new("sh")
					.arg("-c")
					.arg("if pidof xochitl; then systemctl stop xochitl; else systemctl start xochitl; fi")
					.output().unwrap();			
				self.last_trigger = now;
				clear(&mut self.framebuffer);
			}
	
		}
	}
}