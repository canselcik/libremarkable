extern crate libc;
use libc::{c_int,intptr_t};

#[macro_use]
extern crate redhook;

#[derive(Debug)]
#[repr(C)]
struct mxcfb_rect {
  top: u32,
  left: u32,
  width: u32,
  height: u32,
}

#[derive(Debug)]
#[repr(C)]
struct mxcfb_update_marker_data {
	update_marker: u32,
	collision_test: u32,
}

#[derive(Debug)]
#[repr(C)]
struct mxcfb_alt_buffer_data {
	phys_addr: u32,
	width: u32,
	height: u32,
	alt_update_region: mxcfb_rect,
}

#[derive(Debug)]
#[repr(C)]
struct mxcfb_update_data {
	update_region: mxcfb_rect,
  waveform_mode: u32,
  update_mode: u32,
  update_marker: u32,
  temp: i32,
  flags: u32,
  dither_mode: i32,
	quant_bit: i32,
  alt_buffer_data: mxcfb_alt_buffer_data,
}

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

#[derive(Debug)]
#[repr(C)]
pub struct fb_bitfield {
	pub offset: u32,    /* beginning of bitfield	*/
	pub length: u32,    /* length of bitfield		*/
	pub msb_right: u32, /* != 0 : Most significant bit is right */
}

#[derive(Debug)]
#[repr(C)]
pub struct fb_var_screeninfo {
    pub xres: u32, /* visible resolution	*/
	  pub yres: u32,
	  pub xres_virtual: u32,	/* virtual resolution	*/
	  pub yres_virtual: u32,
	  pub xoffset: u32, /* offset from virtual to visible */
	  pub yoffset: u32, /* resolution */

	  pub bits_per_pixel: u32,	/* guess what */
	  pub grayscale: u32, /* 0 = color, 1 = grayscale,  >1 = FOURCC */
	  pub red: fb_bitfield, /* bitfield in fb mem if true color, */
	  pub green: fb_bitfield,	/* else only length is significant */
	  pub blue: fb_bitfield,
	  pub transp: fb_bitfield, /* transparency */

	  pub nonstd: u32, /* != 0 Non standard pixel format */
	  pub activate: u32, /* see FB_ACTIVATE_* */

	  pub height: u32, /* height of picture in mm */
	  pub width: u32, /* width of picture in mm */

	  pub accel_flags: u32, /* (OBSOLETE) see fb_info.flags */

	  /* Timing: All values in pixclocks, except pixclock (of course) */
	  pub pixclock: u32, /* pixel clock in ps (pico seconds) */
	  pub left_margin: u32, /* time from sync to picture	*/
	  pub right_margin: u32, /* time from picture to sync	*/
	  pub upper_margin: u32, /* time from sync to picture	*/
	  pub lower_margin: u32,
	  pub hsync_len: u32, /* length of horizontal sync */
	  pub vsync_len: u32, /* length of vertical sync */
	  pub sync: u32, /* see FB_SYNC_* */
	  pub vmode: u32, /* see FB_VMODE_* */
	  pub rotate: u32, /* angle we rotate counter clockwise */
	  pub colorspace: u32, /* colorspace for FOURCC-based modes */
	  pub reserved: [u32; 4], /* Reserved for future compatibility */
}

const MXCFB_REMARKABLE_MASK: u32       = 0x40484600;
const FBIOPUT_VSCREENINFO: u32         = 0x4601;
const FBIOGET_VSCREENINFO: u32         = 0x4600;
const FBIOGET_FSCREENINFO: u32         = 0x4602;
const FBIOGETCMAP: u32                 = 0x4604;
const FBIOPUTCMAP: u32                 = 0x4605;
const FBIOPAN_DISPLAY: u32             = 0x4606;
const FBIO_CURSOR: u32                 = 0x4608;

