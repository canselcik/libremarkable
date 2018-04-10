#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate libremarkable;
extern crate chrono;

use chrono::{Local, DateTime};

use std::option::Option;
use std::time::Duration;
use std::thread::sleep;
use std::sync::Mutex;
use std::sync::{RwLock, Arc};

use libremarkable::image;
use libremarkable::framebuffer::core;
use libremarkable::framebuffer::common::*;

use libremarkable::appctx;
use libremarkable::ui_extensions::element::{UIElement,UIElementWrapper,UIConstraintRefresh};

use libremarkable::framebuffer::refresh::PartialRefreshMode;
use libremarkable::framebuffer::{FramebufferDraw, FramebufferRefresh};

use libremarkable::input::{wacom,gpio,multitouch};
use libremarkable::battery;

fn loop_update_topbar(framebuffer: &mut core::Framebuffer, y: usize, x: usize, scale: usize, millis: u64) {
    let mut time_draw_area: Option<mxcfb_rect> = None;
    let mut battery_draw_area: Option<mxcfb_rect> = None;
    loop {
        // Get the datetime
        let dt: DateTime<Local> = Local::now();

        // Skip the fill background step upon first pass
        match (time_draw_area, battery_draw_area) {
            (Some(ref time_area), Some(ref battery_area)) => {
                framebuffer.fill_rect(
                    time_area.top as usize,
                    time_area.left as usize,
                    time_area.height as usize,
                    time_area.width as usize,
                    REMARKABLE_BRIGHTEST,
                );
                framebuffer.fill_rect(
                    battery_area.top as usize,
                    battery_area.left as usize,
                    battery_area.height as usize,
                    battery_area.width as usize,
                    REMARKABLE_BRIGHTEST,
                )
            },
            _ => {},
        };

        // Create the draw_areas
        time_draw_area = Some(framebuffer.draw_text(y, x,
            format!("{}", dt.format("%F %r")),
            scale,
            REMARKABLE_DARKEST,
        ));
        battery_draw_area = Some(framebuffer.draw_text(
            y + 65, x,
            format!("{0:<128}", format!("{0} — {1}%",
                    battery::human_readable_charging_status().unwrap(),
                    battery::percentage().unwrap())),
            2 * scale / 3, REMARKABLE_DARKEST
        ));

        // Now refresh the regions
        match (time_draw_area, battery_draw_area) {
            (Some(ref time_area), Some(ref battery_area)) => {
                framebuffer.partial_refresh(
                    time_area,
                    PartialRefreshMode::Async,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_AMBIENT,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                );
                // Refresh the battery status region on the screen
                framebuffer.partial_refresh(
                    battery_area,
                    PartialRefreshMode::Wait,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_AMBIENT,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                );
            },
            _ => {},
        }

        sleep(Duration::from_millis(millis));
    }
}

fn on_wacom_input(app: &mut appctx::ApplicationContext, input: wacom::WacomEvent) {
    let framebuffer = app.get_framebuffer_ref();
    let mut prev = PREV_WACOM.lock().unwrap();
    match input {
        wacom::WacomEvent::Draw { y, x, pressure, tilt_x: _, tilt_y: _ } => {
            let mut rad = 3.8 * (pressure as f32) / 4096.;
            if prev.0 >= 0 && prev.1 >= 0 {
                let rect = framebuffer.draw_line(
                    y as i32, x as i32,prev.0, prev.1,
                    rad.ceil() as usize,REMARKABLE_DARKEST
                );
                framebuffer.partial_refresh(
                    &rect,
                    PartialRefreshMode::Async,
                    waveform_mode::WAVEFORM_MODE_DU,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_EXP1,
                    DRAWING_QUANT_BIT,
                );
            }
            *prev = (y as i32, x as i32);
        }
        wacom::WacomEvent::InstrumentChange { pen: _, state } => {
            // Stop drawing when instrument has left the vicinity of the screen
            if !state {
                *prev = (-1, -1);
            }
        }
        wacom::WacomEvent::Hover { y: _, x: _, distance, tilt_x: _, tilt_y: _ } => {
            // If the pen is hovering, don't record its coordinates as the origin of the next line
            if distance > 1 {
                *prev = (-1, -1);
            }
        }
        _ => {}
    };
}

