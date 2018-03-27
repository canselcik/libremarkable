#![allow(dead_code)]
#![allow(non_camel_case_types)]

use libc;
use libc::intptr_t;

use std;

/// This is to allow tests to run on systems with 64bit pointer types.
/// It doesn't make a difference since we will be mocking the ioctl calls.
#[cfg(target_pointer_width = "64")]
pub type NativeWidthType = u64;
#[cfg(target_pointer_width = "32")]
pub type NativeWidthType = u32;

pub const DISPLAYWIDTH: u16 = 1404;
pub const DISPLAYHEIGHT: u16 = 1872;

pub const MTWIDTH: u16 = 767;
pub const MTHEIGHT: u16 = 1023;

pub const WACOMWIDTH: u16 = 15725;
pub const WACOMHEIGHT: u16 = 20967;

pub const MXCFB_SET_AUTO_UPDATE_MODE: NativeWidthType =
    iow!(b'F', 0x2D, std::mem::size_of::<u32>()) as NativeWidthType;
pub const MXCFB_SET_UPDATE_SCHEME: NativeWidthType =
    iow!(b'F', 0x32, std::mem::size_of::<u32>()) as NativeWidthType;
pub const MXCFB_SEND_UPDATE: NativeWidthType =
    iow!(b'F', 0x2E, std::mem::size_of::<mxcfb_update_data>()) as NativeWidthType;
pub const MXCFB_WAIT_FOR_UPDATE_COMPLETE: NativeWidthType =
    iowr!(b'F', 0x2F, std::mem::size_of::<mxcfb_update_marker_data>()) as NativeWidthType;
pub const MXCFB_DISABLE_EPDC_ACCESS: NativeWidthType =
    io!(b'F', 0x35) as NativeWidthType;
pub const MXCFB_ENABLE_EPDC_ACCESS: NativeWidthType =
    io!(b'F', 0x36) as NativeWidthType;

pub const FBIOPUT_VSCREENINFO: NativeWidthType = 0x4601;
pub const FBIOGET_VSCREENINFO: NativeWidthType = 0x4600;
pub const FBIOGET_FSCREENINFO: NativeWidthType = 0x4602;
pub const FBIOGETCMAP: NativeWidthType = 0x4604;
pub const FBIOPUTCMAP: NativeWidthType = 0x4605;
pub const FBIOPAN_DISPLAY: NativeWidthType = 0x4606;
pub const FBIO_CURSOR: NativeWidthType = 0x4608;

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

impl mxcfb_rect {
    pub fn contains_point(&mut self, y: u32, x: u32) -> bool {
        x >= self.left && x < (self.left + self.width) &&
        y >= self.top && x < (self.top + self.height)
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
    pub fd: libc::c_int,
    pub request: u32,
    pub p1: intptr_t,
    pub p2: intptr_t,
    pub p3: intptr_t,
    pub p4: intptr_t,
    pub ret: libc::c_int,
}

impl ::std::default::Default for ioctl_intercept_event {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct fb_bitfield {
    /// beginning of bitfield
    pub offset: u32,

    /// length of bitfield
    pub length: u32,

