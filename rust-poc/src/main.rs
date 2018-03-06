#![feature(const_ptr_null_mut)]

extern crate librustpad;
extern crate image;
extern crate libc;
extern crate evdev;
extern crate chrono;

use chrono::{Local, DateTime};

use std::option::Option;
use std::time::Duration;
use std::thread::sleep;

use librustpad::fb;
use librustpad::mxc_types;
use librustpad::physical_buttons;
use mxc_types::{display_temp, waveform_mode, update_mode, dither_mode};


fn clear(ptr: *mut fb::Framebuffer) {
    let framebuffer = unsafe { &mut *ptr as &mut fb::Framebuffer };

    let yres = framebuffer.var_screen_info.yres as usize;
    let xres = framebuffer.var_screen_info.xres as usize;
    framebuffer.clear();
    framebuffer.refresh(
        0,
        0,
        yres,
        xres,
        update_mode::UPDATE_MODE_FULL,
        waveform_mode::WAVEFORM_MODE_INIT,
        display_temp::TEMP_USE_AMBIENT,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );
    std::thread::sleep(Duration::from_millis(100));
}

fn display_text(
    ptr: *mut fb::Framebuffer,
    y: usize,
    x: usize,
    scale: usize,
    text: String,
    wait_refresh: bool,
) {
    let framebuffer = unsafe { &mut *ptr as &mut fb::Framebuffer };

    let draw_area: mxc_types::mxcfb_rect =
        framebuffer.draw_text(y, x, text, scale, mxc_types::REMARKABLE_DARKEST);
    let marker = framebuffer.refresh_region(
        draw_area,
        update_mode::UPDATE_MODE_PARTIAL,
        waveform_mode::WAVEFORM_MODE_GC16_FAST,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );
    if !wait_refresh {
        framebuffer.wait_refresh_complete(marker);
    }
}


fn loop_print_time(ptr: *mut fb::Framebuffer, y: usize, x: usize, scale: usize) {
    let framebuffer = unsafe { &mut *ptr as &mut fb::Framebuffer };

    let mut draw_area: Option<mxc_types::mxcfb_rect> = None;
    loop {
        let dt: DateTime<Local> = Local::now();
        match draw_area {
            Some(area) => {
                framebuffer.fill_rect(
                    area.top as usize,
                    area.left as usize,
                    area.height as usize,
                    area.width as usize,
                    mxc_types::REMARKABLE_BRIGHTEST,
                )
            }
            _ => {} 
        }

        draw_area = Some(framebuffer.draw_text(
            y,
            x,
            format!("{}", dt.format("%F %r")),
            scale,
            mxc_types::REMARKABLE_DARKEST,
        ));
        match draw_area {
            Some(area) => {
                let marker = framebuffer.refresh_region(
                    area,
                    update_mode::UPDATE_MODE_PARTIAL,
                    waveform_mode::WAVEFORM_MODE_DU,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_USE_DITHERING_Y1,
                    0,
                    0,
                );
                framebuffer.wait_refresh_complete(marker);
            }
            _ => {}
        }
        sleep(Duration::from_millis(400));
    }
}

fn show_image(ptr: *mut fb::Framebuffer, img: &image::DynamicImage, y: usize, x: usize) {
    let framebuffer = unsafe { &mut *ptr as &mut fb::Framebuffer };

    let rect = framebuffer.draw_image(&img, y, x);
    let marker = framebuffer.refresh_region(rect,
									        update_mode::UPDATE_MODE_PARTIAL,
									        waveform_mode::WAVEFORM_MODE_GC16_FAST,
									        display_temp::TEMP_USE_REMARKABLE_DRAW,
									        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
									        0, 0);
    framebuffer.wait_refresh_complete(marker);
}

fn on_touch(_gesture_seq: u16, _finger_id: u16, y: u16, x: u16) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };

    let rect = framebuffer.draw_circle(y as usize, x as usize, 20, mxc_types::REMARKABLE_DARKEST);
    framebuffer.refresh_region(
        rect,
        update_mode::UPDATE_MODE_PARTIAL,
        waveform_mode::WAVEFORM_MODE_DU,
        display_temp::TEMP_USE_PAPYRUS,
        dither_mode::EPDC_FLAG_USE_DITHERING_DRAWING,
        mxc_types::DRAWING_QUANT_BIT,
        0,
    );
}

fn on_button_press(btn: physical_buttons::PhysicalButton, new_state: u16) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };

    let color = match new_state {
        0 => mxc_types::REMARKABLE_BRIGHTEST,
        _ => mxc_types::REMARKABLE_DARKEST,
    };
    let x_offset = match btn {
        physical_buttons::PhysicalButton::LEFT => 50,
        physical_buttons::PhysicalButton::MIDDLE => 640,
        physical_buttons::PhysicalButton::RIGHT => 1250,
    };

    framebuffer.fill_rect(1500, x_offset, 125, 125, color);
    framebuffer.refresh(
        1500,
        x_offset,
        125,
        125,
        update_mode::UPDATE_MODE_PARTIAL,
        waveform_mode::WAVEFORM_MODE_DU,
        display_temp::TEMP_USE_PAPYRUS,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );
}

static mut G_FRAMEBUFFER: *mut fb::Framebuffer = std::ptr::null_mut::<fb::Framebuffer>();
fn main() {
    let mut fbuffer = fb::Framebuffer::new("/dev/fb0");

    // TODO: Maybe actually try to reason with the borrow checker here
    let framebuffer = unsafe {
        G_FRAMEBUFFER = &mut fbuffer;
        G_FRAMEBUFFER
    };

    let img = image::load_from_memory(include_bytes!("../rustlang.bmp")).unwrap();

    clear(framebuffer);

    let clock_thread = std::thread::spawn(move || {
        let ptr = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };
        loop_print_time(ptr, 100, 100, 65);
    });

    display_text(
        framebuffer,
        200,
        100,
        100,
        "Remarkable Tablet".to_owned(),
        false,
    );
    show_image(framebuffer, &img, 10, 900);

    let hw_btn_demo_thread = std::thread::spawn(move || {
        librustpad::ev::start_evdev(
            "/dev/input/event2".to_owned(),
            physical_buttons::PhysicalButtonHandler::get_instance(on_button_press),
        );
    });
    let mt_demo_thread = std::thread::spawn(move || {
        librustpad::ev::start_evdev(
            "/dev/input/event1".to_owned(),
            librustpad::multitouch::MultitouchHandler::get_instance(on_touch),
        );
    });
    let wacom_demo_thread = std::thread::spawn(move || {
        librustpad::ev::start_evdev(
            "/dev/input/event0".to_owned(),
            librustpad::ev_debug::EvDeviceDebugHandler { name: "Wacom".to_owned() },
        );
    });

    clock_thread.join().unwrap();
    hw_btn_demo_thread.join().unwrap();
    wacom_demo_thread.join().unwrap();
    mt_demo_thread.join().unwrap();
}