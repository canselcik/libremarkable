use evdev::Device;
use evdev::raw::input_event;

use ev;
use mxc_types;

/* Very simple and rather sufficient handler for multitouch screen on Remarkable Paper Tablet */

pub struct MultitouchHandler {
    pub name: String,
    pub last_touch_id: u16,
    pub last_touch_size: u8,
    pub currently_touching: bool,
    pub last_x: u16,
    pub last_y: u16,
    pub on_touch: fn(u16, u16, u16, u16),
    pub last_finger_id: u16,
}

impl MultitouchHandler {
    pub fn get_instance(on_touch: fn(u16, u16, u16, u16)) -> MultitouchHandler {
        return MultitouchHandler {
            name: "MT".to_owned(),
            on_touch: on_touch,
            currently_touching: false,
            last_finger_id: 0,
            last_touch_id: 0,
            last_touch_size: 0,
            last_x: 0,
            last_y: 0,
        };
    }
}


const HSCALAR: f32 = (mxc_types::DISPLAYWIDTH as f32) / (mxc_types::MTWIDTH as f32);
const VSCALAR: f32 = (mxc_types::DISPLAYHEIGHT as f32) / (mxc_types::MTHEIGHT as f32);

impl ev::EvdevHandler for MultitouchHandler {
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
                    47 => {
                        self.last_finger_id = ev.value as u16;
                    }
                    53 => {
                        let val = ev.value as u16;
                        self.last_x = mxc_types::MTWIDTH - val;
                    }
                    54 => {
                        let val = ev.value as u16;
                        self.last_y = mxc_types::MTHEIGHT - val;

                        // callback
                        (self.on_touch)(
                            self.last_touch_id,
                            self.last_finger_id,
                            (self.last_y as f32 * VSCALAR) as u16,
                            (self.last_x as f32 * HSCALAR) as u16,
                        );
                    }
                    52 | 48 | 58 => {
                        // println!("unknown_absolute_touch_event(code={0}, value={1})", ev.code, ev.value);
                    }
                    49 => {
                        // potentially incorrect
                        self.last_touch_size = ev.value as u8;
                    }
                    57 => {
                        match ev.value {
                            -1 => {
                                self.currently_touching = false;
                            }
                            touch_id => {
                                self.last_touch_id = touch_id as u16;
                                self.currently_touching = true;
                            }
                        }
                    }
                    // very unlikely
                    _ => {
                        println!(
                            "Unknown event code for {0} [type: {1} code: {2} value: {3}]",
                            self.name,
                            ev._type,
                            ev.code,
                            ev.value
                        )
                    } 
                }
            }
            _ => {
                // very unlikely
                println!(
                    "Unknown event type for {0} [type: {1} code: {2} value: {3}]",
                    self.name,
                    ev._type,
                    ev.code,
                    ev.value
                );
            }
        }
    }
}
