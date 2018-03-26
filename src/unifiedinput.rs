use evdev::Device;
use evdev::raw::input_event;

use ev;
use std;
use mxc_types;
use rb;

use rb::RbProducer;

const WACOM_HSCALAR: f32 = (mxc_types::DISPLAYWIDTH as f32) / (mxc_types::WACOMWIDTH as f32);
const WACOM_VSCALAR: f32 = (mxc_types::DISPLAYHEIGHT as f32) / (mxc_types::WACOMHEIGHT as f32);

const MT_HSCALAR: f32 = (mxc_types::DISPLAYWIDTH as f32) / (mxc_types::MTWIDTH as f32);
const MT_VSCALAR: f32 = (mxc_types::DISPLAYHEIGHT as f32) / (mxc_types::MTHEIGHT as f32);

unsafe impl<'a> Send for UnifiedInputHandler<'a> {}

unsafe impl<'a> Sync for UnifiedInputHandler<'a> {}

pub struct WacomState {
    last_x: u16,
    last_y: u16,
    last_xtilt: u16,
    last_ytilt: u16,
    last_dist: u16,
    last_pressure: u16,
    previous_x: u16,
    previous_y: u16,
}

#[repr(u16)]
#[derive(PartialEq, Copy, Clone)]
pub enum WacomPen {
    ToolPen = 320,
    ToolRubber = 321,
    Touch = 330,
    Stylus = 331,
    Stylus2 = 332,
}

#[derive(PartialEq, Copy, Clone)]
pub enum WacomEvent {
    InstrumentChange { pen: WacomPen, state: bool },
    Hover { y: u16, x: u16, distance: u16, tilt_x: u16, tilt_y: u16 },
    Draw { y: u16, x: u16, pressure: u16, tilt_x: u16, tilt_y: u16, prevy: u16, prevx: u16 },
    Unknown,
}

pub struct GPIOState {
    states: [bool; 3],
}

pub struct MultitouchState {
    last_touch_size: u8,
    last_touch_id: u16,
    last_x: u16,
    last_y: u16,
    last_finger_id: u16,
    currently_touching: bool,
}

#[derive(PartialEq, Copy, Clone)]
pub enum MultitouchEvent {
    Touch { gesture_seq: u16, finger_id: u16, y: u16, x: u16 },
    Unknown,
}


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

pub struct UnifiedInputHandler<'a> {
    wacom: WacomState,
    gpio: GPIOState,
    mt: MultitouchState,
    ringbuffer: &'a rb::Producer<InputEvent>,
}

#[derive(PartialEq, Copy, Clone)]
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

impl<'a> UnifiedInputHandler<'a> {
    pub fn new(ringbuffer: &rb::Producer<InputEvent>) -> UnifiedInputHandler {
        return UnifiedInputHandler {
            gpio: GPIOState {
                states: [false; 3],
            },
            wacom: WacomState {
                last_x: 0,
                last_y: 0,
                last_xtilt: 0,
                last_ytilt: 0,
                last_dist: 0,
                last_pressure: 0,
                previous_x: 0,
                previous_y: 0,
            },
            mt: MultitouchState {
                last_touch_size: 0,
                last_touch_id: 0,
                last_x: 0,
                last_y: 0,
                last_finger_id: 0,
                currently_touching: false,
            },
            ringbuffer,
        };
    }

