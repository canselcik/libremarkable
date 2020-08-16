use framebuffer::cgmath;
use framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH, MTHEIGHT, MTWIDTH};

use evdev::raw::input_event;
use input::{InputDeviceState, InputEvent};
use super::ecodes;
use std::sync::{Mutex, atomic::{AtomicI32, Ordering}};
use std::convert::TryInto;
use fxhash::FxHashMap;

const MT_HSCALAR: f32 = (DISPLAYWIDTH as f32) / (MTWIDTH as f32);
const MT_VSCALAR: f32 = (DISPLAYHEIGHT as f32) / (MTHEIGHT as f32);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Finger {
    pub tracking_id: i32,
    
    pub pos: cgmath::Point2<u16>,
    pos_updated: bool, // Report motion at SYN_REPORT?
    
    last_pressed: bool,
    pub pressed: bool,
}
impl Default for Finger {
    fn default() -> Finger {
        Finger {
            tracking_id: -1, // -1 should never be seen by a InputEvent receiver
            pos: cgmath::Point2 { x: u16::max_value(), y: u16::max_value() },
            pos_updated: false,
            last_pressed: false,
            pressed: false,
        }
    }
}

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

#[derive(PartialEq, Copy, Clone)]
pub enum MultitouchEvent {
    Press { finger: Finger, slot: u8 },
    Release { finger: Finger, slot: u8 },
    Move { finger: Finger, slot: u8 },
    Unknown,
}

pub fn decode(ev: &input_event, outer_state: &InputDeviceState) -> Vec<InputEvent> {
    let state = match outer_state {
        InputDeviceState::MultitouchState(ref state_arc) => state_arc,
        _ => unreachable!(),
    };
    let mut fingers = state.fingers.lock().unwrap();
    let current_slot = state.current_slot.load(Ordering::Relaxed);
    match ev._type {
        ecodes::EV_SYN => {
            match ev.code {
                ecodes::SYN_REPORT => {
                    let mut events: Vec<InputEvent> = vec![];
                    for (slot, mut finger) in fingers.iter_mut() {
                        if ! finger.last_pressed && finger.pressed {
                            // Pressed
                            finger.last_pressed = finger.pressed;

                            events.push(InputEvent::MultitouchEvent {
                                event: MultitouchEvent::Press {
                                    finger: finger.clone(),
                                    slot: (*slot).try_into().unwrap(),
                                }
                            });
                        } else if finger.last_pressed && ! finger.pressed {
                            // Released
                            finger.last_pressed = finger.pressed;
                            events.push(InputEvent::MultitouchEvent {
                                event: MultitouchEvent::Release {
                                    finger: finger.clone(),
                                    slot: (*slot).try_into().unwrap(),
                                }
                            });
                        }else if finger.last_pressed && finger.pressed && finger.pos_updated {
                            events.push(InputEvent::MultitouchEvent {
                                event: MultitouchEvent::Move {
                                    finger: finger.clone(),
                                    slot: (*slot).try_into().unwrap(),
                                }
                            });
                        }

                        if finger.pos_updated {
                            finger.pos_updated = false;
                        }
                    }
                    events
                },
                _ => {
                    debug!(
                        "Unsupported event code for syn [type: {0} code: {1} value: {2}]",
                        ev._type, ev.code, ev.value
                    );
                    vec![]
                }
            }
            
        }
        ecodes::EV_ABS => {
            // Absolute
            match ev.code {
                ecodes::ABS_MT_SLOT => {
                    state.current_slot.store(ev.value, Ordering::Relaxed);
                    // Since only one event is processed, it isn't
                    // necessary to change the local current_slot variable.
                    vec![]
                }
                ecodes::ABS_MT_POSITION_X => {
                    let scaled_val = (f32::from(MTWIDTH - ev.value as u16) * MT_HSCALAR) as u16;
                    fingers.entry(current_slot).or_default().pos.x = scaled_val;
                    fingers.entry(current_slot).or_default().pos_updated = true;
                    vec![]
                }
                ecodes::ABS_MT_POSITION_Y => {
                    let scaled_val = (f32::from(MTHEIGHT - ev.value as u16) * MT_VSCALAR) as u16;
                    fingers.entry(current_slot).or_default().pos.y = scaled_val;
                    fingers.entry(current_slot).or_default().pos_updated = true;
                    vec![]
                },
                ecodes::ABS_MT_PRESSURE => {
                    if ev.value > 0 { // Pretty much always true, but who knows
                        fingers.entry(current_slot).or_default().pressed = true;
                    }
                    vec![]
                },
                ecodes::ABS_MT_TRACKING_ID => match ev.value {
                    -1 => {
                        fingers.entry(current_slot).or_default().pressed = false;
                        vec![]
                    },
                    _ => {
                        fingers.entry(current_slot).or_default().tracking_id = ev.value;
                        vec![]
                    }
                },
                ecodes::ABS_MT_ORIENTATION |
                ecodes::ABS_MT_TOUCH_MAJOR |
                ecodes::ABS_MT_TOUCH_MINOR => vec![], // Currently not needed
                // very unlikely
                // Technically possible (but maybe not for the reMarkable):
                // ABS_MT_DISTANCE, ABS_MT_TOOL_X, ABS_MT_TOOL_Y, ABS_MT_WIDTH_MAJOR,
                // ABS_MT_WIDTH_MINOR, ABS_MT_TOOL_TYPE or ABS_MT_BLOB_ID
                _ => {
                    warn!(
                        "Unknown event code for multitouch [type: {0} code: {1} value: {2}]",
                        ev._type, ev.code, ev.value
                    );
                    vec![]
                }
            }
        }
        _ => {
            warn!(
                "Unknown event type for [type: {0} code: {1} value: {2}]",
                ev._type, ev.code, ev.value
            );
            vec![]
        }
    }
}
