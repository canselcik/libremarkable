#![feature(integer_atomics)]
#![feature(box_syntax)]
#![feature(nll)]

#[cfg(not(feature = "enable-runtime-benchmarking"))]
#[macro_export]
macro_rules! start_bench {
    ($stopwatch_path:ident, $name:ident) => {};
}

#[cfg(not(feature = "enable-runtime-benchmarking"))]
#[macro_export]
macro_rules! end_bench {
    ($name:expr) => {};
}

#[cfg(feature = "enable-runtime-benchmarking")]
#[macro_export]
macro_rules! start_bench {
    ($stopwatch_path:ident, $name:ident) => {
        let $name = $stopwatch_path::Stopwatch::start_new();
    };
}

#[cfg(feature = "enable-runtime-benchmarking")]
#[macro_export]
macro_rules! end_bench {
    ($name:ident) => {
        let dur = $name.elapsed();
        let s = dur.as_secs();
        let mut us = dur.subsec_nanos() / 1000;
        let ms = us / 1000;
        us -= ms * 1000;
        println!("'{}' took {}s {}ms {}us", stringify!($name), s, ms, us);
    };
}

#[macro_use]
extern crate log;

#[macro_use]
extern crate ioctl_gen;

extern crate aabb_quadtree;
extern crate atomic;
extern crate hlua;
extern crate libc;
extern crate mmap;
extern crate rusttype;
extern crate zstd;
extern crate fxhash;

pub extern crate cgmath;
pub extern crate epoll;
pub extern crate evdev;
pub extern crate image;
pub extern crate line_drawing;
pub extern crate stopwatch;

/// One of the core components, allowing output and refresh of the EInk display
pub mod framebuffer;

/// The other core component, allowing decoding of the three input devices present on the tablet
pub mod input;

/// Simple battery and charging status provider
pub mod battery;

/// Contains the `ApplicationContext`, which is a general framework that can be used to either build
/// your application or design your I/O code after. It uses rudimentary UI elements and adds them
/// to a scene after wrapping them in `UIElementWrapper`. None of these are mandatory to be used.
/// You can choose to entirely ignore the `ApplicationContext` and `ui_extensions` and interact
/// with the `framebuffer` and `input` devices directly.
pub mod appctx;
pub mod ui_extensions;
