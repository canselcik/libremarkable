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
        let mut us = dur.subsec_micros();
        let ms = us / 1000;
        us -= ms * 1000;
        println!("'{}' took {}s {}ms {}us", stringify!($name), s, ms, us);
    };
}

#[cfg(feature = "framebuffer")]
#[macro_use(io, ioc, iow, iowr)]
extern crate ioctl_gen;

pub use cgmath;
pub use epoll;
pub use evdev;
#[cfg(feature = "framebuffer")]
pub use image;
#[cfg(feature = "framebuffer")]
pub use line_drawing;
#[cfg(feature = "enable-runtime-benchmarking")]
pub use stopwatch;

/// One of the core components, allowing output and refresh of the EInk display
#[cfg(feature = "framebuffer")]
pub mod framebuffer;

/// The other core component, allowing decoding of the three input devices present on the tablet
pub mod input;

/// Device dimensions.
pub mod dimensions;

/// Simple battery and charging status provider
pub mod battery;

// TODO: Docs
pub mod device;

/// Contains the `ApplicationContext`, which is a general framework that can be used to either build
/// your application or design your I/O code after. It uses rudimentary UI elements and adds them
/// to a scene after wrapping them in `UIElementWrapper`. None of these are mandatory to be used.
/// You can choose to entirely ignore the `ApplicationContext` and `ui_extensions` and interact
/// with the `framebuffer` and `input` devices directly.
#[cfg(all(feature = "framebuffer", feature = "appctx"))]
pub mod appctx;
#[cfg(all(feature = "framebuffer", feature = "appctx"))]
pub mod ui_extensions;
