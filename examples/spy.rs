#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::Mutex;

extern crate libc;

use libc::c_int;
use libc::intptr_t;

#[macro_use]
extern crate redhook;

extern crate libremarkable;

use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::mxcfb::*;
use libremarkable::framebuffer::screeninfo::VarScreeninfo;

lazy_static! {
    static ref DIST_DITHER: Mutex<HashMap<u32, u32>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    static ref DIST_WAVEFORM: Mutex<HashMap<u32, u32>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    static ref DIST_QUANT: Mutex<HashMap<u32, u32>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    static ref DIST_FLAGS: Mutex<HashMap<u32, u32>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
    static ref DIST_TEMP: Mutex<HashMap<u32, u32>> = {
        let m = HashMap::new();
        Mutex::new(m)
    };
}

#[derive(Debug)]
#[repr(C)]
struct ioctl_intercept_event {
    fd: libc::c_int,
    request: NativeWidthType,
    p1: intptr_t,
    p2: intptr_t,
    p3: intptr_t,
    p4: intptr_t,
    ret: libc::c_int,
}

fn add_entry(map: &mut HashMap<u32, u32>, ent: u32) {
    let count = match map.get(&ent) {
        Some(count) => *count,
        None => 0,
    };
    map.insert(ent, count + 1);
    for (key, value) in &*map {
        println!("  {0:x} --> {1} times", key, value);
    }
}

fn handle_send_update(event: ioctl_intercept_event) {
    let mut distdither = DIST_DITHER.lock().unwrap();
    let mut distwave = DIST_WAVEFORM.lock().unwrap();
    let mut distquant = DIST_QUANT.lock().unwrap();
    let mut distflags = DIST_FLAGS.lock().unwrap();
    let mut disttemp = DIST_TEMP.lock().unwrap();

    let update_data = event.p1 as *mut mxcfb_update_data;

    println!("===WAVEFORM DISTRIBUTION===");
    add_entry(&mut distwave, unsafe { (*update_data).waveform_mode });
    println!("===DITHERING DISTRIBUTION===");
    add_entry(&mut distdither, unsafe { (*update_data).dither_mode }
        as u32);
    println!("===TEMP DISTRIBUTION===");
    add_entry(&mut disttemp, unsafe { (*update_data).temp } as u32);
    println!("===QUANT DISTRIBUTION===");
    add_entry(&mut distquant, unsafe { (*update_data).quant_bit } as u32);
    println!("===FLAGS DISTRIBUTION===");
    add_entry(&mut distflags, unsafe { (*update_data).flags } as u32);

    unsafe {
        println!(
            "mxcfb_send_update(fd: {0}, updateData: {1:#?}) = {2}",
            event.fd, *update_data, event.ret
        );
    }
}

fn handle_wait_update_complete(event: ioctl_intercept_event) {
    unsafe {
        let update_data = event.p1 as *mut mxcfb_update_marker_data;
        println!(
            "mxcfb_wait_update_complete(fd: {0}, updateData: {1:#?}) = {2}",
            event.fd, *update_data, event.ret
        );
    }
}

hook! {
  unsafe fn ioctl(fd: c_int, request: NativeWidthType, p1: intptr_t, p2: intptr_t, p3: intptr_t, p4: intptr_t) -> c_int => ioctl_hook {
    if request == FBIOPUT_VSCREENINFO {
        let info = p1 as *mut VarScreeninfo;
        println!("fb_var_screeninfo before FBIOPUT_VSCREENINFO is called: {0:#?}", *info);
    }

    let res = real!(ioctl)(fd, request, p1, p2, p3, p4);
    let event = ioctl_intercept_event {
      fd: fd,
      request: request,
      p1: p1,
      p2: p2,
      p3: p3,
      p4: p4,
      ret: res,
    };

    // For xochitl /dev/fb0 is FD=3. For remarkable-test program, it is FD=8.
    if fd != 3 {
      return res;
    }

    match request {
        FBIOGETCMAP => println!("FBIOGETCMAP({0:#?})", event),
        FBIOPUTCMAP => println!("FBIOPUTCMAP({0:#?})", event),
        FBIO_CURSOR => println!("FBIO_CURSOR({0:#?})", event),
        FBIOPAN_DISPLAY => println!("FBIOPAN_DISPLAY({0:#?})", event),
        FBIOPUT_VSCREENINFO => println!("FBIOPUT_VSCREENINFO(after: {0:#?}) = {1}", *(p1 as *mut VarScreeninfo), res),
        FBIOGET_VSCREENINFO => println!("FBIOGET_VSCREENINFO(out: {0:#?})", p1 as *mut VarScreeninfo),
        FBIOGET_FSCREENINFO => println!("FBIOGET_FSCREENINFO(out: {0:#?})", event),
        MXCFB_WAIT_FOR_UPDATE_COMPLETE => handle_wait_update_complete(event),
        MXCFB_SEND_UPDATE => handle_send_update(event),
        _ => println!("unknown_ioctl({0:#?})", event),
    }
    return res;
  }
}
