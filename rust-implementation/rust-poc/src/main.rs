#![feature(const_ptr_null_mut)]
#![feature(box_leak)]


extern crate librustpad;
extern crate chrono;

use chrono::{Local, DateTime};

use std::option::Option;
use std::time::Duration;
use std::thread::sleep;

use librustpad::rb::*;
use librustpad::image;
use librustpad::fb;
use librustpad::mxc_types;
use librustpad::unifiedinput;

use mxc_types::{display_temp, waveform_mode, update_mode, dither_mode};


fn clear(quick: bool) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };

    let (yres, xres) = (
        framebuffer.var_screen_info.yres,
        framebuffer.var_screen_info.xres,
    );
    framebuffer.clear();

    let (update_mode, waveform_mode) = match quick {
        true  => (update_mode::UPDATE_MODE_PARTIAL, waveform_mode::WAVEFORM_MODE_GC16_FAST),
        false => (update_mode::UPDATE_MODE_FULL, waveform_mode::WAVEFORM_MODE_INIT),
    };
    let marker = framebuffer.refresh(
        mxc_types::mxcfb_rect {
            top: 0,
            left: 0,
            height: yres,
            width: xres,
        },
        update_mode,
        waveform_mode,
        display_temp::TEMP_USE_AMBIENT,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0, 0,
    );
    match quick {
        true => framebuffer.wait_refresh_complete(marker),
        false => std::thread::sleep(Duration::from_millis(150)),
    }
}

fn display_text(
    y: usize,
    x: usize,
    scale: usize,
    text: String,
    wait_refresh: bool,
) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };

    let draw_area: mxc_types::mxcfb_rect =
        framebuffer.draw_text(y, x, text, scale, mxc_types::REMARKABLE_DARKEST);
    let marker = framebuffer.refresh(
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


fn loop_print_time(y: usize, x: usize, scale: usize) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };

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
                let marker = framebuffer.refresh(
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

fn show_image(img: &image::DynamicImage, y: usize, x: usize) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };

    let rect = framebuffer.draw_image(&img, y, x);
    let marker = framebuffer.refresh(
        rect,
        update_mode::UPDATE_MODE_PARTIAL,
        waveform_mode::WAVEFORM_MODE_GC16_FAST,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );
    framebuffer.wait_refresh_complete(marker);
}

#[allow(unused_variables)]
fn on_wacom_input(input: unifiedinput::WacomEvent) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };
    match input {
        unifiedinput::WacomEvent::Draw{y, x, pressure, tilt_x, tilt_y} => {
            let rad = 8. * (pressure as f32) / 4096.;

            let rect = framebuffer.fill_circle(
                y as usize, x as usize,
                rad as usize, mxc_types::REMARKABLE_DARKEST);

            framebuffer.refresh(
                rect,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_DU,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_DRAWING,
                mxc_types::DRAWING_QUANT_BIT,
                0,
            );
        },
        unifiedinput::WacomEvent::InstrumentChange{pen, state} => {
            // println!("WacomInstrumentChanged(inst: {0}, state: {1})", pen as u16, state);
        },
        unifiedinput::WacomEvent::Hover{y, x, distance, tilt_x, tilt_y} => {
            // println!("WacomHover(y: {0}, x: {1}, distance: {2})", y, x, distance);
        },
        _ => {},
    };
}

#[allow(unused_variables)]
fn on_touch(input: unifiedinput::MultitouchEvent) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };
    match input {
        unifiedinput::MultitouchEvent::Touch{gesture_seq, finger_id, y, x} => {
            let rect = framebuffer.draw_circle(y as usize, x as usize, 20, mxc_types::REMARKABLE_DARKEST);
            framebuffer.refresh(
                rect,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_DU,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_DRAWING,
                mxc_types::DRAWING_QUANT_BIT,
                0,
            );
        },
        _ => {}
    }
}

