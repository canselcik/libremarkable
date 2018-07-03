use atomic::Atomic;
use evdev::raw::input_event;
use input::{InputDeviceState, InputEvent};
use std;
use std::sync::atomic::{AtomicU16, Ordering};

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
    last_event_type: Atomic<WacomEventType>,
}

impl ::std::default::Default for WacomState {
    fn default() -> Self {
        WacomState {
            last_x: AtomicU16::new(0),
            last_y: AtomicU16::new(0),
            last_xtilt: AtomicU16::new(0),
            last_ytilt: AtomicU16::new(0),
            last_dist: AtomicU16::new(0),
            last_pressure: AtomicU16::new(0),
            last_event_type: Atomic::new(WacomEventType::Unknown),
        }
    }
}

#[repr(u16)]
#[derive(PartialEq, Copy, Clone)]
pub enum WacomPen {
    /// This includes the pen hovering
    ToolPen = 320,
    ToolRubber = 321,
    /// This is the pen making contact with the display
    Touch = 330,
    Stylus = 331,
    Stylus2 = 332,
}

#[derive(PartialEq, Copy, Clone)]
pub enum WacomEventType {
    InstrumentChange,
    Hover,
    Draw,
    Unknown,
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

pub fn decode(ev: &input_event, outer_state: &InputDeviceState) -> Option<InputEvent> {
    let state = match outer_state {
        InputDeviceState::WacomState(ref state_arc) => state_arc,
        _ => unreachable!(),
    };
    match ev._type {
        0 => {
            // At sync, we will be re-emitting type of event we emitted the last, however with
            // fresh values populated form the current WacomState.
            match state.last_event_type.load(Ordering::Relaxed) {
                WacomEventType::Draw => Some(InputEvent::WacomEvent {
                    event: WacomEvent::Draw {
                        x: (f32::from(state.last_x.load(Ordering::Relaxed)) * WACOM_HSCALAR) as u16,
                        y: (f32::from(state.last_y.load(Ordering::Relaxed)) * WACOM_VSCALAR) as u16,
                        pressure: state.last_pressure.load(Ordering::Relaxed),
                        tilt_x: state.last_xtilt.load(Ordering::Relaxed),
                        tilt_y: state.last_ytilt.load(Ordering::Relaxed),
                    },
                }),
                WacomEventType::Hover => Some(InputEvent::WacomEvent {
                    event: WacomEvent::Hover {
                        y: (f32::from(state.last_y.load(Ordering::Relaxed)) * WACOM_VSCALAR) as u16,
                        x: (f32::from(state.last_x.load(Ordering::Relaxed)) * WACOM_HSCALAR) as u16,
                        distance: state.last_dist.load(Ordering::Relaxed) as u16,
                        tilt_x: state.last_xtilt.load(Ordering::Relaxed),
                        tilt_y: state.last_ytilt.load(Ordering::Relaxed),
                    },
                }),
                _ => None,
            }
        }
        1 => {
            /* key (device detected - device out of range etc.) */
            if ev.code >= WacomPen::ToolPen as u16 && ev.code <= WacomPen::Stylus2 as u16 {
                let event = WacomEvent::InstrumentChange {
                    pen: unsafe { std::mem::transmute_copy(&ev.code) },
                    state: ev.value != 0,
                };
                state
                    .last_event_type
                    .store(WacomEventType::InstrumentChange, Ordering::Relaxed);
                Some(InputEvent::WacomEvent { event })
            } else {
                error!(
                    "Unknown key event code for Wacom [type: {0} code: {1} value: {2}]",
                    ev._type, ev.code, ev.value
                );
                None
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

                    let mut last_pressure = state.last_pressure.load(Ordering::Relaxed);
                    let event = if last_pressure == 0 {
                        state.last_dist.store(ev.value as u16, Ordering::Relaxed);
                        state
                            .last_event_type
                            .store(WacomEventType::Hover, Ordering::Relaxed);
                        WacomEvent::Hover {
                            y: (f32::from(state.last_y.load(Ordering::Relaxed)) * WACOM_VSCALAR)
                                as u16,
                            x: (f32::from(state.last_x.load(Ordering::Relaxed)) * WACOM_HSCALAR)
                                as u16,
                            distance: ev.value as u16,
                            tilt_x: state.last_xtilt.load(Ordering::Relaxed),
                            tilt_y: state.last_ytilt.load(Ordering::Relaxed),
                        }
                    } else {
                        last_pressure += ev.value as u16;
                        state.last_pressure.store(last_pressure, Ordering::Relaxed);
                        state
                            .last_event_type
                            .store(WacomEventType::Draw, Ordering::Relaxed);
                        WacomEvent::Draw {
                            x: (f32::from(state.last_x.load(Ordering::Relaxed)) * WACOM_HSCALAR)
                                as u16,
                            y: (f32::from(state.last_y.load(Ordering::Relaxed)) * WACOM_VSCALAR)
                                as u16,
                            pressure: last_pressure,
                            tilt_x: state.last_xtilt.load(Ordering::Relaxed),
                            tilt_y: state.last_ytilt.load(Ordering::Relaxed),
                        }
                    };
                    Some(InputEvent::WacomEvent { event })
                }
                26 => {
                    // xtilt -9000 to 9000
                    state.last_xtilt.store(ev.value as u16, Ordering::Relaxed);
                    None
                }
                27 => {
                    // ytilt -9000 to 9000
                    state.last_ytilt.store(ev.value as u16, Ordering::Relaxed);
                    None
                }
                24 => {
                    // contact made with pressure val up to 4095
                    state
                        .last_pressure
                        .store(ev.value as u16, Ordering::Relaxed);
                    state
                        .last_event_type
                        .store(WacomEventType::Draw, Ordering::Relaxed);
                    let event = WacomEvent::Draw {
                        x: (f32::from(state.last_x.load(Ordering::Relaxed)) * WACOM_HSCALAR) as u16,
                        y: (f32::from(state.last_y.load(Ordering::Relaxed)) * WACOM_VSCALAR) as u16,
                        pressure: state.last_pressure.load(Ordering::Relaxed),
                        tilt_x: state.last_xtilt.load(Ordering::Relaxed),
                        tilt_y: state.last_ytilt.load(Ordering::Relaxed),
                    };
                    Some(InputEvent::WacomEvent { event })
                }
                0x0 => {
                    // x and y are inverted due to remarkable
                    let val = ev.value as u16;
                    state.last_y.store(WACOMHEIGHT - val, Ordering::Relaxed);
                    None
                }
                0x1 => {
                    state.last_x.store(ev.value as u16, Ordering::Relaxed);
                    None
                }
                _ => {
                    debug!(
                        "Unknown absolute event code for Wacom [type: {0} code: {1} value: {2}]",
                        ev._type, ev.code, ev.value
                    );
                    None
                }
            }
        }
        _ => {
            debug!(
                "Unknown event TYPE for Wacom [type: {0} code: {1} value: {2}]",
                ev._type, ev.code, ev.value
            );
            None
        }
    }
}
