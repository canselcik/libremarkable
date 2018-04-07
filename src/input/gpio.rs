use input::{UnifiedInputHandler,InputEvent};
use evdev::raw::input_event;
use rb::RbProducer;


#[derive(PartialEq, Copy, Clone)]
pub enum PhysicalButton {
    LEFT,
    MIDDLE,
    RIGHT,
}

#[derive(PartialEq, Copy, Clone)]
pub enum GPIOEvent {
    Press { button: PhysicalButton },
    Unpress { button: PhysicalButton },
    Unknown,
}

pub struct GPIOState {
    states: [bool; 3],
}

impl GPIOState {
    pub fn new() -> GPIOState {
        GPIOState {
            states: [false; 3],
        }
    }
}

impl<'a> UnifiedInputHandler<'a> {
    pub fn gpio_handler(&mut self, ev: &input_event) {
        match ev._type {
            0 => { /* safely ignored. sync event*/ }
            1 => {
                let (p, before_state) = match ev.code {
                    102 => {
                        let ret = (PhysicalButton::MIDDLE, self.gpio.states[1]);
                        self.gpio.states[1] = ev.value != 0;
                        ret
                    }
                    105 => {
                        let ret = (PhysicalButton::LEFT, self.gpio.states[0]);
                        self.gpio.states[0] = ev.value != 0;
                        ret
                    }
                    106 => {
                        let ret = (PhysicalButton::RIGHT, self.gpio.states[2]);
                        self.gpio.states[2] = ev.value != 0;
                        ret
                    }
                    _ => return,
                };

                // Edge trigger -- debouncing
                let new_state = ev.value != 0;
                if new_state == before_state {
                    return;
                }

                let event = match new_state {
                    true => GPIOEvent::Press {
                        button: p,
                    },
                    false => GPIOEvent::Unpress {
                        button: p,
                    },
                };
                self.ringbuffer.write(&[InputEvent::GPIO { event }]).unwrap();
            }
            _ => {
                // Shouldn't happen
                error!("Unknown event on PhysicalButtonHandler (type: {0})", ev._type);
            }
        }
    }
}
