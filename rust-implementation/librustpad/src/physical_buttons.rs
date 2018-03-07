use evdev::Device;
use evdev::raw::input_event;

use ev;

pub enum PhysicalButton {
    LEFT,
    MIDDLE,
    RIGHT,
}

pub struct PhysicalButtonHandler {
    pub name: String,
    pub callback: fn(PhysicalButton, u16),
    states: [bool;3],
}

impl PhysicalButtonHandler {
    pub fn get_instance(callback: fn(PhysicalButton, u16)) -> PhysicalButtonHandler {
        return PhysicalButtonHandler {
            name: "GPIO".to_owned(),
            callback,
            states: [false, false, false],
        };
    }
}

impl ev::EvdevHandler for PhysicalButtonHandler {
    fn on_init(&mut self, name: String, _device: &mut Device) {
        println!("INFO: '{0}' input device EPOLL initialized", name);
        self.name = name;
    }

    fn on_event(&mut self, ev: input_event) {
        match ev._type {
            0 => { /* safely ignored. sync event*/ }
            1 => {
                let (p, before_state) = match ev.code {
                    102 => {
                        let ret = (PhysicalButton::MIDDLE, self.states[1]);
                        self.states[1] = ev.value != 0;
                        ret
                    },
                    105 => {
                        let ret = (PhysicalButton::LEFT, self.states[0]);
                        self.states[0] = ev.value != 0;
                        ret
                    },
                    106 => {
                        let ret = (PhysicalButton::RIGHT, self.states[2]);
                        self.states[2] = ev.value != 0;
                        ret
                    },
                    _ => return,
                };

                // Edge trigger -- debouncing
                if (ev.value != 0) != before_state {
                    (self.callback)(p, ev.value as u16);
                }
            }
            _ => {
                // Shouldn't happen
                println!(
                    "[WARN] Unknown event on PhysicalButtonHandler (type: {0})",
                    ev._type
                );
            }
        }
    }
}
