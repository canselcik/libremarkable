use super::ecodes;
use crate::device::rotate::CoordinatePart;
use crate::device::CURRENT_DEVICE;
use crate::dimensions::{DISPLAYHEIGHT, DISPLAYWIDTH, MTHEIGHT, MTWIDTH};
use crate::input::scan::SCANNED;
use crate::input::{Finger, InputDeviceState, InputEvent, MultitouchEvent};
use once_cell::sync::Lazy;

use evdev::InputEvent as EvInputEvent;
use fxhash::FxHashMap;
use log::{debug, warn};
use std::sync::{
	atomic::{AtomicI32, Ordering},
	Mutex,
};

static MT_HSCALAR: Lazy<f32> = Lazy::new(|| (DISPLAYWIDTH as f32) / (*MTWIDTH as f32));
static MT_VSCALAR: Lazy<f32> = Lazy::new(|| (DISPLAYHEIGHT as f32) / (*MTHEIGHT as f32));

pub struct MultitouchState {
    fingers: Mutex<FxHashMap<i32 /* slot */, Finger>>,
    current_slot: AtomicI32,
}

impl ::std::default::Default for MultitouchState {
    fn default() -> Self {
        MultitouchState {
            fingers: Mutex::new(FxHashMap::default()),
            current_slot: AtomicI32::new(0),
        }
    }
}

pub fn decode(ev: &EvInputEvent, outer_state: &InputDeviceState) -> Vec<InputEvent> {
    let state = match outer_state {
        InputDeviceState::MultitouchState(ref state_arc) => state_arc,
        _ => unreachable!(),
    };
    let mut fingers = state.fingers.lock().unwrap();
    let current_slot = state.current_slot.load(Ordering::Relaxed);
    match ev.event_type().0 {
        ecodes::EV_SYN => {
            match ev.code() {
                ecodes::SYN_REPORT => {
                    let mut events: Vec<InputEvent> = vec![];
                    for (_slot, mut finger) in fingers.iter_mut() {
                        if !finger.last_pressed && finger.pressed {
                            // Pressed
                            finger.last_pressed = finger.pressed;

                            events.push(InputEvent::MultitouchEvent {
                                event: MultitouchEvent::Press { finger: *finger },
                            });
                        } else if finger.last_pressed && !finger.pressed {
                            // Released
                            finger.last_pressed = finger.pressed;
                            events.push(InputEvent::MultitouchEvent {
                                event: MultitouchEvent::Release { finger: *finger },
                            });
                        } else if finger.last_pressed && finger.pressed && finger.pos_updated {
                            events.push(InputEvent::MultitouchEvent {
                                event: MultitouchEvent::Move { finger: *finger },
                            });
                        }

                        if finger.pos_updated {
                            finger.pos_updated = false;
                        }
                    }
                    events
                }
                _ => {
                    debug!(
                        "Unsupported event code for syn [type: {0:?} code: {1} value: {2}]",
                        ev.event_type(),
                        ev.code(),
                        ev.value()
                    );
                    vec![]
                }
            }
        }
        ecodes::EV_ABS => {
            // Absolute
            match ev.code() {
                ecodes::ABS_MT_SLOT => {
                    state.current_slot.store(ev.value(), Ordering::Relaxed);
                    // Since only one event is processed, it isn't
                    // necessary to change the local current_slot variable.
                    vec![]
                }
                ecodes::ABS_MT_POSITION_X => {
                    let placement = CURRENT_DEVICE.get_multitouch_placement();
                    let mut rotated_part = placement.rotation.rotate_part(
                        CoordinatePart::X(ev.value() as u16),
                        &SCANNED.multitouch_orig_size,
                    );
                    if placement.invert_x {
                        if let CoordinatePart::X(ref mut x_value) = rotated_part {
                            *x_value = *MTWIDTH - *x_value;
                        }
                    }
                    if placement.invert_y {
                        if let CoordinatePart::Y(ref mut y_value) = rotated_part {
                            *y_value = *MTHEIGHT - *y_value;
                        }
                    }
                    let finger: &mut Finger = fingers.entry(current_slot).or_default();
                    match rotated_part {
                        CoordinatePart::X(rotated_value) => {
                            finger.pos.x = (f32::from(rotated_value) * *MT_HSCALAR) as u16;
                        }
                        CoordinatePart::Y(rotated_value) => {
                            finger.pos.y = (f32::from(rotated_value) * *MT_VSCALAR) as u16;
                        }
                    }
                    finger.pos_updated = true;
                    vec![]
                }
                ecodes::ABS_MT_POSITION_Y => {
                    let placement = CURRENT_DEVICE.get_multitouch_placement();
                    let mut rotated_part = placement.rotation.rotate_part(
                        CoordinatePart::Y(ev.value() as u16),
                        &SCANNED.multitouch_orig_size,
                    );
                    if placement.invert_x {
                        if let CoordinatePart::X(ref mut x_value) = rotated_part {
                            *x_value = *MTWIDTH - *x_value;
                        }
                    }
                    if placement.invert_y {
                        if let CoordinatePart::Y(ref mut y_value) = rotated_part {
                            *y_value = *MTHEIGHT - *y_value;
                        }
                    }
                    let finger: &mut Finger = fingers.entry(current_slot).or_default();
                    match rotated_part {
                        CoordinatePart::X(rotated_value) => {
                            finger.pos.x = (f32::from(rotated_value) * *MT_HSCALAR) as u16;
                        }
                        CoordinatePart::Y(rotated_value) => {
                            finger.pos.y = (f32::from(rotated_value) * *MT_VSCALAR) as u16;
                        }
                    }
                    finger.pos_updated = true;
                    vec![]
                }
                ecodes::ABS_MT_PRESSURE => {
                    if ev.value() > 0 {
                        // Pretty much always true, but who knows
                        fingers.entry(current_slot).or_default().pressed = true;
                    }
                    vec![]
                }
                ecodes::ABS_MT_TRACKING_ID => match ev.value() {
                    -1 => {
                        fingers.entry(current_slot).or_default().pressed = false;
                        vec![]
                    }
                    _ => {
                        fingers.entry(current_slot).or_default().tracking_id = ev.value();
                        vec![]
                    }
                },
                ecodes::ABS_MT_ORIENTATION
                | ecodes::ABS_MT_TOUCH_MAJOR
                | ecodes::ABS_MT_TOUCH_MINOR => vec![], // Currently not needed
                // very unlikely
                // Technically possible (but maybe not for the reMarkable):
                // ABS_MT_DISTANCE, ABS_MT_TOOL_X, ABS_MT_TOOL_Y, ABS_MT_WIDTH_MAJOR,
                // ABS_MT_WIDTH_MINOR, ABS_MT_TOOL_TYPE or ABS_MT_BLOB_ID
                _ => {
                    warn!(
                        "Unknown event code for multitouch [type: {0:?} code: {1} value: {2}]",
                        ev.event_type(),
                        ev.code(),
                        ev.value()
                    );
                    vec![]
                }
            }
        }
        _ => {
            warn!(
                "Unknown event type for [type: {0:?} code: {1} value: {2}]",
                ev.event_type(),
                ev.code(),
                ev.value()
            );
            vec![]
        }
    }
}
