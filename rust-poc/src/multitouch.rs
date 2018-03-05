use evdev::Device;
use evdev::raw::input_event;

use librustpad::ev;

pub struct MultitouchHandler {
	pub name: String,
	pub last_touch_id: u16,
	pub last_touch_size: u8,
	pub currently_touching: bool,
}

impl ev::EvdevHandler for MultitouchHandler {
	fn on_init(&mut self, name: String, _device: &mut Device) {
		println!("INFO: '{0}' input device EPOLL initialized", name);
		self.name = name;
	}
	
	fn on_event(&mut self, ev: input_event) {
		match ev._type {
			0 => { /* sync */ },
			3 => { // Absolute
				match ev.code {
					49 => self.last_touch_size = ev.value,
					57 => {
						match ev.value {
							-1 => self.currently_touching = false,
							_  => self.last_touch_id = ev.value,
						}
					}
					
				}
			},
			_ => {
				println!("Unknown event type for {0} [type: {1} code: {2} value: {3}]",
					self.name, ev._type, ev.code, ev.value);				
			}
		}
		
	}
}  