#[allow(unused_variables)]
fn on_touch_handler(app: &mut appctx::ApplicationContext, input: multitouch::MultitouchEvent) {
    let framebuffer = app.get_framebuffer_ref();
    match input {
        multitouch::MultitouchEvent::Touch { gesture_seq, finger_id, y, x } => {
            let action = { DRAW_ON_TOUCH.lock().unwrap().clone() };
            let rect = match action {
               1 => framebuffer.draw_bezier((x as f32, y as f32),
                                            ((x + 155) as f32, (y + 14) as f32),
                                            ((x + 200) as f32, (y + 200) as f32),
                                            REMARKABLE_DARKEST),
               2 => framebuffer.draw_circle(y as usize,
                                            x as usize,
                                            20,
                                            REMARKABLE_DARKEST),
               _ => return,
            };
            framebuffer.partial_refresh(
                &rect,
                PartialRefreshMode::Async,
                waveform_mode::WAVEFORM_MODE_DU,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_ALPHA,
                DRAWING_QUANT_BIT,
            );
        }
        _ => {}
    }
}

fn on_button_press(app: &mut appctx::ApplicationContext, input: gpio::GPIOEvent) {
    let (btn, new_state) = match input {
        gpio::GPIOEvent::Press { button } => (button, true),
        gpio::GPIOEvent::Unpress { button } => (button, false),
        _ => return,
    };

    // Ignoring the unpressed event
    if !new_state {
        return;
    }

    match btn {
        gpio::PhysicalButton::LEFT => {
            let mut data = DRAW_ON_TOUCH.lock().unwrap();
            *data = (*data + 1) % 3;
            return
        },
        gpio::PhysicalButton::MIDDLE => {
            app.clear(true);
        },
        gpio::PhysicalButton::RIGHT => {
            app.clear(false);
        },
    };

    // do_loop set to false, it will simply clear, print and
    // update the top bar so that it isn't empty after the
    // scene is redrawn.
//    loop_update_topbar(framebuffer, 150, 100,
//                       75, 30 * 1000, false);

    app.draw_elements();

    // Alternatively, if we wanted to just clear the bottom half of the screen
    // we could have done something like this:
    //     let framebuffer = app.get_framebuffer_ref();
    //     let (yres, xres) = (framebuffer.var_screen_info.yres,
    //                         framebuffer.var_screen_info.xres);
    //     let offset = 45 * yres / 100;
    //     let height = yres - offset;
    //     framebuffer.fill_rect(
    //         offset as usize,
    //         0,
    //         height as usize,
    //         xres as usize,
    //         REMARKABLE_BRIGHTEST
    //     );
    //     framebuffer.partial_refresh(
    //         &mxcfb_rect {
    //             top: offset,
    //             left: 0,
    //             height,
    //             width: xres,
    //         },
    //         PartialRefreshMode::Wait,
    //         waveform_mode::WAVEFORM_MODE_INIT,
    //         display_temp::TEMP_USE_AMBIENT,
    //         dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
    //         0,
    //     );

    // If you wanted to draw a rectangle and refresh it, this is how it is done:
    //    framebuffer.fill_rect(1500, x_offset, 125, 125, color);
    //    framebuffer.partial_refresh(
    //        &mxcfb_rect {
    //            top: 1500,
    //            left: x_offset as u32,
    //            height: 125,
    //            width: 125,
    //        },
    //        PartialRefreshMode::Async,
    //        waveform_mode::WAVEFORM_MODE_DU,
    //        display_temp::TEMP_USE_PAPYRUS,
    //        dither_mode::EPDC_FLAG_USE_DITHERING_ALPHA,
    //        0,
    //    );
}

fn on_touch_rustlogo(app: &mut appctx::ApplicationContext,
                     _element: Arc<RwLock<UIElementWrapper>>) {
    let framebuffer = app.get_framebuffer_ref();
    let new_press_count = {
        let mut v = G_COUNTER.lock().unwrap();
        *v += 1;
        (*v).clone()
    };

    // First drawing with GC16_FAST to draw it thoroughly and then
    // alternating between DU which has more artifacts but is faster.
    let waveform = if new_press_count % 2 == 0 {
        waveform_mode::WAVEFORM_MODE_DU
    } else {
        waveform_mode::WAVEFORM_MODE_GC16_FAST
    };

    let rect = framebuffer.draw_text(
        240,
        1140,
        format!("{0}", new_press_count),
        65,
        REMARKABLE_DARKEST
    );
    framebuffer.partial_refresh(&rect, PartialRefreshMode::Wait,
                                waveform, display_temp::TEMP_USE_MAX,
                                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH, 0);
}

lazy_static! {
    // 0 -> None
    // 1 -> Circles
    // 2 -> Bezier
    static ref DRAW_ON_TOUCH: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    static ref PREV_WACOM: Arc<Mutex<(i32, i32)>> = Arc::new(Mutex::new((-1, -1)));
    static ref G_COUNTER: Mutex<u32> = Mutex::new(0);
}

