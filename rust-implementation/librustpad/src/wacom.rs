use evdev::Device;
use evdev::raw::input_event;

use ev;
use mxc_types;

const HSCALAR: f32 = (mxc_types::DISPLAYWIDTH as f32) / (mxc_types::WACOMWIDTH as f32);
const VSCALAR: f32 = (mxc_types::DISPLAYHEIGHT as f32) / (mxc_types::WACOMHEIGHT as f32);

/* Very basic handler (does hover tracking right now -- more soon to come) for Wacom digitizer on Remarkable Paper Tablet */

pub struct WacomHandler {
    pub name: String,
    pub last_x: u16,
    pub last_y: u16,
    pub on_touch: fn(u16, u16),
    pub verbose: bool,
}

impl WacomHandler {
    pub fn get_instance(verbose: bool, on_touch: fn(u16, u16)) -> WacomHandler {
        return WacomHandler {
            name: "MT".to_owned(),
            on_touch,
            verbose,
            last_x: 0,
            last_y: 0,
        };
    }
}

impl ev::EvdevHandler for WacomHandler {
    fn on_init(&mut self, name: String, _device: &mut Device) {
        println!("INFO: '{0}' input device EPOLL initialized", name);
        self.name = name;
    }

    fn on_event(&mut self, ev: input_event) {
        match ev._type {
            0 => { /* sync */ }
            3 => {
                // Absolute
                match ev.code {
                    0x0 => { // x and y are inverted due to remarkable
                        let val = ev.value as u16;
                        self.last_y = mxc_types::WACOMHEIGHT - val;
                    }
                    0x1 => {
                        self.last_x = ev.value as u16;

                        // callback
                        (self.on_touch)(
                            (self.last_y as f32 * VSCALAR) as u16,
                            (self.last_x as f32 * HSCALAR) as u16,
                        );
                    }
                    _ => {
                        if self.verbose {
                            println!("Unknown event CODE for {0} [type: {1} code: {2} value: {3}]",
                                     self.name, ev._type, ev.code, ev.value);
                        }
                    }
                }
            }
            _ => {
                if self.verbose {
                    println!("Unknown event TYPE for {0} [type: {1} code: {2} value: {3}]",
                             self.name, ev._type, ev.code, ev.value);
                }
            }
        }
    }
}
