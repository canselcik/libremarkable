#![feature(integer_atomics)]
#![feature(const_size_of)]

#[macro_use]
extern crate ioctl_gen;

extern crate libc;
extern crate mmap;
extern crate image;
extern crate rusttype;

mod mxc_types;
mod fb;
mod refresh;

macro_rules! min {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => (::std::cmp::min($x, min!($($z),*)));
}

macro_rules! max {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => (::std::cmp::max($x, max!($($z),*)));
}



use image::GenericImage;
use mxc_types::{display_temp, waveform_mode, update_mode, dither_mode};

#[allow(dead_code)]
#[allow(deprecated)]
fn main() {
    let mut framebuffer = fb::Framebuffer::new("/dev/fb0");
    framebuffer.var_screen_info.xres = 1872;
    framebuffer.var_screen_info.yres = 1404;
    framebuffer.var_screen_info.rotate = 1;
    framebuffer.var_screen_info.width = framebuffer.var_screen_info.xres;
    framebuffer.var_screen_info.height = framebuffer.var_screen_info.yres;
    framebuffer.var_screen_info.pixclock = 160000000;
    framebuffer.var_screen_info.left_margin = 32;
    framebuffer.var_screen_info.right_margin = 326;
    framebuffer.var_screen_info.upper_margin = 4;
    framebuffer.var_screen_info.lower_margin = 12;
    framebuffer.var_screen_info.hsync_len = 44;
    framebuffer.var_screen_info.vsync_len = 1;
    framebuffer.var_screen_info.sync = 0;
    framebuffer.var_screen_info.vmode = 0; // FB_VMODE_NONINTERLACED
    framebuffer.var_screen_info.accel_flags = 0;
    if !framebuffer.put_var_screeninfo() {
        panic!("FBIOPUT_VSCREENINFO failed");
    }

    let imgx = framebuffer.var_screen_info.xres;
    let imgy = framebuffer.var_screen_info.yres;

    framebuffer.clear();
    framebuffer.refresh(
        0,
        0,
        imgy as usize,
        imgx as usize,
        update_mode::UPDATE_MODE_FULL,
        waveform_mode::WAVEFORM_MODE_INIT,
        display_temp::TEMP_USE_AMBIENT,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );

    let img = image::open("/home/root/lowbattery.bmp")
        .unwrap()
        .rotate270();
    framebuffer.draw_image(&img, 1050, 220);
    let mut marker = framebuffer.refresh(
        1050,
        220,
        img.height() as usize,
        img.width() as usize,
        update_mode::UPDATE_MODE_PARTIAL,
        waveform_mode::WAVEFORM_MODE_GC16_FAST,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );
    framebuffer.wait_refresh_complete(marker);

    let draw_area: mxc_types::mxcfb_rect = framebuffer.draw_text(
        120,
        120,
        "TestinGG".to_owned(),
        120,
        mxc_types::REMARKABLE_DARKEST,
    );
    marker = framebuffer.refresh(
        draw_area.top as usize,
        draw_area.left as usize,
        draw_area.height as usize,
        draw_area.width as usize,
        update_mode::UPDATE_MODE_PARTIAL,
        waveform_mode::WAVEFORM_MODE_GC16_FAST,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );
    framebuffer.wait_refresh_complete(marker);

    let mut x: i32 = 10;
    let mut y: i32 = 10;
    let mut old_x: i32 = -1;
    let mut old_y: i32 = -1;
    loop {
        if old_y > 0 && old_x > 0 {
            framebuffer.draw_rect(
                old_y as usize,
                old_x as usize,
                50,
                50,
                mxc_types::REMARKABLE_BRIGHTEST,
            );
        }
        framebuffer.draw_rect(
            y as usize,
            x as usize,
            50,
            50,
            mxc_types::REMARKABLE_DARKEST,
        );
        marker = framebuffer.refresh(
            min!(y, old_y) as usize,
            min!(x, old_x) as usize,
            (max!(y, old_y) - min!(y, old_y) + 50) as usize,
            (max!(x, old_x) - min!(x, old_x) + 50) as usize,
            update_mode::UPDATE_MODE_PARTIAL,
            waveform_mode::WAVEFORM_MODE_DU,
            display_temp::TEMP_USE_REMARKABLE_DRAW,
            dither_mode::EPDC_FLAG_USE_DITHERING_Y1,
            0,
            0,
        );
        framebuffer.wait_refresh_complete(marker);

        old_x = x;
        old_y = y;
        y += 6;
        x += 6;
    }

}
