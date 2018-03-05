use evdev::Device;
use evdev::raw::input_event;

use fb;
use ev;

pub struct PhysicalButtonHandler<'a> {
	pub framebuffer: &'a mut fb::Framebuffer<'a>,
	pub name: String,
}

/*
	INFO: 'gpio-keys' input device EPOLL initialized
	INFO: 'Wacom I2C Digitizer' input device EPOLL initialized
	INFO: 'cyttsp5_mt' input device EPOLL initialized
	gpio-keys: [t: 1, c: 105, v: 1]
	gpio-keys: [t: 0, c: 0, v: 0]
	gpio-keys: [t: 1, c: 105, v: 0]
	gpio-keys: [t: 0, c: 0, v: 0]
	
	gpio-keys: [t: 1, c: 102, v: 1]
	gpio-keys: [t: 0, c: 0, v: 0]
	gpio-keys: [t: 1, c: 102, v: 0]
	gpio-keys: [t: 0, c: 0, v: 0]
	
	gpio-keys: [t: 1, c: 106, v: 1]
	gpio-keys: [t: 0, c: 0, v: 0]
	gpio-keys: [t: 1, c: 106, v: 0]
	gpio-keys: [t: 0, c: 0, v: 0]
*/
impl ev::EvdevHandler for PhysicalButtonHandler<'static> {
	fn on_init(&mut self, name: String, _device: &mut Device) {
		println!("INFO: '{0}' input device EPOLL initialized", name);
		self.name = name;
	}
	
	fn on_event(&mut self, ev: input_event) {
		match ev._type {
			0 => { /* safely ignored. only sent after the meaningful events to sync */ },
			1 => {
				let human_button = match ev.code {
					102 => "MIDDLE",
					105 => "LEFT",
					106 => "RIGHT",
					_ => "<UNKNOWN>", /* shouldn't happen */
				};
				let human_event = match ev.value {
					0 => "UNPRESSED",
					1 => "PRESSED",
					_ => "<UNKNOWN>", /* shouldn't happen */					
				};
				println!("{0} button is {1}", human_button, human_event);
			}
			_ => {
				// Shouldn't happen
				println!("[WARN] Unknown event on PhysicalButtonHandler (type: {0})", ev._type);
			}
		}
        
	}
}  