#![allow(dead_code)]
#![allow(non_camel_case_types)]
use framebuffer::mxcfb::*;
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
pub const MXCFB_DISABLE_EPDC_ACCESS: NativeWidthType = io!(b'F', 0x35) as NativeWidthType;
pub const MXCFB_ENABLE_EPDC_ACCESS: NativeWidthType = io!(b'F', 0x36) as NativeWidthType;

pub const FBIOPUT_VSCREENINFO: NativeWidthType = 0x4601;
pub const FBIOGET_VSCREENINFO: NativeWidthType = 0x4600;
pub const FBIOGET_FSCREENINFO: NativeWidthType = 0x4602;
pub const FBIOGETCMAP: NativeWidthType = 0x4604;
pub const FBIOPUTCMAP: NativeWidthType = 0x4605;
pub const FBIOPAN_DISPLAY: NativeWidthType = 0x4606;
pub const FBIO_CURSOR: NativeWidthType = 0x4608;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum color {
    BLACK,
    RED,
    GREEN,
    BLUE,
    WHITE,
    NATIVE_COMPONENTS(u8, u8),
    RGB(u8, u8, u8),
    GRAY(u8),
}

impl color {
    pub fn from_native(c: [u8; 2]) -> color {
        color::NATIVE_COMPONENTS(c[0], c[1])
    }

    pub fn to_rgb565(self) -> [u8; 2] {
        self.as_native()
    }

    pub fn to_rgb8(self) -> [u8; 3] {
        // Components reversed because of the device
        let components = self.as_native();

        let mut combined: u16 = u16::from(components[1]) << 8;
        combined |= u16::from(components[0]);

        let red = (((combined & 0b1111_1000_0000_0000) >> 11) << 3) as u8;
        let green = (((combined & 0b0000_0111_1110_0000) >> 5) << 2) as u8;
        let blue = ((combined & 0b0000_0000_0001_1111) << 3) as u8;

        [red, green, blue]
    }

    pub fn as_native(self) -> [u8; 2] {
        match self {
            color::BLACK => [0x00, 0x00],
            color::RED => [0x07, 0xE0],
            color::GREEN => [0x00, 0x1F],
            color::BLUE => [0xF8, 0x00],
            color::WHITE => [0xFF, 0xFF],
            color::GRAY(level) => color::RGB(255 - level, 255 - level, 255 - level).as_native(),
            color::NATIVE_COMPONENTS(c1, c2) => [c1, c2],
            color::RGB(r8, g8, b8) => {
                // Simply can be referred to as `rgb565_le`
                //
                //    red     : offset = 11,  length =5,      msb_right = 0
                //    green   : offset = 5,   length =6,      msb_right = 0
                //    blue    : offset = 0,   length =5,      msb_right = 0
                //
                let r5 = (u16::from(r8) >> 3) as u8;
                let g6 = (u16::from(g8) >> 2) as u8;
                let b5 = (u16::from(b8) >> 3) as u8;

                [
                    (((g6 & 0b00_0111) << 5) | b5),
                    ((r5 << 3) | ((g6 & 0b11_1000) >> 3)),
                ]
            }
        }
    }
}

impl ::std::default::Default for color {
    fn default() -> Self {
        color::WHITE
    }
}

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
pub const DRAWING_QUANT_BIT: i32 = 0x7614_3b24;
pub const DRAWING_QUANT_BIT_2: i32 = 0x75e7_bb24;
pub const DRAWING_QUANT_BIT_3: i32 = 0x5_3ed4;

#[derive(Copy, Clone, Debug, PartialEq)]
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
    pub fn invalid() -> Self {
        mxcfb_rect {
            top: 9999,
            left: 9999,
            height: 0,
            width: 0,
        }
    }
}

impl mxcfb_rect {
    pub fn contains_point(&self, y: u32, x: u32) -> bool {
        !(x < self.left
            || x > (self.left + self.width)
            || y < self.top
            || y > (self.top + self.height))
    }

    pub fn contains_rect(&self, rect: &mxcfb_rect) -> bool {
        self.contains_point(rect.top, rect.left)
            && self.contains_point(rect.top + rect.height, rect.left + rect.width)
    }
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

#[derive(Debug)]
pub enum dither_mode {
    EPDC_FLAG_USE_DITHERING_PASSTHROUGH = 0x0,
    EPDC_FLAG_USE_DITHERING_DRAWING = 0x1,
    /// Dithering Processing (Version 1.0 - for i.MX508 and i.MX6SL)
    EPDC_FLAG_USE_DITHERING_Y1 = 0x00_2000,
    EPDC_FLAG_USE_REMARKABLE_DITHER = 0x30_0f30,
    EPDC_FLAG_USE_DITHERING_Y4 = 0x00_4000,
    EPDC_FLAG_USE_DITHERING_ALPHA = 0x3ff0_0000,
    EPDC_FLAG_USE_DITHERING_BETA = 0x7546_1440,
    EPDC_FLAG_EXP1 = 0x270_ce20,
    EPDC_FLAG_EXP2 = 0x270_db98,
    EPDC_FLAG_EXP3 = 0x274_45a0,
    EPDC_FLAG_EXP4 = 0x274_6f68,
    EPDC_FLAG_EXP5 = 0x274_aa58,
    EPDC_FLAG_EXP6 = 0x274_bd40,
    EPDC_FLAG_EXP7 = 0x7ecf_22c0,
    EPDC_FLAG_EXP8 = 0x7ed3_d2c0,
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
    /// remarkable uses this for drawing. it is impossible to draw an RGB pixel with this.
    /// it is for DIRECT UPDATE transitions only. Use GC16_* for colored updates.
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
    /// Seems to have the best draw latency. Perhaps the rule of thumb here is the lower the faster.
    /// `xochitl` seems to use this value.
    TEMP_USE_REMARKABLE_DRAW = 0x0018,
    /// For some odd reason, using this display temp will yield higher draw latency
    TEMP_USE_AMBIENT = 0x1000,
    /// This also has high draw latency
    TEMP_USE_PAPYRUS = 0x1001,
    /// High draw latency again
    TEMP_USE_MAX = 0xFFFF,
}
