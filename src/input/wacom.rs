use super::ecodes;
use crate::device::CURRENT_DEVICE;
use crate::input::rotate::CoordinatePart;
use crate::input::scan::SCANNED;
use crate::input::{InputDeviceState, InputEvent};
use atomic::Atomic;
use evdev::raw::input_event;
use log::debug;
use std::sync::atomic::{AtomicU16, Ordering};

use crate::framebuffer::cgmath;
use crate::framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH, WACOMHEIGHT, WACOMWIDTH};

lazy_static! {
    static ref WACOM_HSCALAR: f32 = (DISPLAYWIDTH as f32) / (*WACOMWIDTH as f32);
    static ref WACOM_VSCALAR: f32 = (DISPLAYHEIGHT as f32) / (*WACOMHEIGHT as f32);
}

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
    /// When the pen gets into the reach of the digitizer
    /// a tool will be selected. This is useful for software
    /// to know whether the user is hovering the backside (rubber)
    /// or frontside (pen) of a stylus above the screen.
    /// Both at once shouldn't be possible.
    ToolPen = ecodes::BTN_TOOL_PEN,
    ToolRubber = ecodes::BTN_TOOL_RUBBER,
    /// This is the pen making contact with the display
    Touch = ecodes::BTN_TOUCH,
    Stylus = ecodes::BTN_STYLUS,
    Stylus2 = ecodes::BTN_STYLUS2,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum WacomEventType {
    InstrumentChange,
    Hover,
    Draw,
    Unknown,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum WacomEvent {
    InstrumentChange {
        pen: WacomPen,
        state: bool,
    },
    Hover {
        position: cgmath::Point2<f32>,
        distance: u16,
        tilt: cgmath::Vector2<u16>,
    },
    Draw {
        position: cgmath::Point2<f32>,
        pressure: u16,
        tilt: cgmath::Vector2<u16>,
    },
    Unknown,
}

pub fn decode(ev: &input_event, outer_state: &InputDeviceState) -> Option<InputEvent> {
    let state = match outer_state {
        InputDeviceState::WacomState(ref state_arc) => state_arc,
        _ => unreachable!(),
    };
    match ev._type {
        ecodes::EV_SYN => match state.last_tool.load(Ordering::Relaxed) {
            Some(WacomPen::ToolPen) => Some(InputEvent::WacomEvent {
                event: WacomEvent::Hover {
                    position: cgmath::Point2 {
                        x: (f32::from(state.last_x.load(Ordering::Relaxed)) * *WACOM_HSCALAR),
                        y: (f32::from(state.last_y.load(Ordering::Relaxed)) * *WACOM_VSCALAR),
                    },
                    distance: state.last_dist.load(Ordering::Relaxed) as u16,
                    tilt: cgmath::Vector2 {
                        x: state.last_xtilt.load(Ordering::Relaxed),
                        y: state.last_ytilt.load(Ordering::Relaxed),
                    },
                },
            }),
            Some(WacomPen::Touch) => Some(InputEvent::WacomEvent {
                event: WacomEvent::Draw {
                    position: cgmath::Point2 {
                        x: (f32::from(state.last_x.load(Ordering::Relaxed)) * *WACOM_HSCALAR),
                        y: (f32::from(state.last_y.load(Ordering::Relaxed)) * *WACOM_VSCALAR),
                    },
                    pressure: state.last_pressure.load(Ordering::Relaxed),
                    tilt: cgmath::Vector2 {
                        x: state.last_xtilt.load(Ordering::Relaxed),
                        y: state.last_ytilt.load(Ordering::Relaxed),
                    },
                },
            }),
            _ => None,
        },
        ecodes::EV_KEY => {
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
        ecodes::EV_ABS => {
            // Absolute
            match ev.code {
                ecodes::ABS_DISTANCE => {
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
                ecodes::ABS_TILT_X => {
                    // xtilt -9000 to 9000
                    state.last_xtilt.store(ev.value as u16, Ordering::Relaxed);
                }
                ecodes::ABS_TILT_Y => {
                    // ytilt -9000 to 9000
                    state.last_ytilt.store(ev.value as u16, Ordering::Relaxed);
                }
                ecodes::ABS_PRESSURE => {
                    // contact made with pressure val up to 4095
                    state
                        .last_pressure
                        .store(ev.value as u16, Ordering::Relaxed);
                }
                ecodes::ABS_X => {
                    let placement = CURRENT_DEVICE.get_wacom_placement();
                    let mut rotated_part = placement
                        .rotation
                        .rotate_part(CoordinatePart::X(ev.value as u16), &SCANNED.wacom_orig_size);
                    if placement.invert_x {
                        if let CoordinatePart::X(ref mut x_value) = rotated_part {
                            *x_value = *WACOMWIDTH - *x_value;
                        }
                    }
                    if placement.invert_y {
                        if let CoordinatePart::Y(ref mut y_value) = rotated_part {
                            *y_value = *WACOMHEIGHT - *y_value;
                        }
                    }
                    match rotated_part {
                        CoordinatePart::X(rotated_value) => {
                            state.last_x.store(rotated_value, Ordering::Relaxed);
                        }
                        CoordinatePart::Y(rotated_value) => {
                            state.last_y.store(rotated_value, Ordering::Relaxed);
                        }
                    }
                }
                ecodes::ABS_Y => {
                    let placement = CURRENT_DEVICE.get_wacom_placement();
                    let mut rotated_part = placement
                        .rotation
                        .rotate_part(CoordinatePart::Y(ev.value as u16), &SCANNED.wacom_orig_size);
                    if placement.invert_x {
                        if let CoordinatePart::X(ref mut x_value) = rotated_part {
                            *x_value = *WACOMWIDTH - *x_value;
                        }
                    }
                    if placement.invert_y {
                        if let CoordinatePart::Y(ref mut y_value) = rotated_part {
                            *y_value = *WACOMHEIGHT - *y_value;
                        }
                    }
                    match rotated_part {
                        CoordinatePart::X(rotated_value) => {
                            state.last_x.store(rotated_value, Ordering::Relaxed);
                        }
                        CoordinatePart::Y(rotated_value) => {
                            state.last_y.store(rotated_value, Ordering::Relaxed);
                        }
                    }
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