    /// != 0 : Most significant bit is right
    pub msb_right: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct fb_var_screeninfo {
    /// visible resolution
    pub xres: u32,
    pub yres: u32,
    /// virtual resolution
    pub xres_virtual: u32,
    pub yres_virtual: u32,

    /// offset from virtual to visible
    pub xoffset: u32,
    pub yoffset: u32,

    /// resolution
    pub bits_per_pixel: u32,

    pub grayscale: u32,

    /// 0 = color, 1 = grayscale,  >1 = FOURCC
    /// bitfield in fb mem if true color,
    /// else only length is significant
    pub red: fb_bitfield,
    pub green: fb_bitfield,
    pub blue: fb_bitfield,
    pub transp: fb_bitfield,

    /// != 0 Non standard pixel format
    pub nonstd: u32,

    /// see FB_ACTIVATE_*
    pub activate: u32,

    /// height of picture in mm
    pub height: u32,

    /// width of picture in mm
    pub width: u32,

    /// (OBSOLETE) see fb_info.flags
    pub accel_flags: u32,


    /// Timing: All values in pixclocks, except pixclock (of course)
    /// pixel clock in ps (pico seconds)
    pub pixclock: u32,

    /// time from sync to picture
    pub left_margin: u32,

    /// time from picture to sync
    pub right_margin: u32,

    /// time from sync to picture
    pub upper_margin: u32,
    pub lower_margin: u32,

    /// length of horizontal sync
    pub hsync_len: u32,

    /// length of vertical sync
    pub vsync_len: u32,

    /// see FB_SYNC_*
    pub sync: u32,

    /// see FB_VMODE_*
    pub vmode: u32,

    /// angle we rotate counter clockwise
    pub rotate: u32,

    /// colorspace for FOURCC-based modes
    pub colorspace: u32,

    /// Reserved for future compatibility
    pub reserved: [u32; 4],
}


#[derive(Copy, Clone, Debug, PartialEq)]
pub enum mxcfb_ioctl {
    MXCFB_NONE = 0x00,
    MXCFB_SET_WAVEFORM_MODES = 0x2B,
    /// takes struct mxcfb_waveform_modes
    MXCFB_SET_TEMPERATURE = 0x2C,
    /// takes int32_t
    MXCFB_SET_AUTO_UPDATE_MODE = 0x2D,
    /// takes __u32
    MXCFB_SEND_UPDATE = 0x2E,
    /// takes struct mxcfb_update_data
    MXCFB_WAIT_FOR_UPDATE_COMPLETE = 0x2F,
    /// takes struct mxcfb_update_marker_data
    MXCFB_SET_PWRDOWN_DELAY = 0x30,
    /// takes int32_t
    MXCFB_GET_PWRDOWN_DELAY = 0x31,
    /// takes int32_t
    MXCFB_SET_UPDATE_SCHEME = 0x32,
    /// takes __u32
    MXCFB_GET_WORK_BUFFER = 0x34,
    /// takes unsigned long
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
    /// Returns a marker, no locking, no waiting on the
    /// clean state on the update region
    UPDATE_MODE_PARTIAL = 0,

    /// Waits for all other updates in the region and performs
    /// in an ordered fashion after them
    UPDATE_MODE_FULL = 1,
}


/// Further experimentation is needed here but here is the gist of it:
///
///    red     : offset = 11,  length =5,      msb_right = 0
///    green   : offset = 5,   length =6,      msb_right = 0
///    blue    : offset = 0,   length =5,      msb_right = 0
///
/// TO_REMARKABLE_COLOR(r, g, b) = ((r << 11) | (g << 5) | b)
pub const REMARKABLE_DARKEST: u8 = 0x00;
pub const REMARKABLE_BRIGHTEST: u8 = 0xFF;

///
/// If no processing required, skip update processing
///  No processing means:
///  - FB unrotated
///  - FB pixel format = 8-bit grayscale
///  - No look-up transformations (inversion, posterization, etc.)
///
/// Enables PXP_LUT_INVERT transform on the buffer
pub const EPDC_FLAG_ENABLE_INVERSION: u32 = 0x0001;

/// Enables PXP_LUT_BLACK_WHITE transform on the buffer
pub const EPDC_FLAG_FORCE_MONOCHROME: u32 = 0x0002;

/// Enables PXP_USE_CMAP transform on the buffer
pub const EPDC_FLAG_USE_CMAP: u32 = 0x0004;

/// This is basically double buffering. We give it the bitmap we want to
/// update, it swaps them. However the bitmap needs to fall within the smem.
pub const EPDC_FLAG_USE_ALT_BUFFER: u32 = 0x0100;

/// An update won't be merged upon a conflict in case of a collusion if
/// either update has this flag set, unless they are identical regions (same y,x,h,w)
pub const EPDC_FLAG_TEST_COLLISION: u32 = 0x0200;
pub const EPDC_FLAG_GROUP_UPDATE: u32 = 0x0400;


/// xochitl tends to draw with these but there are many more
pub const DRAWING_QUANT_BIT: i32 = 0x76143b24;
pub const DRAWING_QUANT_BIT_2 : i32 = 0x75e7bb24;
pub const DRAWING_QUANT_BIT_3 : i32 = 0x53ed4;

#[derive(Debug)]
pub enum dither_mode {
    EPDC_FLAG_USE_DITHERING_PASSTHROUGH = 0x0,
    EPDC_FLAG_USE_DITHERING_DRAWING = 0x1,
    /// Dithering Processing (Version 1.0 - for i.MX508 and i.MX6SL)
    EPDC_FLAG_USE_DITHERING_Y1 = 0x002000,
    EPDC_FLAG_USE_REMARKABLE_DITHER = 0x300f30,
    EPDC_FLAG_USE_DITHERING_Y4 = 0x004000,
    EPDC_FLAG_USE_DITHERING_ALPHA = 0x3ff00000,
    EPDC_FLAG_USE_DITHERING_BETA = 0x75461440,
    EPDC_FLAG_EXP1 = 0x270ce20,
    EPDC_FLAG_EXP2 = 0x270db98,
    EPDC_FLAG_EXP3 = 0x27445a0,
    EPDC_FLAG_EXP4 = 0x2746f68,
    EPDC_FLAG_EXP5 = 0x274aa58,
    EPDC_FLAG_EXP6 = 0x274bd40,
    EPDC_FLAG_EXP7 = 0x7ecf22c0,
    EPDC_FLAG_EXP8 = 0x7ed3d2c0,
}

#[derive(Debug)]
pub enum waveform_mode {
    /// (Recommended) Screen goes to white
    /// (flashes black/white once to clear ghosting when used with UPDATE_MODE_FULL)
    WAVEFORM_MODE_INIT = 0x0,

