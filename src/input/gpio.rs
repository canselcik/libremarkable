use super::ecodes;
use crate::input::{InputDeviceState, InputEvent};
use evdev::raw::input_event;
use log::error;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum PhysicalButton {
    LEFT,
    MIDDLE,
    RIGHT,
    POWER,
    WAKEUP,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum GPIOEvent {
    Press { button: PhysicalButton },
    Unpress { button: PhysicalButton },
    Unknown,
}

pub struct GPIOState {
    states: [AtomicBool; 5],
}

impl ::std::default::Default for GPIOState {
    fn default() -> Self {
        GPIOState {
            states: [
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
                AtomicBool::new(false),
            ],
        }
    }
}

pub fn decode(ev: &input_event, outer_state: &InputDeviceState) -> Option<InputEvent> {
    let state = match outer_state {
        InputDeviceState::GPIOState(ref state_arc) => state_arc,
        _ => unreachable!(),
    };
    match ev._type {
        ecodes::EV_SYN => {
            /* safely ignored. sync event*/
            None
        }
        ecodes::EV_KEY => {
            let p = match ev.code {
                ecodes::KEY_HOME => {
                    state.states[0].store(ev.value != 0, Ordering::Relaxed);
                    PhysicalButton::MIDDLE
                }
                ecodes::KEY_LEFT => {
                    state.states[1].store(ev.value != 0, Ordering::Relaxed);
                    PhysicalButton::LEFT
                }
                ecodes::KEY_RIGHT => {
                    state.states[2].store(ev.value != 0, Ordering::Relaxed);
                    PhysicalButton::RIGHT
                }
                ecodes::KEY_POWER => {
                    state.states[3].store(ev.value != 0, Ordering::Relaxed);
                    PhysicalButton::POWER
                }
                ecodes::KEY_WAKEUP => {
                    state.states[4].store(ev.value != 0, Ordering::Relaxed);
                    PhysicalButton::WAKEUP
                }
                _ => return None,
            };

            let event = if ev.value != 0 {
                GPIOEvent::Press { button: p }
            } else {
                GPIOEvent::Unpress { button: p }
            };
            Some(InputEvent::GPIO { event })
        }
        _ => {
            // Shouldn't happen
            error!(
                "Unknown event on PhysicalButtonHandler (type: {0})",
                ev._type
            );
            None
        }
    }
}
