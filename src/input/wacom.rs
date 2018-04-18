use std;
use std::sync::atomic::{AtomicU16, Ordering};

use input::UnifiedInputHandler;
use input::InputEvent;

use evdev::raw::input_event;

use framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH, WACOMHEIGHT, WACOMWIDTH};

const WACOM_HSCALAR: f32 = (DISPLAYWIDTH as f32) / (WACOMWIDTH as f32);
const WACOM_VSCALAR: f32 = (DISPLAYHEIGHT as f32) / (WACOMHEIGHT as f32);

pub struct WacomState {
    last_x: AtomicU16,
    last_y: AtomicU16,
    last_xtilt: AtomicU16,
    last_ytilt: AtomicU16,
    last_dist: AtomicU16,
    last_pressure: AtomicU16,
}

impl WacomState {
    pub fn new() -> WacomState {
        WacomState {
            last_x: AtomicU16::new(0),
            last_y: AtomicU16::new(0),
            last_xtilt: AtomicU16::new(0),
            last_ytilt: AtomicU16::new(0),
            last_dist: AtomicU16::new(0),
            last_pressure: AtomicU16::new(0),
        }
    }
}

#[repr(u16)]
#[derive(PartialEq, Copy, Clone)]
pub enum WacomPen {
    ToolPen = 320,
    ToolRubber = 321,
    Touch = 330,
    Stylus = 331,
    Stylus2 = 332,
}

#[derive(PartialEq, Copy, Clone)]
pub enum WacomEvent {
    InstrumentChange {
        pen: WacomPen,
        state: bool,
    },
    Hover {
        y: u16,
        x: u16,
        distance: u16,
        tilt_x: u16,
        tilt_y: u16,
    },
    Draw {
        y: u16,
        x: u16,
        pressure: u16,
        tilt_x: u16,
        tilt_y: u16,
    },
    Unknown,
}

impl UnifiedInputHandler {
    pub fn wacom_handler(&mut self, ev: &input_event) {
        match ev._type {
            0 => { /* sync */ }
            1 => {
                /* key (device detected - device out of range etc.) */
                if ev.code >= WacomPen::ToolPen as u16 && ev.code <= WacomPen::Stylus2 as u16 {
                    let event = WacomEvent::InstrumentChange {
                        pen: unsafe { std::mem::transmute_copy(&ev.code) },
                        state: ev.value != 0,
                    };
                    self.tx.send(InputEvent::WacomEvent { event }).unwrap();
                } else {
                    error!(
                        "Unknown key event code for Wacom [type: {0} code: {1} value: {2}]",
                        ev._type, ev.code, ev.value
                    );
                }
            }
            3 => {
                // Absolute
                match ev.code {
                    25 => {
                        // distance up to 255
                        // So we have an interesting behavior here.
                        // When the tip is pressed to the point where last_pressure is 4095,
                        // the last_dist supplants to that current max.
                        self.wacom
                            .last_dist
                            .store(ev.value as u16, Ordering::Relaxed);
                        let last_pressure = self.wacom.last_pressure.load(Ordering::Relaxed);
                        let event = if last_pressure == 0 {
                            WacomEvent::Hover {
                                y: (self.wacom.last_y.load(Ordering::Relaxed) as f32
                                    * WACOM_VSCALAR) as u16,
                                x: (self.wacom.last_x.load(Ordering::Relaxed) as f32
                                    * WACOM_HSCALAR) as u16,
                                distance: ev.value as u16,
                                tilt_x: self.wacom.last_xtilt.load(Ordering::Relaxed),
                                tilt_y: self.wacom.last_ytilt.load(Ordering::Relaxed),
                            }
                        } else {
                            WacomEvent::Draw {
                                x: (self.wacom.last_x.load(Ordering::Relaxed) as f32
                                    * WACOM_HSCALAR) as u16,
                                y: (self.wacom.last_y.load(Ordering::Relaxed) as f32
                                    * WACOM_VSCALAR) as u16,
                                pressure: last_pressure + (ev.value as u16),
                                tilt_x: self.wacom.last_xtilt.load(Ordering::Relaxed),
                                tilt_y: self.wacom.last_ytilt.load(Ordering::Relaxed),
                            }
                        };
                        self.tx.send(InputEvent::WacomEvent { event }).unwrap();
                    }
                    26 => {
                        // xtilt -9000 to 9000
                        self.wacom
                            .last_xtilt
                            .store(ev.value as u16, Ordering::Relaxed);
                    }
                    27 => {
                        // ytilt -9000 to 9000
                        self.wacom
                            .last_ytilt
                            .store(ev.value as u16, Ordering::Relaxed);
                    }
                    24 => {
                        // contact made with pressure val up to 4095
                        self.wacom
                            .last_pressure
                            .store(ev.value as u16, Ordering::Relaxed);
                        let event = WacomEvent::Draw {
                            x: (self.wacom.last_x.load(Ordering::Relaxed) as f32 * WACOM_HSCALAR)
                                as u16,
                            y: (self.wacom.last_y.load(Ordering::Relaxed) as f32 * WACOM_VSCALAR)
                                as u16,
                            pressure: self.wacom.last_pressure.load(Ordering::Relaxed),
                            tilt_x: self.wacom.last_xtilt.load(Ordering::Relaxed),
                            tilt_y: self.wacom.last_ytilt.load(Ordering::Relaxed),
                        };
                        self.tx.send(InputEvent::WacomEvent { event }).unwrap();
                    }
                    0x0 => {
                        // x and y are inverted due to remarkable
                        let val = ev.value as u16;
                        self.wacom
                            .last_y
                            .store(WACOMHEIGHT - val, Ordering::Relaxed);
                    }
                    0x1 => {
                        self.wacom.last_x.store(ev.value as u16, Ordering::Relaxed);
                    }
                    _ => {
                        debug!("Unknown absolute event code for Wacom [type: {0} code: {1} value: {2}]",
                               ev._type, ev.code, ev.value);
                    }
                }
            }
            _ => {
                debug!(
                    "Unknown event TYPE for Wacom [type: {0} code: {1} value: {2}]",
                    ev._type, ev.code, ev.value
                );
            }
        }
    }
}
