// Used event codes (input events as standardized in the linux kernel)
// See https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h

// Event types
pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01; // BTN prefixed constants are of type EV_KEY, too
pub const EV_ABS: u16 = 0x03;

// Syn events seem to be used for syncing and configuring the events themselves
pub const SYN_REPORT: u16 = 0x00;
// SYN_CONFIG, MT_REPORT, SYN_DROPPED, SYN_MAX, SYN_CNT

// Absolute Multitouch (Touchpad on reMarkable)
pub const ABS_MT_SLOT: u16 = 0x2f; // = 47
pub const ABS_MT_TOUCH_MAJOR: u16 = 0x30; // = 48
pub const ABS_MT_TOUCH_MINOR: u16 = 0x31; // = 49
pub const ABS_MT_ORIENTATION: u16 = 0x34; // = 52
pub const ABS_MT_POSITION_X: u16 = 0x35; // = 53
pub const ABS_MT_POSITION_Y: u16 = 0x36; // = 54
pub const ABS_MT_TRACKING_ID: u16 = 0x39; // = 57
pub const ABS_MT_PRESSURE: u16 = 0x3a; // = 58
                                       // Absolute (Wacom Digitizer on reMarkable)
pub const ABS_PRESSURE: u16 = 0x18; // = 24
pub const ABS_DISTANCE: u16 = 0x19; // = 25
pub const ABS_TILT_X: u16 = 0x1a; // = 26
pub const ABS_TILT_Y: u16 = 0x1b; // = 27
pub const ABS_X: u16 = 0x00; // = 0
pub const ABS_Y: u16 = 0x01; // = 1

// Keys (Wacom Digitizer buttons on reMarkable)
pub const BTN_TOOL_PEN: u16 = 0x140; // = 320
pub const BTN_TOOL_RUBBER: u16 = 0x141; // = 321
pub const BTN_TOUCH: u16 = 0x14a; // = 330
pub const BTN_STYLUS: u16 = 0x14b; // = 331
pub const BTN_STYLUS2: u16 = 0x14c; // = 332
                                    // Keys (GPIOs on reMarkable)
pub const KEY_HOME: u16 = 0x66; // = 102 (aka middle button on the reMarkable)
pub const KEY_LEFT: u16 = 0x69; // = 105
pub const KEY_RIGHT: u16 = 0x6a; // = 106
pub const KEY_POWER: u16 = 0x74; // = 116
pub const KEY_WAKEUP: u16 = 0x8f; // = 143