#[derive(Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum mxcfb_ioctl {
  MXCFB_NONE                           = 0x00,
  MXCFB_SET_WAVEFORM_MODES	           = 0x2B,  // takes struct mxcfb_waveform_modes
  MXCFB_SET_TEMPERATURE		             = 0x2C,  // takes int32_t
  MXCFB_SET_AUTO_UPDATE_MODE           = 0x2D,  // takes __u32
  MXCFB_SEND_UPDATE                    = 0x2E,  // takes struct mxcfb_update_data
  MXCFB_WAIT_FOR_UPDATE_COMPLETE       = 0x2F,  // takes struct mxcfb_update_marker_data
  MXCFB_SET_PWRDOWN_DELAY              = 0x30,  // takes int32_t
  MXCFB_GET_PWRDOWN_DELAY              = 0x31,  // takes int32_t
  MXCFB_SET_UPDATE_SCHEME              = 0x32,  // takes __u32
  MXCFB_GET_WORK_BUFFER                = 0x34,  // takes unsigned long
  MXCFB_DISABLE_EPDC_ACCESS            = 0x35,
  MXCFB_ENABLE_EPDC_ACCESS             = 0x36,
}

fn handle_send_update(event: ioctl_intercept_event) {
  unsafe {
    let update_data = event.p1 as *mut mxcfb_update_data;
    println!("mxcfb_send_update(fd: {0}, updateData: {1:#?}) = {2}", event.fd, *update_data, event.ret);
  }
}

fn handle_wait_update_complete(event: ioctl_intercept_event) {
  unsafe {
    let update_data = event.p1 as *mut mxcfb_update_marker_data;
    println!("mxcfb_wait_update_complete(fd: {0}, updateData: {1:#?}) = {2}", event.fd, *update_data, event.ret);
  }
}

fn get_call_type(request: u32) -> mxcfb_ioctl {
  let lo = (request & 0x000000ff) as u8;
  if lo < mxcfb_ioctl::MXCFB_NONE as u8 || lo > mxcfb_ioctl::MXCFB_ENABLE_EPDC_ACCESS as u8 {
    return mxcfb_ioctl::MXCFB_NONE;
  }
  let transmuted: mxcfb_ioctl = unsafe { std::mem::transmute(lo) };
  return transmuted;
}

hook! {
  unsafe fn ioctl(fd: c_int, request: u32, p1: intptr_t, p2: intptr_t, p3: intptr_t, p4: intptr_t) -> c_int => ioctl_hook {
    if request == FBIOPUT_VSCREENINFO {
        let info = p1 as *mut fb_var_screeninfo;
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

    if ((request & 0xffffff00) as u32) != MXCFB_REMARKABLE_MASK {
      match request {
        FBIOGETCMAP => println!("FBIOGETCMAP({0:#?})", event),
        FBIOPUTCMAP => println!("FBIOPUTCMAP({0:#?})", event),
        FBIO_CURSOR => println!("FBIO_CURSOR({0:#?})", event),
        FBIOPAN_DISPLAY => println!("FBIOPAN_DISPLAY({0:#?})", event),
        FBIOPUT_VSCREENINFO => println!("FBIOPUT_VSCREENINFO(after: {0:#?})", p1 as *mut fb_var_screeninfo),
        FBIOGET_VSCREENINFO => println!("FBIOGET_VSCREENINFO(out: {0:#?})", p1 as *mut fb_var_screeninfo),
        FBIOGET_FSCREENINFO => println!("FBIOGET_FSCREENINFO(out: {0:#?})", event),
        _ => println!("unknown_ioctl({0:#?})", event),
      }
      return res;
    }

    match get_call_type(request) {
      mxcfb_ioctl::MXCFB_WAIT_FOR_UPDATE_COMPLETE => handle_wait_update_complete(event),
      mxcfb_ioctl::MXCFB_SEND_UPDATE => handle_send_update(event),
      _                              => println!("remarkable_unknown_ioctl({0:#?})", event),
    }
    return res;
  }
}
