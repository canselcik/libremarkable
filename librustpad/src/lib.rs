#![feature(integer_atomics)]
#![feature(const_size_of)]

#[macro_use]
extern crate ioctl_gen;

extern crate libc;
extern crate mmap;
extern crate image;
extern crate rusttype;

pub mod mxc_types;
pub mod fb;
pub mod fbio;
pub mod fbdraw;
pub mod refresh;