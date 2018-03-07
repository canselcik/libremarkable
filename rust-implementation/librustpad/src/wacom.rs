use evdev::Device;
use evdev::raw::input_event;

use ev;
use mxc_types;
use std;

const HSCALAR: f32 = (mxc_types::DISPLAYWIDTH as f32) / (mxc_types::WACOMWIDTH as f32);
const VSCALAR: f32 = (mxc_types::DISPLAYHEIGHT as f32) / (mxc_types::WACOMHEIGHT as f32);

/* Very basic handler (does hover tracking right now -- more soon to come) for Wacom digitizer on Remarkable Paper Tablet */

pub struct WacomHandler {
    pub name: String,
    pub last_x: u16,
    pub last_y: u16,
    pub last_xtilt: u16,
    pub last_ytilt: u16,
    pub last_dist: u16,
    pub last_pressure: u16,
    pub callback: fn(WacomEvent),
    pub verbose: bool,
}

impl WacomHandler {
    pub fn get_instance(verbose: bool, on_event: fn(WacomEvent)) -> WacomHandler {
        return WacomHandler {
            name: "MT".to_owned(),
            callback: on_event,
            verbose,
            last_pressure: 0,
            last_x: 0,
            last_y: 0,
            last_xtilt: 0,
            last_ytilt: 0,
            last_dist: 0,
        };
    }
}


#[repr(u16)]
#[derive(PartialEq)]
pub enum WacomPen {
    ToolPen = 320,
    ToolRubber = 321,
    Touch = 330,
    Stylus = 331,
    Stylus2 = 332,
}

pub enum WacomEvent {
    InstrumentChange { pen: WacomPen, state: bool },
    Hover { y: u16, x: u16, distance: u16, tilt_x: u16, tilt_y: u16 },
    Draw  { y: u16, x: u16, pressure: u16, tilt_x: u16, tilt_y: u16 },
}

impl ev::EvdevHandler for WacomHandler {
    fn on_init(&mut self, name: String, _device: &mut Device) {
        println!("INFO: '{0}' input device EPOLL initialized", name);
        self.name = name;
    }

    fn on_event(&mut self, ev: input_event) {
        match ev._type {
            0 => { /* sync */ }
            1 => { /* key (device detected - device out of range etc.) */
                if ev.code >= WacomPen::ToolPen as u16 && ev.code <= WacomPen::Stylus2 as u16 {
                    (self.callback)(WacomEvent::InstrumentChange {
                        pen: unsafe { std::mem::transmute_copy(&ev.code) },
                        state: ev.value != 0,
                    });
                }
                else {
                    println!("Unknown key event code for {0} [type: {1} code: {2} value: {3}]",
                             self.name, ev._type, ev.code, ev.value);
                }
            },
            3 => {
                // Absolute
                match ev.code {
                    25 => { // distance up to 255
                        self.last_dist = ev.value as u16;
                        (self.callback)(WacomEvent::Hover {
                            y: (self.last_y as f32 * VSCALAR) as u16,
                            x: (self.last_x as f32 * HSCALAR) as u16,
                            distance: self.last_dist,
                            tilt_x: self.last_xtilt,
                            tilt_y: self.last_ytilt,
                        });
                    },
                    26 => { // xtilt -9000 to 9000
                        self.last_xtilt = ev.value as u16;
                    },
                    27 => { // ytilt -9000 to 9000
                        self.last_ytilt = ev.value as u16;
                    }
                    24 => { // contact made with pressure val up to 4095
                        self.last_pressure = ev.value as u16;
                        (self.callback)(WacomEvent::Draw {
                            y: (self.last_y as f32 * VSCALAR) as u16,
                            x: (self.last_x as f32 * HSCALAR) as u16,
                            pressure: self.last_pressure,
                            tilt_x: self.last_xtilt,
                            tilt_y: self.last_ytilt,
                        });
                    }
                    0x0 => { // x and y are inverted due to remarkable
                        let val = ev.value as u16;
                        self.last_y = mxc_types::WACOMHEIGHT - val;
                    }
                    0x1 => {
                        self.last_x = ev.value as u16;
                    }
                    _ => {
                        if self.verbose {
                            println!("Unknown absolute event code for {0} [type: {1} code: {2} value: {3}]",
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