fn main() {
    env_logger::init();

    // Takes callback functions as arguments
    // They are called with the event and the &mut framebuffer
    let mut app : appctx::ApplicationContext = appctx::ApplicationContext::new(
        on_button_press,
        on_wacom_input,
        on_touch_handler,
    );

    // Alternatively we could have called `app.execute_lua("fb.clear()")`
    app.clear(true);

    // A rudimentary way to declare a scene and layout
    app.add_element("logo", Arc::new(RwLock::new(UIElementWrapper {
        y: 10, x: 900,
        refresh: UIConstraintRefresh::Refresh,

        /* We could have alternatively done this:
           // Create a clickable region for multitouch input and associate it with its handler fn
           app.create_active_region(10, 900, 240, 480, on_touch_rustlogo);
        */
        onclick: Some(on_touch_rustlogo),
        inner: UIElement::Image {
            img: image::load_from_memory(include_bytes!("../assets/rustlang.bmp")).unwrap(),
        },
        ..Default::default()
    })));
    app.add_element("availAt", Arc::new(RwLock::new(UIElementWrapper {
        y: 650, x: 120,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Available at:".to_owned(),
            scale: 70,
        },
        ..Default::default()
    })));
    app.add_element("github", Arc::new(RwLock::new(UIElementWrapper {
        y: 750, x: 100,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "github.com/canselcik/libremarkable".to_owned(),
            scale: 60,
        },
        ..Default::default()
    })));
    app.add_element("l1", Arc::new(RwLock::new(UIElementWrapper {
        y: 350, x: 120,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Low Latency eInk Display Partial Refresh API".to_owned(),
            scale: 55,
        },
        ..Default::default()
    })));
    app.add_element("l2", Arc::new(RwLock::new(UIElementWrapper {
        y: 470, x: 120,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Physical Button Support".to_owned(),
            scale: 55,
        },
        ..Default::default()
    })));
    app.add_element("l3", Arc::new(RwLock::new(UIElementWrapper {
        y: 410, x: 120,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Capacitive Multitouch Input Support".to_owned(),
            scale: 55,
        },
        ..Default::default()
    })));
    app.add_element("l4", Arc::new(RwLock::new(UIElementWrapper {
        y: 530, x: 120,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Wacom Digitizer Support".to_owned(),
            scale: 55,
        },
        ..Default::default()
    })));

    app.add_element("tooltipLeft", Arc::new(RwLock::new(UIElementWrapper {
        y: 1850, x: 15,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Toggle Touch".to_owned(),
            scale: 50,
        },
        ..Default::default()
    })));
    app.add_element("tooltipMiddle", Arc::new(RwLock::new(UIElementWrapper {
        y: 1850, x: 550,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Redraw Layout".to_owned(),
            scale: 50,
        },
        ..Default::default()
    })));
    app.add_element("tooltipRight", Arc::new(RwLock::new(UIElementWrapper {
        y: 1850, x: 1085,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: "Quick Redraw".to_owned(), // maybe quick redraw for the demo or waveform change?
            scale: 50,
        },
        ..Default::default()
    })));


    // Create the top bar's time label. We can mutate this later.
    let dt: DateTime<Local> = Local::now();
    let time_label = Arc::new(RwLock::new(UIElementWrapper {
        y: 150, x: 100,
        refresh: UIConstraintRefresh::Refresh,
        inner: UIElement::Text {
            text: format!("{}", dt.format("%F %r")),
            scale: 75,
        },
        ..Default::default()
    }));
    app.add_element("time", Arc::clone(&time_label));

    // Draw the scene
    app.draw_elements();

    // Get a &mut to the framebuffer object, exposing many convenience functions
    let fb = app.get_framebuffer_ref();
    let clock_thread = std::thread::spawn(move || {
        loop_update_topbar(fb, 150, 100, 75, 30 * 1000);
    });

    app.execute_lua(r#"
      function draw_box(y, x, height, width, borderpx, bordercolor)
        local maxy = y+height;
        local maxx = x+width;
        for cy=y,maxy,1 do
          for cx=x,maxx,1 do
            if (math.abs(cx-x) < borderpx or math.abs(maxx-cx) < borderpx) or
               (math.abs(cy-y) < borderpx or math.abs(maxy-cy) < borderpx) then
              fb.set_pixel(cy, cx, bordercolor);
            end
          end
        end
      end

      top = 450;
      left = 820;
      width = 420;
      height = 90;
      borderpx = 5;
      draw_box(top, left, height, width, borderpx, 0);

      -- Draw black text inside the box. Notice the text is bottom aligned.
      fb.draw_text(top+55, left+22, '...also supports Lua', 45, 0);

      -- Update the drawn rect w/ `deep_plot=false` and `wait_for_update_complete=true`
      fb.refresh(top, left, height, width, false, true);
    "#);

    info!("Init complete. Beginning event dispatch...");

    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.dispatch_events(8192, 1);
    clock_thread.join().unwrap();
}
