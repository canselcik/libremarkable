use evdev::Device;
use evdev::raw::input_event;

use ev;

pub struct EvDeviceDebugHandler {
    pub name: String,
}

impl ev::EvdevHandler for EvDeviceDebugHandler {
    fn on_init(&mut self, name: String, _device: &mut Device) {
        println!("INFO: '{0}' input device EPOLL initialized", name);
        self.name = name;
    }

    fn on_event(&mut self, ev: input_event) {
        println!(
            "[{0}] type: {1} code: {2} value: {3}",
            self.name,
            ev._type,
            ev.code,
            ev.value
        );
    }
}
