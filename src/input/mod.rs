/// Contains the epoll code to read from the device when the worker thread is woken up by the
/// kernel upon new data to consume
pub mod ev;

/// Contains the code to decode Wacom events
pub mod wacom;

/// Contains the code to decode physical button events
pub mod gpio;

/// Contains the code to decode multitouch events
pub mod multitouch;

#[derive(PartialEq, Copy, Clone)]
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

use evdev;

/// Trait to implement to be dispatched evdev events by the `start_evdev` function
pub trait EvdevHandler {
    fn on_init(&mut self, name: String, device: &mut evdev::Device);
    fn on_event(&mut self, device: &String, event: evdev::raw::input_event);
}

unsafe impl<'a> Send for UnifiedInputHandler<'a> {}
unsafe impl<'a> Sync for UnifiedInputHandler<'a> {}

use rb;
pub struct UnifiedInputHandler<'a> {
    pub wacom: wacom::WacomState,
    pub gpio: gpio::GPIOState,
    pub mt: multitouch::MultitouchState,
    pub ringbuffer: &'a rb::Producer<InputEvent>,
}

impl<'a> UnifiedInputHandler<'a> {
    pub fn new(ringbuffer: &rb::Producer<InputEvent>) -> UnifiedInputHandler {
        return UnifiedInputHandler {
            gpio: gpio::GPIOState::new(),
            wacom: wacom::WacomState::new(),
            mt: multitouch::MultitouchState::new(),
            ringbuffer,
        };
    }
}

impl<'a> EvdevHandler for UnifiedInputHandler<'a> {
    fn on_init(&mut self, name: String, _device: &mut evdev::Device) {
        info!("'{0}' input device EPOLL initialized", name);
    }

    fn on_event(&mut self, device: &String, ev: evdev::raw::input_event) {
        match device.as_ref() {
            "Wacom I2C Digitizer" => self.wacom_handler(&ev),
            "cyttsp5_mt" => self.multitouch_handler(&ev),
            "gpio-keys" => self.gpio_handler(&ev),
            _ => {}
        }
    }
}
