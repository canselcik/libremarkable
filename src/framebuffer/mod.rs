pub mod common;
pub mod mxcfb;
pub mod screeninfo;


pub mod io;
pub trait FramebufferIO {
    fn write_frame(&mut self, frame: &[u8]);
    fn write_pixel(&mut self, y: usize, x: usize, v: u8);
    fn read_pixel(&mut self, y: usize, x: usize) -> u8;
    fn read_offset(&mut self, ofst: isize) -> u8;
}

use image;
pub mod draw;
pub trait FramebufferDraw {
    fn draw_image(&mut self, img: &image::DynamicImage, top: usize, left: usize) -> common::mxcfb_rect;
    fn draw_line(&mut self, y0: i32, x0: i32, y1: i32, x1: i32, width: usize, color: u8) -> common::mxcfb_rect;
    fn draw_circle(&mut self, y: usize, x: usize, rad: usize, color: u8) -> common::mxcfb_rect;
    fn fill_circle(&mut self, y: usize, x: usize, rad: usize, color: u8) -> common::mxcfb_rect;
    fn draw_bezier(&mut self, startpt: (f32, f32), ctrlpt: (f32, f32), endpt: (f32, f32), color: u8) -> common::mxcfb_rect;
    fn draw_text(
        &mut self,
        y: usize,
        x: usize,
        text: String,
        size: usize,
        color: u8,
    ) -> common::mxcfb_rect;
    fn fill_rect(&mut self, y: usize, x: usize, height: usize, width: usize, color: u8);
    fn clear(&mut self);
}

use std;
pub mod core;
pub trait FramebufferBase<'a> {
    fn new(path_to_device: &str) -> core::Framebuffer;
    fn set_epdc_access(&mut self, state: bool);
    fn set_autoupdate_mode(&mut self, mode: u32);
    fn set_update_scheme(&mut self, scheme: u32);
    fn get_fix_screeninfo(device: &std::fs::File) -> screeninfo::FixScreeninfo;
    fn get_var_screeninfo(device: &std::fs::File) -> screeninfo::VarScreeninfo;
    fn put_var_screeninfo(&mut self) -> bool;
}


pub mod refresh;
pub trait FramebufferRefresh {
    fn refresh(
        &mut self,
        region: &common::mxcfb_rect,
        update_mode: common::update_mode,
        waveform_mode: common::waveform_mode,
        temperature: common::display_temp,
        dither_mode: common::dither_mode,
        quant_bit: i32,
        flags: u32,
    ) -> u32;

    fn wait_refresh_complete(&mut self, marker: u32);
}