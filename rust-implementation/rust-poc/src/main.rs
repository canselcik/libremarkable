extern crate librustpad;
extern crate chrono;

use chrono::{Local, DateTime};

use std::option::Option;
use std::time::Duration;
use std::thread::sleep;

use librustpad::image;
use librustpad::fb;
use librustpad::mxc_types;
use librustpad::mxc_types::display_temp;
use librustpad::mxc_types::dither_mode;
use librustpad::mxc_types::update_mode;
use librustpad::mxc_types::waveform_mode;
use librustpad::unifiedinput;
use librustpad::uix;
use librustpad::uix::UIConstraintRefresh;
use librustpad::uix::UIElement;

fn loop_print_time(framebuffer: &mut fb::Framebuffer, y: usize, x: usize, scale: usize) {
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

#[allow(unused_variables)]
fn on_wacom_input(framebuffer: &mut fb::Framebuffer, input: unifiedinput::WacomEvent) {
    match input {
        unifiedinput::WacomEvent::Draw { y, x, pressure, tilt_x, tilt_y, prevy, prevx} => {
            let mut rad = 3.8 * (pressure as f32) / 4096.;

            let rect = framebuffer.fill_circle(
                y as usize, x as usize,
                rad as usize, mxc_types::REMARKABLE_DARKEST);

            framebuffer.refresh(
                rect,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_DU,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_EXP1,
                mxc_types::DRAWING_QUANT_BIT,
                0,
            );
        }
        unifiedinput::WacomEvent::InstrumentChange { pen, state } => {
            // println!("WacomInstrumentChanged(inst: {0}, state: {1})", pen as u16, state);
        }
        unifiedinput::WacomEvent::Hover { y, x, distance, tilt_x, tilt_y } => {
            // println!("WacomHover(y: {0}, x: {1}, distance: {2})", y, x, distance);
        }
        _ => {}
    };
}

#[allow(unused_variables)]
fn on_touch(framebuffer: &mut fb::Framebuffer, input: unifiedinput::MultitouchEvent) {
    match input {
        unifiedinput::MultitouchEvent::Touch { gesture_seq, finger_id, y, x } => {
            let rect = match unsafe { DRAW_ON_TOUCH } {
               1 => framebuffer.draw_bezier((x as f32, y as f32),
                                            ((x + 155) as f32, (y + 14) as f32),
                                            ((x + 200) as f32, (y + 200) as f32),
                                            mxc_types::REMARKABLE_DARKEST),
               2 => framebuffer.draw_circle(y as usize,
                                            x as usize,
                                            20,
                                            mxc_types::REMARKABLE_DARKEST),
               _ => return,
            };
            framebuffer.refresh(
                rect,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_DU,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_ALPHA,
                mxc_types::DRAWING_QUANT_BIT,
                0,
            );
        }
        _ => {}
    }
}

fn on_button_press(framebuffer: &mut fb::Framebuffer, input: unifiedinput::GPIOEvent) {
    let (btn, new_state) = match input {
        unifiedinput::GPIOEvent::Press { button } => (button, true),
        unifiedinput::GPIOEvent::Unpress { button } => (button, false),
        _ => return,
    };

    let color = match new_state {
        false => mxc_types::REMARKABLE_BRIGHTEST,
        true => mxc_types::REMARKABLE_DARKEST,
    };

    let x_offset = match btn {
        unifiedinput::PhysicalButton::LEFT => {
            if new_state {
                unsafe {
                    DRAW_ON_TOUCH = (DRAW_ON_TOUCH + 1) % 3;
                }
            }
            return; // 50
        },
        unifiedinput::PhysicalButton::MIDDLE => {
            if new_state {
                framebuffer.clear();
            }
            return;
        }
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
        dither_mode::EPDC_FLAG_USE_DITHERING_ALPHA,
        0,
        0,
    );
}

// 0 -> None
// 1 -> Circles
// 2 -> Bezier
static mut DRAW_ON_TOUCH: u32 = 0;
fn main() {
    // Takes callback functions as arguments
    // They are called with the event and the &mut framebuffer
    let mut app = uix::ApplicationContext::new(
        on_button_press,
        on_wacom_input,
        on_touch,
    );

    // We could just call `app.clear(true)` but let's invoke via Lua to showcase the API
    app.execute_lua("fb.clear()");

    // A rudimentary way to declare a scene and layout
    app.draw_elements(&vec![
        UIElement::Text {
            text: "Remarkable Tablet".to_owned(),
            y: 200, x: 100,
            scale: 100,
            refresh: UIConstraintRefresh::NoRefresh
        },
        UIElement::Image {
            img: image::load_from_memory(include_bytes!("../rustlang.bmp")).unwrap(),
            y: 10, x: 900,
            refresh: UIConstraintRefresh::Refresh },
        UIElement::Text {
            text: "Current Waveform: ".to_owned(),
            y: 350, x: 120,
            scale: 65, refresh: UIConstraintRefresh::NoRefresh
        },
        UIElement::Text {
            text: "Current Dither Mode: ".to_owned(),
            y: 410, x: 120,
            scale: 65,
            refresh: UIConstraintRefresh::NoRefresh
        },
        UIElement::Text {
            text: "Current Quant: ".to_owned(),
            y: 470, x: 120,
            scale: 65,
            refresh: UIConstraintRefresh::RefreshAndWait
        },
    ]);

    // Get a &mut to the framebuffer object, exposing many convenience functions
    let fb = app.get_framebuffer_ref();
    let clock_thread = std::thread::spawn(move || {
        loop_print_time(fb, 100, 100, 65);
    });

    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.dispatch_events(8192, 512);
    clock_thread.join().unwrap();
}