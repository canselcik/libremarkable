#![allow(dead_code)]
#![allow(non_camel_case_types)]
use libc::{intptr_t, c_int};
use std;


pub const DISPLAYWIDTH: u16 = 1404;
pub const DISPLAYHEIGHT: u16 = 1872;

pub const MTWIDTH: u16 = 767;
pub const MTHEIGHT: u16 = 1023;

pub const WACOMWIDTH: u16 = 15725;
pub const WACOMHEIGHT: u16 = 20967;

pub const MXCFB_SET_AUTO_UPDATE_MODE: u32 = iow!(b'F', 0x2D, std::mem::size_of::<u32>());
pub const MXCFB_SET_UPDATE_SCHEME: u32 = iow!(b'F', 0x32, std::mem::size_of::<u32>());
pub const MXCFB_SEND_UPDATE: u32 = iow!(b'F', 0x2E, std::mem::size_of::<mxcfb_update_data>());
pub const MXCFB_WAIT_FOR_UPDATE_COMPLETE: u32 =
    iowr!(b'F', 0x2F, std::mem::size_of::<mxcfb_update_marker_data>());
pub const FBIOPUT_VSCREENINFO: u32 = 0x4601;
pub const FBIOGET_VSCREENINFO: u32 = 0x4600;
pub const FBIOGET_FSCREENINFO: u32 = 0x4602;
pub const FBIOGETCMAP: u32 = 0x4604;
pub const FBIOPUTCMAP: u32 = 0x4605;
pub const FBIOPAN_DISPLAY: u32 = 0x4606;
pub const FBIO_CURSOR: u32 = 0x4608;

///Bitfield which is a part of VarScreeninfo.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Bitfield {
    pub offset: u32,
    pub length: u32,
    pub msb_right: u32,
}

///Struct as defined in /usr/include/linux/fb.h
#[repr(C)]
#[derive(Clone, Debug)]
pub struct VarScreeninfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: Bitfield,
    pub green: Bitfield,
    pub blue: Bitfield,
    pub transp: Bitfield,
    pub nonstd: u32,
    pub activate: u32,
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}


///Struct as defined in /usr/include/linux/fb.h Note: type is a keyword in Rust and therefore has been
///changed to fb_type.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct FixScreeninfo {
    pub id: [u8; 16],
    pub smem_start: usize,
    pub smem_len: u32,
    pub fb_type: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: usize,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

impl ::std::default::Default for Bitfield {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl ::std::default::Default for VarScreeninfo {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl ::std::default::Default for FixScreeninfo {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct mxcfb_rect {
    pub top: u32,
    pub left: u32,
    pub width: u32,
    pub height: u32,
}

impl ::std::default::Default for mxcfb_rect {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct mxcfb_update_marker_data {
    pub update_marker: u32,
    pub collision_test: u32,
}

impl ::std::default::Default for mxcfb_update_marker_data {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}


#[derive(Debug)]
#[repr(C)]
pub struct mxcfb_alt_buffer_data {
    pub phys_addr: u32,
    pub width: u32,
    pub height: u32,
    pub alt_update_region: mxcfb_rect,
}

impl ::std::default::Default for mxcfb_alt_buffer_data {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct mxcfb_update_data {
    pub update_region: mxcfb_rect,
    pub waveform_mode: u32,
    pub update_mode: u32,
    pub update_marker: u32,
    pub temp: i32,
    pub flags: u32,
    pub dither_mode: i32,
    pub quant_bit: i32,
    pub alt_buffer_data: mxcfb_alt_buffer_data,
}

impl ::std::default::Default for mxcfb_update_data {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ioctl_intercept_event {
    pub fd: c_int,
    pub request: u32,
    pub p1: intptr_t,
    pub p2: intptr_t,
    pub p3: intptr_t,
    pub p4: intptr_t,
    pub ret: c_int,
}

impl ::std::default::Default for ioctl_intercept_event {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct fb_bitfield {
    pub offset: u32, /* beginning of bitfield	*/
    pub length: u32, /* length of bitfield		*/
    pub msb_right: u32, /* != 0 : Most significant bit is right */
}

#[derive(Debug)]
#[repr(C)]
pub struct fb_var_screeninfo {
    pub xres: u32, /* visible resolution	*/
    pub yres: u32,
    pub xres_virtual: u32, /* virtual resolution	*/
    pub yres_virtual: u32,
    pub xoffset: u32, /* offset from virtual to visible */
    pub yoffset: u32, /* resolution */

