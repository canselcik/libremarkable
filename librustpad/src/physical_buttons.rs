use evdev::Device;
use evdev::raw::input_event;

use ev;

pub enum PhysicalButton {
	LEFT, MIDDLE, RIGHT
}

pub struct PhysicalButtonHandler {
	pub name: String,
	pub callback: fn(PhysicalButton, u16),
}

impl PhysicalButtonHandler {
	pub fn get_instance(callback: fn(PhysicalButton, u16)) -> PhysicalButtonHandler {
		return PhysicalButtonHandler {
			name: "GPIO".to_owned(),
			callback: callback,
		};
	}
}

impl ev::EvdevHandler for PhysicalButtonHandler {
	fn on_init(&mut self, name: String, _device: &mut Device) {
		println!("INFO: '{0}' input device EPOLL initialized", name);
		self.name = name;
	}
	
	fn on_event(&mut self, ev: input_event) {
		match ev._type {
			0 => { /* safely ignored. sync event*/ },
			1 => {
				let p = match ev.code {
					102 => PhysicalButton::MIDDLE,
					105 => PhysicalButton::LEFT,
					106 => PhysicalButton::RIGHT,
					_ => return,
				};

				(self.callback)(p, ev.value as u16);
			}
			_ => {
				// Shouldn't happen
				println!("[WARN] Unknown event on PhysicalButtonHandler (type: {0})", ev._type);
			}
		}       
	}
}  