    /// (Recommended) Basically A2 according to documentation found from various sources, therefore
    /// partial refresh shouldn't be possible here however it is and really good
    /// for quick black->white transition with some leftovers behind
    WAVEFORM_MODE_GLR16 = 0x4,

    /// (Further exploration needed) Enables Regal D Processing, also observed being used
    WAVEFORM_MODE_GLD16 = 0x5,

    /// (Recommended) "Direct Update" Grey->white/grey->black
    /// remarkable uses this for drawing
    WAVEFORM_MODE_DU = 0x1,

    /// (Recommended) High fidelity (flashes black/white when used with UPDATE_MODE_FULL)
    /// also called WAVEFORM_MODE_GC4
    WAVEFORM_MODE_GC16 = 0x2,

    /// (Recommended) Medium fidelity -- remarkable uses this for UI
    WAVEFORM_MODE_GC16_FAST = 0x3,

    /// (Further exploration needed) Medium fidelity from white transition
    WAVEFORM_MODE_GL16_FAST = 0x6,

    /// (Further exploration needed) Medium fidelity 4 level of gray direct update
    WAVEFORM_MODE_DU4 = 0x7,

    /// (Further exploration needed) Ghost compensation waveform
    WAVEFORM_MODE_REAGL = 0x8,

    /// (Further exploration needed) Ghost compensation waveform with dithering
    WAVEFORM_MODE_REAGLD = 0x9,

    /// (Further exploration needed) 2-bit from white transition
    /// (odd fade-out effect that eventually settles at semi-sketched)
    WAVEFORM_MODE_GL4 = 0xA,

    /// (Further exploration needed) High fidelity for black
    /// transition (similar experience to GL4)
    WAVEFORM_MODE_GL16_INV = 0xB,

    /// (Recommended) The mechanism behind its selection isn't well
    /// understood however it is supported.
    WAVEFORM_MODE_AUTO = 257,
}

#[derive(Debug)]
pub enum display_temp {
    TEMP_USE_AMBIENT = 0x1000,
    TEMP_USE_PAPYRUS = 0x1001,
    TEMP_USE_REMARKABLE_DRAW = 0x0018,
    TEMP_USE_MAX = 0xFFFF,
}
