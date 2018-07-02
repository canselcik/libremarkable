use framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH, MTHEIGHT, MTWIDTH};

use evdev::raw::input_event;
use input::{InputDeviceState, InputEvent};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering};

const MT_HSCALAR: f32 = (DISPLAYWIDTH as f32) / (MTWIDTH as f32);
const MT_VSCALAR: f32 = (DISPLAYHEIGHT as f32) / (MTHEIGHT as f32);

pub struct MultitouchState {
    last_pressure: AtomicU8,
    last_touch_size: AtomicU8,
    last_touch_id: AtomicU16,
    last_x: AtomicU16,
    last_y: AtomicU16,
    last_finger_id: AtomicU16,
    currently_touching: AtomicBool,
}

impl ::std::default::Default for MultitouchState {
    fn default() -> Self {
        MultitouchState {
            last_pressure: AtomicU8::new(0),
            last_touch_size: AtomicU8::new(0),
            last_touch_id: AtomicU16::new(0),
            last_x: AtomicU16::new(0),
            last_y: AtomicU16::new(0),
            last_finger_id: AtomicU16::new(0),
            currently_touching: AtomicBool::new(false),
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum MultitouchEvent {
    Touch {
        gesture_seq: u16,
        finger_id: u16,
        y: u16,
        x: u16,
    },
    Unknown,
}

pub fn decode(ev: &input_event, outer_state: &InputDeviceState) -> Option<InputEvent> {
    let state = match outer_state {
        InputDeviceState::MultitouchState(ref state_arc) => state_arc,
        _ => unreachable!(),
    };
    match ev._type {
        0 => {
            /* sync */
            None
        }
        3 => {
            // Absolute
            match ev.code {
                47 => {
                    state
                        .last_finger_id
                        .store(ev.value as u16, Ordering::Relaxed);
                    None
                }
                53 => {
                    let val = ev.value as u16;
                    state.last_x.store(MTWIDTH - val, Ordering::Relaxed);
                    None
                }
                54 => {
                    let val = ev.value as u16;
                    state.last_y.store(MTHEIGHT - val, Ordering::Relaxed);

                    let y = (f32::from(state.last_y.load(Ordering::Relaxed)) * MT_VSCALAR) as u16;
                    let x = (f32::from(state.last_x.load(Ordering::Relaxed)) * MT_HSCALAR) as u16;
                    let event = MultitouchEvent::Touch {
                        gesture_seq: state.last_touch_id.load(Ordering::Relaxed),
                        finger_id: state.last_finger_id.load(Ordering::Relaxed),
                        y,
                        x,
                    };

                    Some(InputEvent::MultitouchEvent { event })
                }
                52 | 48 => {
                    debug!(
                        "unknown_absolute_touch_event(code={0}, value={1})",
                        ev.code, ev.value
                    );
                    None
                }
                58 => {
                    state.last_pressure.store(ev.value as u8, Ordering::Relaxed);
                    None
                }
                49 => {
                    // potentially incorrect
                    state
                        .last_touch_size
                        .store(ev.value as u8, Ordering::Relaxed);
                    None
                }
                57 => match ev.value {
                    -1 => {
                        state.currently_touching.store(false, Ordering::Relaxed);
                        None
                    }
                    touch_id => {
                        state
                            .last_touch_id
                            .store(touch_id as u16, Ordering::Relaxed);
                        state.currently_touching.store(true, Ordering::Relaxed);
                        None
                    }
                },
                // very unlikely
                _ => {
                    warn!(
                        "Unknown event code for multitouch [type: {0} code: {1} value: {2}]",
                        ev._type, ev.code, ev.value
                    );
                    None
                }
            }
        }
        _ => {
            warn!(
                "Unknown event type for [type: {0} code: {1} value: {2}]",
                ev._type, ev.code, ev.value
            );
            None
        }
    }
}
