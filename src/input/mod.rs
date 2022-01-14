/// Contains the epoll code to read from the device when the worker thread is woken up by the
/// kernel upon new data to consume
pub mod ev;

/// Contains the code to decode Wacom events
pub mod wacom;

/// Contains the code to decode physical button events
pub mod gpio;

/// Contains the code to decode multitouch events
pub mod multitouch;

/// Contains the ev codes in use
pub mod ecodes;

/// Figures out where the input devices are as well as
/// device dependant properties
pub mod scan;

#[derive(PartialEq, Copy, Clone, Debug, Hash, Eq)]
pub enum InputDevice {
    Wacom,
    Multitouch,
    GPIO,
    Unknown,
}

pub enum InputDeviceState {
    WacomState(std::sync::Arc<wacom::WacomState>),
    MultitouchState(std::sync::Arc<multitouch::MultitouchState>),
    GPIOState(std::sync::Arc<gpio::GPIOState>),
}

use std::sync::Arc;
impl Clone for InputDeviceState {
    fn clone(&self) -> InputDeviceState {
        match self {
            InputDeviceState::WacomState(ref state) => {
                InputDeviceState::WacomState(Arc::clone(state))
            }
            InputDeviceState::MultitouchState(ref state) => {
                InputDeviceState::MultitouchState(Arc::clone(state))
            }
            InputDeviceState::GPIOState(ref state) => {
                InputDeviceState::GPIOState(Arc::clone(state))
            }
        }
    }
}

impl InputDeviceState {
    pub fn new(dev: InputDevice) -> InputDeviceState {
        match dev {
            InputDevice::GPIO => InputDeviceState::GPIOState(Arc::new(gpio::GPIOState::default())),
            InputDevice::Wacom => {
                InputDeviceState::WacomState(Arc::new(wacom::WacomState::default()))
            }
            InputDevice::Multitouch => {
                InputDeviceState::MultitouchState(Arc::new(multitouch::MultitouchState::default()))
            }
            _ => unreachable!(),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum InputEvent {
    WacomEvent { event: wacom::WacomEvent },
    MultitouchEvent { event: multitouch::MultitouchEvent },
    GPIO { event: gpio::GPIOEvent },
    Unknown {},
}

impl Default for InputEvent {
    fn default() -> InputEvent {
        InputEvent::Unknown {}
    }
}