    fn wacom_handler(&mut self, ev: &input_event) {
        match ev._type {
            0 => { /* sync */ }
            1 => {
                /* key (device detected - device out of range etc.) */
                if ev.code >= WacomPen::ToolPen as u16 && ev.code <= WacomPen::Stylus2 as u16 {
                    let event = WacomEvent::InstrumentChange {
                        pen: unsafe { std::mem::transmute_copy(&ev.code) },
                        state: ev.value != 0,
                    };
                    self.ringbuffer.write(&[InputEvent::WacomEvent { event }]).unwrap();
                } else {
                    warn!("Unknown key event code for Wacom [type: {0} code: {1} value: {2}]",
                             ev._type, ev.code, ev.value);
                }
            }
            3 => {
                // Absolute
                match ev.code {
                    25 => { // distance up to 255
                        self.wacom.last_dist = ev.value as u16;
                        let event = WacomEvent::Hover {
                            y: (self.wacom.last_y as f32 * WACOM_VSCALAR) as u16,
                            x: (self.wacom.last_x as f32 * WACOM_HSCALAR) as u16,
                            distance: self.wacom.last_dist,
                            tilt_x: self.wacom.last_xtilt,
                            tilt_y: self.wacom.last_ytilt,
                        };
                        self.ringbuffer.write(&[InputEvent::WacomEvent { event }]).unwrap();
                    }
                    26 => { // xtilt -9000 to 9000
                        self.wacom.last_xtilt = ev.value as u16;
                    }
                    27 => { // ytilt -9000 to 9000
                        self.wacom.last_ytilt = ev.value as u16;
                    }
                    24 => { // contact made with pressure val up to 4095
                        self.wacom.last_pressure = ev.value as u16;
                        let event = WacomEvent::Draw {
                            x: (self.wacom.last_x as f32 * WACOM_HSCALAR) as u16,
                            y: (self.wacom.last_y as f32 * WACOM_VSCALAR) as u16,
                            pressure: self.wacom.last_pressure,
                            tilt_x: self.wacom.last_xtilt,
                            tilt_y: self.wacom.last_ytilt,
                            prevy: self.wacom.previous_y,
                            prevx: self.wacom.previous_x,
                        };
                        self.ringbuffer.write(&[InputEvent::WacomEvent { event }]).unwrap();
                    }
                    0x0 => { // x and y are inverted due to remarkable
                        let val = ev.value as u16;
                        self.wacom.previous_y = self.wacom.last_y;
                        self.wacom.last_y = mxc_types::WACOMHEIGHT - val;
                    }
                    0x1 => {
                        self.wacom.previous_x = self.wacom.last_x;
                        self.wacom.last_x = ev.value as u16;
                    }
                    _ => {
                        warn!("Unknown absolute event code for Wacom [type: {0} code: {1} value: {2}]",
                                 ev._type, ev.code, ev.value);
                    }
                }
            }
            _ => {
                warn!("Unknown event TYPE for Wacom [type: {0} code: {1} value: {2}]",
                         ev._type, ev.code, ev.value);
            }
        }
    }

    fn multitouch_handler(&mut self, ev: &input_event) {
        match ev._type {
            0 => { /* sync */ }
            3 => {
                // Absolute
                match ev.code {
                    47 => {
                        self.mt.last_finger_id = ev.value as u16;
                    }
                    53 => {
                        let val = ev.value as u16;
                        self.mt.last_x = mxc_types::MTWIDTH - val;
                    }
                    54 => {
                        let val = ev.value as u16;
                        self.mt.last_y = mxc_types::MTHEIGHT - val;


                        let y = (self.mt.last_y as f32 * MT_VSCALAR) as u16;
                        let x = (self.mt.last_x as f32 * MT_HSCALAR) as u16;
                        let event = MultitouchEvent::Touch {
                            gesture_seq: self.mt.last_touch_id,
                            finger_id: self.mt.last_finger_id,
                            y, x,
                        };

                        self.ringbuffer.write(&[InputEvent::MultitouchEvent { event }]).unwrap();
                    }
                    52 | 48 | 58 =>
                        debug!("unknown_absolute_touch_event(code={0}, value={1})", ev.code, ev.value),
                    49 => {
                        // potentially incorrect
                        self.mt.last_touch_size = ev.value as u8;
                    }
                    57 => {
                        match ev.value {
                            -1 => {
                                self.mt.currently_touching = false;
                            }
                            touch_id => {
                                self.mt.last_touch_id = touch_id as u16;
                                self.mt.currently_touching = true;
                            }
                        }
                    }
                    // very unlikely
                    _ => warn!("Unknown event code for multitouch [type: {0} code: {1} value: {2}]",
                                 ev._type, ev.code, ev.value),
                }
            }
            _ =>
                warn!("Unknown event type for [type: {0} code: {1} value: {2}]",
                     ev._type, ev.code, ev.value),
        }
    }

    fn gpio_handler(&mut self, ev: &input_event) {
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

impl<'a> ev::EvdevHandler for UnifiedInputHandler<'a> {
    fn on_init(&mut self, name: String, _device: &mut Device) {
        info!("'{0}' input device EPOLL initialized", name);
    }

    fn on_event(&mut self, device: &String, ev: input_event) {
        match device.as_ref() {
            "Wacom I2C Digitizer" => self.wacom_handler(&ev),
            "cyttsp5_mt" => self.multitouch_handler(&ev),
            "gpio-keys" => self.gpio_handler(&ev),
            _ => {}
        }
    }
}