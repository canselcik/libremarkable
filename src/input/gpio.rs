use evdev::raw::input_event;
use input::{InputDeviceState, InputEvent};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(PartialEq, Copy, Clone)]
pub enum PhysicalButton {
    LEFT,
    MIDDLE,
    RIGHT,
    POWER,
    WAKEUP,
}

#[derive(PartialEq, Copy, Clone)]
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
        0 => {
            /* safely ignored. sync event*/
            None
        }
        1 => {
            let (p, before_state) = match ev.code {
                102 => (
                    PhysicalButton::MIDDLE,
                    state.states[0].fetch_and(ev.value != 0, Ordering::Relaxed),
                ),
                105 => (
                    PhysicalButton::LEFT,
                    state.states[1].fetch_and(ev.value != 0, Ordering::Relaxed),
                ),
                106 => (
                    PhysicalButton::RIGHT,
                    state.states[2].fetch_and(ev.value != 0, Ordering::Relaxed),
                ),
                116 => (
                    PhysicalButton::POWER,
                    state.states[3].fetch_and(ev.value != 0, Ordering::Relaxed),
                ),
                143 => (
                    PhysicalButton::WAKEUP,
                    state.states[4].fetch_and(ev.value != 0, Ordering::Relaxed),
                ),
                _ => return None,
            };

            // Edge trigger -- debouncing
            let new_state = ev.value != 0;
            if new_state == before_state {
                return None;
            }

            let event = if new_state {
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
