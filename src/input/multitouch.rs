use framebuffer::common::{DISPLAYHEIGHT, DISPLAYWIDTH, MTHEIGHT, MTWIDTH};

use input::{InputEvent, UnifiedInputHandler};

use evdev::raw::input_event;

const MT_HSCALAR: f32 = (DISPLAYWIDTH as f32) / (MTWIDTH as f32);
const MT_VSCALAR: f32 = (DISPLAYHEIGHT as f32) / (MTHEIGHT as f32);

pub struct MultitouchState {
    last_touch_size: u8,
    last_touch_id: u16,
    last_x: u16,
    last_y: u16,
    last_finger_id: u16,
    currently_touching: bool,
}

impl MultitouchState {
    pub fn new() -> MultitouchState {
        MultitouchState {
            last_touch_size: 0,
            last_touch_id: 0,
            last_x: 0,
            last_y: 0,
            last_finger_id: 0,
            currently_touching: false,
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

impl UnifiedInputHandler {
    pub fn multitouch_handler(&mut self, ev: &input_event) {
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
                        self.mt.last_x = MTWIDTH - val;
                    }
                    54 => {
                        let val = ev.value as u16;
                        self.mt.last_y = MTHEIGHT - val;

                        let y = (self.mt.last_y as f32 * MT_VSCALAR) as u16;
                        let x = (self.mt.last_x as f32 * MT_HSCALAR) as u16;
                        let event = MultitouchEvent::Touch {
                            gesture_seq: self.mt.last_touch_id,
                            finger_id: self.mt.last_finger_id,
                            y,
                            x,
                        };

                        self.tx.send(InputEvent::MultitouchEvent { event }).unwrap();
                    }
                    52 | 48 | 58 => debug!(
                        "unknown_absolute_touch_event(code={0}, value={1})",
                        ev.code, ev.value
                    ),
                    49 => {
                        // potentially incorrect
                        self.mt.last_touch_size = ev.value as u8;
                    }
                    57 => match ev.value {
                        -1 => {
                            self.mt.currently_touching = false;
                        }
                        touch_id => {
                            self.mt.last_touch_id = touch_id as u16;
                            self.mt.currently_touching = true;
                        }
                    },
                    // very unlikely
                    _ => warn!(
                        "Unknown event code for multitouch [type: {0} code: {1} value: {2}]",
                        ev._type, ev.code, ev.value
                    ),
                }
            }
            _ => warn!(
                "Unknown event type for [type: {0} code: {1} value: {2}]",
                ev._type, ev.code, ev.value
            ),
        }
    }
}
