#![allow(non_camel_case_types)]

use libc;
use libc::intptr_t;

use framebuffer::common::mxcfb_rect;

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
