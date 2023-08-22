use super::ecodes;
use crate::device::rotate::CoordinatePart;
use crate::device::CURRENT_DEVICE;
use crate::input::scan::SCANNED;
use crate::input::{InputDeviceState, InputEvent, WacomEvent, WacomPen};
use atomic::Atomic;
use evdev::InputEvent as EvInputEvent;
use log::debug;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU16, Ordering};

use crate::cgmath;
use crate::dimensions::{DISPLAYHEIGHT, DISPLAYWIDTH, WACOMHEIGHT, WACOMWIDTH};

static WACOM_HSCALAR: Lazy<f32> = Lazy::new(|| f32::from(DISPLAYWIDTH) / f32::from(*WACOMWIDTH));
static WACOM_VSCALAR: Lazy<f32> = Lazy::new(|| f32::from(DISPLAYHEIGHT) / f32::from(*WACOMHEIGHT));

pub struct WacomState {
    last_x: AtomicU16,
    last_y: AtomicU16,
    last_xtilt: AtomicU16,
    last_ytilt: AtomicU16,
    last_dist: AtomicU16,
    last_pressure: AtomicU16,
    last_touch_state: Atomic<bool>,
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
            last_touch_state: Atomic::new(false),
        }
    }
}

pub fn decode(ev: &EvInputEvent, outer_state: &InputDeviceState) -> Option<InputEvent> {
    let state = match outer_state {
        InputDeviceState::WacomState(ref state_arc) => state_arc,
        _ => unreachable!(),
    };
    match ev.event_type().0 {
        ecodes::EV_SYN => match state.last_touch_state.load(Ordering::Relaxed) {
            false => Some(InputEvent::WacomEvent {
                event: WacomEvent::Hover {
                    position: cgmath::Point2 {
                        x: (f32::from(state.last_x.load(Ordering::Relaxed)) * *WACOM_HSCALAR),
                        y: (f32::from(state.last_y.load(Ordering::Relaxed)) * *WACOM_VSCALAR),
                    },
                    distance: state.last_dist.load(Ordering::Relaxed),
                    tilt: cgmath::Vector2 {
                        x: state.last_xtilt.load(Ordering::Relaxed),
                        y: state.last_ytilt.load(Ordering::Relaxed),
                    },
                },
            }),
            true => Some(InputEvent::WacomEvent {
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
        },
        ecodes::EV_KEY => {
            /* key (device detected - device out of range etc.) */
            if ev.code() < WacomPen::ToolPen as u16 || ev.code() > WacomPen::Stylus2 as u16 {
                return None;
            }

            let pen: WacomPen = unsafe { std::mem::transmute_copy(&ev.code()) };
            let pen_state = ev.value() != 0;

            if pen == WacomPen::Touch {
                state.last_touch_state.store(pen_state, Ordering::Relaxed);
            }

            Some(InputEvent::WacomEvent {
                event: WacomEvent::InstrumentChange {
                    pen,
                    state: pen_state,
                },
            })
        }
        ecodes::EV_ABS => {
            // Absolute
            match ev.code() {
                ecodes::ABS_DISTANCE => {
                    // distance up to 255
                    // So we have an interesting behavior here.
                    // When the tip is pressed to the point where last_pressure is 4095,
                    // the last_dist supplants to that current max.
                    if state.last_pressure.load(Ordering::Relaxed) == 0 {
                        state.last_dist.store(ev.value() as u16, Ordering::Relaxed);
                        state.last_touch_state.store(false, Ordering::Relaxed);
                    } else {
                        state
                            .last_pressure
                            .fetch_add(ev.value() as u16, Ordering::Relaxed);
                        state.last_touch_state.store(true, Ordering::Relaxed);
                    }
                }
                ecodes::ABS_TILT_X => {
                    // xtilt -9000 to 9000
                    state.last_xtilt.store(ev.value() as u16, Ordering::Relaxed);
                }
                ecodes::ABS_TILT_Y => {
                    // ytilt -9000 to 9000
                    state.last_ytilt.store(ev.value() as u16, Ordering::Relaxed);
                }
                ecodes::ABS_PRESSURE => {
                    // contact made with pressure val up to 4095
                    state
                        .last_pressure
                        .store(ev.value() as u16, Ordering::Relaxed);
                }
                ecodes::ABS_X => {
                    let placement = CURRENT_DEVICE.get_wacom_placement();
                    let mut rotated_part = placement.rotation.rotate_part(
                        CoordinatePart::X(ev.value() as u16),
                        &SCANNED.wacom_orig_size,
                    );
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
                    let mut rotated_part = placement.rotation.rotate_part(
                        CoordinatePart::Y(ev.value() as u16),
                        &SCANNED.wacom_orig_size,
                    );
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
                        "Unknown absolute event code for Wacom [type: {0:?} code: {1} value: {2}]",
                        ev.event_type(),
                        ev.code(),
                        ev.value()
                    );
                }
            }
            None
        }
        _ => {
            debug!(
                "Unknown event TYPE for Wacom [type: {0:?} code: {1} value: {2}]",
                ev.event_type(),
                ev.code(),
                ev.value()
            );
            None
        }
    }
}