fn on_button_press(input: unifiedinput::GPIOEvent) {
    let framebuffer = unsafe { &mut *G_FRAMEBUFFER as &mut fb::Framebuffer };
    let (btn, new_state) = match input {
        unifiedinput::GPIOEvent::Press{button} => (button, true),
        unifiedinput::GPIOEvent::Unpress{button} => (button, false),
        _ => return,
    };

    let color = match new_state {
        false => mxc_types::REMARKABLE_BRIGHTEST,
        true => mxc_types::REMARKABLE_DARKEST,
    };

    let x_offset = match btn {
        unifiedinput::PhysicalButton::LEFT => 50,
        unifiedinput::PhysicalButton::MIDDLE => {
            if new_state {
                draw_initial_scene(true);
            };
            return
        },
        unifiedinput::PhysicalButton::RIGHT => 1250,
    };

    framebuffer.fill_rect(1500, x_offset, 125, 125, color);
    framebuffer.refresh(
        mxc_types::mxcfb_rect {
            top: 1500,
            left: x_offset as u32,
            height: 125,
            width: 125,
        },
        update_mode::UPDATE_MODE_PARTIAL,
        waveform_mode::WAVEFORM_MODE_DU,
        display_temp::TEMP_USE_PAPYRUS,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        0,
    );
}

fn draw_initial_scene(quick: bool) {
    let img = image::load_from_memory(include_bytes!("../rustlang.bmp")).unwrap();
    clear(quick);

    show_image(&img, 10, 900);
    display_text(
        200,
        100,
        100,
        "Remarkable Tablet".to_owned(),
        false,
    );
}

static mut G_FRAMEBUFFER: *mut fb::Framebuffer = std::ptr::null_mut::<fb::Framebuffer>();
static mut G_UNIFIED_IOH: *mut unifiedinput::UnifiedInputHandler = std::ptr::null_mut::<unifiedinput::UnifiedInputHandler>();
fn main() {
    let fbuffer = Box::new(fb::Framebuffer::new("/dev/fb0"));

    // TODO: Maybe actually try to reason with the borrow checker here
    unsafe {
        G_FRAMEBUFFER = Box::leak(fbuffer);
    };

    draw_initial_scene(false);

    let clock_thread = std::thread::spawn(move || {
        loop_print_time(100, 100, 65);
    });

    let ringbuffer = librustpad::rb::SpscRb::new(4096);
    let consumer = ringbuffer.consumer();
    let producer = Box::new(ringbuffer.producer());
    let static_ref: &'static mut Producer<unifiedinput::InputEvent> = Box::leak(producer);
    let mut unified = unifiedinput::UnifiedInputHandler::new(false, static_ref);
    unsafe {
        G_UNIFIED_IOH = &mut unified;
    }
    let wacom_thread = std::thread::spawn(move || {
        librustpad::ev::start_evdev(
            "/dev/input/event0".to_owned(),
            unsafe { &mut *G_UNIFIED_IOH as &mut unifiedinput::UnifiedInputHandler },
        );
    });
    let touch_thread = std::thread::spawn(move || {
        librustpad::ev::start_evdev(
            "/dev/input/event1".to_owned(),
            unsafe { &mut *G_UNIFIED_IOH as &mut unifiedinput::UnifiedInputHandler },
        );
    });
    let gpio_thread = std::thread::spawn(move || {
        librustpad::ev::start_evdev(
            "/dev/input/event2".to_owned(),
            unsafe { &mut *G_UNIFIED_IOH as &mut unifiedinput::UnifiedInputHandler },
        );
    });

    // Now we consume the input events;
    let mut buf = [unifiedinput::InputEvent::Unknown {}; 512];
    let mut _running = true;
    while _running {
        let _read = consumer.read_blocking(&mut buf).unwrap();
        for &ev in buf.iter() {
            match ev {
                unifiedinput::InputEvent::GPIO{event} => {
                    on_button_press(event);
                },
                unifiedinput::InputEvent::MultitouchEvent{event} => {
                    on_touch(event);
                },
                unifiedinput::InputEvent::WacomEvent{event} => {
                    on_wacom_input(event);
                },
                _ => {},
            }
        }
    }

    // Wait for all threads to join
    clock_thread.join().unwrap();
    gpio_thread.join().unwrap();
    wacom_thread.join().unwrap();
    touch_thread.join().unwrap();
}
