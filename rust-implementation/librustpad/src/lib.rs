#![feature(integer_atomics)]
#![feature(const_size_of)]

#[macro_use]
extern crate ioctl_gen;

extern crate libc;
extern crate mmap;
extern crate image;
extern crate rusttype;
extern crate epoll;

pub extern crate evdev;
pub extern crate line_drawing;

pub mod mxc_types;
pub mod fb;
pub mod fbio;
pub mod fbdraw;
pub mod refresh;
pub mod ev;

pub mod ev_debug;
pub mod physical_buttons;
pub mod multitouch;

pub use evdev::Device;
pub use evdev::raw::input_event;
