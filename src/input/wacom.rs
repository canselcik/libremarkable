use atomic::Atomic;
use evdev::raw::input_event;
use input::{InputDeviceState, InputEvent};
use std;
use std::sync::atomic::{AtomicU16, Ordering};

use framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH, WACOMHEIGHT, WACOMWIDTH};

const WACOM_HSCALAR: f32 = (DISPLAYWIDTH as f32) / (WACOMWIDTH as f32);
const WACOM_VSCALAR: f32 = (DISPLAYHEIGHT as f32) / (WACOMHEIGHT as f32);

const EV_SYNC: u16 = 0;
const EV_KEY: u16 = 1;
const EV_ABS: u16 = 3;
const WACOM_EVCODE_PRESSURE: u16 = 24;
const WACOM_EVCODE_DISTANCE: u16 = 25;
const WACOM_EVCODE_XTILT: u16 = 26;
const WACOM_EVCODE_YTILT: u16 = 27;
const WACOM_EVCODE_XPOS: u16 = 0;
const WACOM_EVCODE_YPOS: u16 = 1;

pub struct WacomState {
    last_x: AtomicU16,
    last_y: AtomicU16,
    last_xtilt: AtomicU16,
    last_ytilt: AtomicU16,
    last_dist: AtomicU16,
    last_pressure: AtomicU16,
    last_tool: Atomic<Option<WacomPen>>,
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
            last_tool: Atomic::new(None),
        }
    }
}

#[repr(u16)]
#[derive(PartialEq, Copy, Clone, Debug)]
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
        EV_SYNC => match state.last_tool.load(Ordering::Relaxed) {
            Some(WacomPen::ToolPen) => Some(InputEvent::WacomEvent {
                event: WacomEvent::Hover {
                    y: (f32::from(state.last_y.load(Ordering::Relaxed)) * WACOM_VSCALAR) as u16,
                    x: (f32::from(state.last_x.load(Ordering::Relaxed)) * WACOM_HSCALAR) as u16,
                    distance: state.last_dist.load(Ordering::Relaxed) as u16,
                    tilt_x: state.last_xtilt.load(Ordering::Relaxed),
                    tilt_y: state.last_ytilt.load(Ordering::Relaxed),
                },
            }),
            Some(WacomPen::Touch) => Some(InputEvent::WacomEvent {
                event: WacomEvent::Draw {
                    x: (f32::from(state.last_x.load(Ordering::Relaxed)) * WACOM_HSCALAR) as u16,
                    y: (f32::from(state.last_y.load(Ordering::Relaxed)) * WACOM_VSCALAR) as u16,
                    pressure: state.last_pressure.load(Ordering::Relaxed),
                    tilt_x: state.last_xtilt.load(Ordering::Relaxed),
                    tilt_y: state.last_ytilt.load(Ordering::Relaxed),
                },
            }),
            _ => None,
        },
        EV_KEY => {
            /* key (device detected - device out of range etc.) */
            if ev.code < WacomPen::ToolPen as u16 || ev.code > WacomPen::Stylus2 as u16 {
                return None;
            }

            let pen: WacomPen = unsafe { std::mem::transmute_copy(&ev.code) };
            state.last_tool.store(Some(pen), Ordering::Relaxed);

            Some(InputEvent::WacomEvent {
                event: WacomEvent::InstrumentChange {
                    pen,
                    state: ev.value != 0,
                },
            })
        }
        EV_ABS => {
            // Absolute
            match ev.code {
                WACOM_EVCODE_DISTANCE => {
                    // distance up to 255
                    // So we have an interesting behavior here.
                    // When the tip is pressed to the point where last_pressure is 4095,
                    // the last_dist supplants to that current max.
                    if state.last_pressure.load(Ordering::Relaxed) == 0 {
                        state.last_dist.store(ev.value as u16, Ordering::Relaxed);
                        state
                            .last_tool
                            .store(Some(WacomPen::ToolPen), Ordering::Relaxed);
                    } else {
                        state
                            .last_pressure
                            .fetch_add(ev.value as u16, Ordering::Relaxed);
                        state
                            .last_tool
                            .store(Some(WacomPen::Touch), Ordering::Relaxed);
                    }
                }
                WACOM_EVCODE_XTILT => {
                    // xtilt -9000 to 9000
                    state.last_xtilt.store(ev.value as u16, Ordering::Relaxed);
                }
                WACOM_EVCODE_YTILT => {
                    // ytilt -9000 to 9000
                    state.last_ytilt.store(ev.value as u16, Ordering::Relaxed);
                }
                WACOM_EVCODE_PRESSURE => {
                    // contact made with pressure val up to 4095
                    state
                        .last_pressure
                        .store(ev.value as u16, Ordering::Relaxed);;
                }
                WACOM_EVCODE_XPOS => {
                    // x and y are inverted due to remarkable
                    let val = ev.value as u16;
                    state.last_y.store(WACOMHEIGHT - val, Ordering::Relaxed);
                }
                WACOM_EVCODE_YPOS => {
                    state.last_x.store(ev.value as u16, Ordering::Relaxed);
                }
                _ => {
                    debug!(
                        "Unknown absolute event code for Wacom [type: {0} code: {1} value: {2}]",
                        ev._type, ev.code, ev.value
                    );
                }
            }
            None
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
