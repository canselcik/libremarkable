/// Contains the epoll code to read from the device when the worker thread is woken up by the
/// kernel upon new data to consume
#[cfg(feature = "input")]
pub mod ev;

/// Contains the code to decode Wacom events
#[cfg(feature = "input")]
pub mod wacom;

/// Contains the code to decode physical button events
#[cfg(feature = "input")]
pub mod gpio;

/// Contains the code to decode multitouch events
#[cfg(feature = "input")]
pub mod multitouch;

/// Contains the ev codes in use
pub mod ecodes;

/// Figures out where the input devices are as well as
/// device dependant properties
#[cfg(feature = "scan")]
pub mod scan;

#[derive(PartialEq, Copy, Clone, Debug, Hash, Eq)]
pub enum InputDevice {
    Wacom,
    Multitouch,
    GPIO,
    Unknown,
}

#[cfg(feature = "input")]
pub enum InputDeviceState {
    WacomState(std::sync::Arc<wacom::WacomState>),
    MultitouchState(std::sync::Arc<multitouch::MultitouchState>),
    GPIOState(std::sync::Arc<gpio::GPIOState>),
}

#[cfg(feature = "input")]
use std::sync::Arc;
#[cfg(feature = "input")]
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

#[cfg(feature = "input")]
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

#[repr(u16)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
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

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Finger {
    pub tracking_id: i32,

    pub pos: cgmath::Point2<u16>,
    pub(crate) pos_updated: bool, // Report motion at SYN_REPORT?

    pub(crate) last_pressed: bool,
    pub pressed: bool,
}

impl Default for Finger {
    fn default() -> Finger {
        Finger {
            tracking_id: -1, // -1 should never be seen by a InputEvent receiver
            pos: cgmath::Point2 {
                x: u16::MAX,
                y: u16::MAX,
            },
            pos_updated: false,
            last_pressed: false,
            pressed: false,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum MultitouchEvent {
    Press { finger: Finger },
    Release { finger: Finger },
    Move { finger: Finger },
    Unknown,
}

impl MultitouchEvent {
    pub fn finger(&self) -> Option<&Finger> {
        match self {
            MultitouchEvent::Press { ref finger }
            | MultitouchEvent::Release { ref finger }
            | MultitouchEvent::Move { ref finger } => Some(finger),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum PhysicalButton {
    LEFT,
    MIDDLE,
    RIGHT,
    POWER,
    WAKEUP,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum GPIOEvent {
    Press { button: PhysicalButton },
    Unpress { button: PhysicalButton },
    Unknown,
}

#[derive(PartialEq, Clone, Debug)]
pub enum InputEvent {
    WacomEvent { event: WacomEvent },
    MultitouchEvent { event: MultitouchEvent },
    GPIO { event: GPIOEvent },
    Unknown {},
}

impl Default for InputEvent {
    fn default() -> InputEvent {
        InputEvent::Unknown {}
    }
}