    pub bits_per_pixel: u32, /* guess what */
    pub grayscale: u32, /* 0 = color, 1 = grayscale,  >1 = FOURCC */
    pub red: fb_bitfield, /* bitfield in fb mem if true color, */
    pub green: fb_bitfield, /* else only length is significant */
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



#[derive(Copy, Clone, Debug, PartialEq)]
pub enum mxcfb_ioctl {
    MXCFB_NONE = 0x00,
    MXCFB_SET_WAVEFORM_MODES = 0x2B, // takes struct mxcfb_waveform_modes
    MXCFB_SET_TEMPERATURE = 0x2C, // takes int32_t
    MXCFB_SET_AUTO_UPDATE_MODE = 0x2D, // takes __u32
    MXCFB_SEND_UPDATE = 0x2E, // takes struct mxcfb_update_data
    MXCFB_WAIT_FOR_UPDATE_COMPLETE = 0x2F, // takes struct mxcfb_update_marker_data
    MXCFB_SET_PWRDOWN_DELAY = 0x30, // takes int32_t
    MXCFB_GET_PWRDOWN_DELAY = 0x31, // takes int32_t
    MXCFB_SET_UPDATE_SCHEME = 0x32, // takes __u32
    MXCFB_GET_WORK_BUFFER = 0x34, // takes unsigned long
    MXCFB_DISABLE_EPDC_ACCESS = 0x35,
    MXCFB_ENABLE_EPDC_ACCESS = 0x36,
}

#[derive(Debug)]
pub enum auto_update_mode {
    AUTO_UPDATE_MODE_REGION_MODE = 0,
    AUTO_UPDATE_MODE_AUTOMATIC_MODE = 1,
}

#[derive(Debug)]
pub enum update_scheme {
    UPDATE_SCHEME_SNAPSHOT = 0,
    UPDATE_SCHEME_QUEUE = 1,
    UPDATE_SCHEME_QUEUE_AND_MERGE = 2,
}

#[derive(Debug)]
pub enum update_mode {
    UPDATE_MODE_PARTIAL = 0,
    UPDATE_MODE_FULL = 1,
}

// red     : offset = 11,  length =5,      msb_right = 0
// green   : offset = 5,   length =6,      msb_right = 0
// blue    : offset = 0,   length =5,      msb_right = 0
// typedef uint8_t remarkable_color;
// #define TO_REMARKABLE_COLOR(r, g, b)               ((r << 11) | (g << 5) | b)
pub const REMARKABLE_DARKEST: u8 = 0x00;
pub const REMARKABLE_BRIGHTEST: u8 = 0xFF;

// //// FLAGS
/*
* If no processing required, skip update processing
* No processing means:
*   - FB unrotated
*   - FB pixel format = 8-bit grayscale
*   - No look-up transformations (inversion, posterization, etc.)
*/
// Enables PXP_LUT_INVERT transform on the buffer
pub const EPDC_FLAG_ENABLE_INVERSION: u32 = 0x0001;
// Enables PXP_LUT_BLACK_WHITE transform on the buffer
pub const EPDC_FLAG_FORCE_MONOCHROME: u32 = 0x0002;
// Enables PXP_USE_CMAP transform on the buffer
pub const EPDC_FLAG_USE_CMAP: u32 = 0x0004;

// This is basically double buffering. We give it the bitmap we want to
// update, it swaps them.
pub const EPDC_FLAG_USE_ALT_BUFFER: u32 = 0x0100;

// An update won't be merged upon a conflict in case of a collusion if
// either update has this flag set, unless they are identical regions (same y,x,h,w)
pub const EPDC_FLAG_TEST_COLLISION: u32 = 0x0200;
pub const EPDC_FLAG_GROUP_UPDATE: u32 = 0x0400;


// xochitl tends to draw with this
pub const DRAWING_QUANT_BIT: i32 = 0x76143b24;

#[derive(Debug)]
pub enum dither_mode {
    EPDC_FLAG_USE_DITHERING_PASSTHROUGH = 0x0,
    EPDC_FLAG_USE_DITHERING_DRAWING = 0x1,
    // Dithering Processing (Version 1.0 - for i.MX508 and i.MX6SL)
    EPDC_FLAG_USE_DITHERING_Y1 = 0x002000,
    EPDC_FLAG_USE_REMARKABLE_DITHER = 0x300f30,
    EPDC_FLAG_USE_DITHERING_Y4 = 0x004000,
}

#[derive(Debug)]
pub enum waveform_mode {
    WAVEFORM_MODE_INIT = 0x0, /* Screen goes to white (clears) */
    WAVEFORM_MODE_GLR16 = 0x4, /* Basically A2 (so partial refresh shouldnt be possible here) */
    WAVEFORM_MODE_GLD16 = 0x5, /* Official -- and enables Regal D Processing */

    // Unsupported?
    WAVEFORM_MODE_DU = 0x1, /* [Direct Update] Grey->white/grey->black  -- remarkable uses this for drawing */
    WAVEFORM_MODE_GC16 = 0x2, /* High fidelity (flashing) */
    //  WAVEFORM_MODE_GC4          = WAVEFORM_MODE_GC16,   /* For compatibility */
    WAVEFORM_MODE_GC16_FAST = 0x3, /* Medium fidelity  -- remarkable uses this for UI */
    WAVEFORM_MODE_GL16_FAST = 0x6, /* Medium fidelity from white transition */
    WAVEFORM_MODE_DU4 = 0x7, /* Medium fidelity 4 level of gray direct update */
    WAVEFORM_MODE_REAGL = 0x8, /* Ghost compensation waveform */
    WAVEFORM_MODE_REAGLD = 0x9, /* Ghost compensation waveform with dithering */
    WAVEFORM_MODE_GL4 = 0xA, /* 2-bit from white transition */
    WAVEFORM_MODE_GL16_INV = 0xB, /* High fidelity for black transition */
    WAVEFORM_MODE_AUTO = 257, /* Official */
}

#[derive(Debug)]
pub enum display_temp {
    TEMP_USE_AMBIENT = 0x1000,
    TEMP_USE_PAPYRUS = 0x1001,
    TEMP_USE_REMARKABLE_DRAW = 0x0018,
    TEMP_USE_MAX = 0xFFFF,
}
