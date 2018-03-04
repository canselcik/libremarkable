extern crate libc;
use libc::{c_int,intptr_t};

#[macro_use]
extern crate redhook;

extern crate librustpad;
use librustpad::mxc_types;

#[derive(Debug)]
#[repr(C)]
struct ioctl_intercept_event {
  fd: c_int,
  request: u32,
  p1: intptr_t,
  p2: intptr_t,
  p3: intptr_t,
  p4: intptr_t,
  ret: c_int,
}


fn handle_send_update(event: ioctl_intercept_event) {
  unsafe {
    let update_data = event.p1 as *mut mxc_types::mxcfb_update_data;
    println!("mxcfb_send_update(fd: {0}, updateData: {1:#?}) = {2}", event.fd, *update_data, event.ret);
  }
}

fn handle_wait_update_complete(event: ioctl_intercept_event) {
  unsafe {
    let update_data = event.p1 as *mut mxc_types::mxcfb_update_marker_data;
    println!("mxcfb_wait_update_complete(fd: {0}, updateData: {1:#?}) = {2}", event.fd, *update_data, event.ret);
  }
}

hook! {
  unsafe fn ioctl(fd: c_int, request: u32, p1: intptr_t, p2: intptr_t, p3: intptr_t, p4: intptr_t) -> c_int => ioctl_hook {
    if request == mxc_types::FBIOPUT_VSCREENINFO {
        let info = p1 as *mut mxc_types::VarScreeninfo;
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

    match request {
        mxc_types::FBIOGETCMAP => println!("FBIOGETCMAP({0:#?})", event),
        mxc_types::FBIOPUTCMAP => println!("FBIOPUTCMAP({0:#?})", event),
        mxc_types::FBIO_CURSOR => println!("FBIO_CURSOR({0:#?})", event),
        mxc_types::FBIOPAN_DISPLAY => println!("FBIOPAN_DISPLAY({0:#?})", event),
        mxc_types::FBIOPUT_VSCREENINFO => println!("FBIOPUT_VSCREENINFO(after: {0:#?}) = {1}", *(p1 as *mut mxc_types::VarScreeninfo), res),
        mxc_types::FBIOGET_VSCREENINFO => println!("FBIOGET_VSCREENINFO(out: {0:#?})", p1 as *mut mxc_types::VarScreeninfo),
        mxc_types::FBIOGET_FSCREENINFO => println!("FBIOGET_FSCREENINFO(out: {0:#?})", event),
        mxc_types::MXCFB_WAIT_FOR_UPDATE_COMPLETE => handle_wait_update_complete(event),
        mxc_types::MXCFB_SEND_UPDATE => handle_send_update(event),
        _ => println!("unknown_ioctl({0:#?})", event),
    }
    return res;
  }
}
