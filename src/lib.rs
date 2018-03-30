#![feature(integer_atomics)]
#![feature(const_size_of)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate ioctl_gen;

extern crate libc;
extern crate mmap;
extern crate rusttype;
extern crate aabb_quadtree;
extern crate hlua;

pub extern crate image;
pub extern crate epoll;
pub extern crate rb;
pub extern crate evdev;
pub extern crate line_drawing;

pub mod mxc_types;
pub mod fb;
pub mod fbio;
pub mod fbdraw;
pub mod refresh;
pub mod ev;

pub mod unifiedinput;

mod uix_lua;
pub mod uix;

