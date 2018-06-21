#![feature(nll)]
#[macro_use]
extern crate lazy_static;

extern crate env_logger;
#[macro_use]
extern crate log;

extern crate chrono;
extern crate libremarkable;

use chrono::{DateTime, Local};

use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use libremarkable::framebuffer::common::*;
use libremarkable::image;
use libremarkable::image::GenericImage;

use libremarkable::appctx;
use libremarkable::ui_extensions::element::{
    UIConstraintRefresh, UIElement, UIElementHandle, UIElementWrapper,
};

use libremarkable::framebuffer::refresh::PartialRefreshMode;
use libremarkable::framebuffer::storage;
use libremarkable::framebuffer::{FramebufferDraw, FramebufferIO, FramebufferRefresh};

use libremarkable::battery;
use libremarkable::input::{gpio, multitouch, wacom, InputDevice};

use std::process::Command;

fn loop_update_topbar(app: &mut appctx::ApplicationContext, millis: u64) {
    let time_label = app.get_element_by_name("time").unwrap();
    let battery_label = app.get_element_by_name("battery").unwrap();
    loop {
        // Get the datetime
        let dt: DateTime<Local> = Local::now();

        if let UIElement::Text {
            ref mut text,
            scale: _,
            foreground: _,
            border_px: _,
        } = time_label.write().inner
        {
            *text = format!("{}", dt.format("%F %r"));
        }

        if let UIElement::Text {
            ref mut text,
            scale: _,
            foreground: _,
            border_px: _,
        } = battery_label.write().inner
        {
            *text = format!(
                "{0:<128}",
                format!(
                    "{0} — {1}%",
                    battery::human_readable_charging_status().unwrap(),
                    battery::percentage().unwrap()
                )
            );
        }
        app.draw_element("time");
        app.draw_element("battery");
        sleep(Duration::from_millis(millis));
    }
}

fn on_wacom_input(app: &mut appctx::ApplicationContext, input: wacom::WacomEvent) {
    match input {
        wacom::WacomEvent::Draw {
            y,
            x,
            pressure,
            tilt_x: _,
            tilt_y: _,
        } => {
            let mut prev = PREV_WACOM.lock().unwrap();

            // This is so that we can click the buttons outside the canvas region
            // normally meant to be touched with a finger using our stylus
            if !CANVAS_REGION.contains_point(y.into(), x.into()) {
                *prev = (-1, -1);
                if UNPRESS_OBSERVED.fetch_and(false, Ordering::Relaxed) {
                    match app.find_active_region(y, x) {
                        Some((region, _)) => (region.handler)(app, region.element.clone()),
                        None => {}
                    };
                }
                return;
            }

            let framebuffer = app.get_framebuffer_ref();

            let mut rad =
                SIZE_MULTIPLIER.load(Ordering::Relaxed) as f32 * (pressure as f32) / 4096.;
            let mut color = color::BLACK;

            // Eraser has a larger radius for ease of removal
            if ERASE_MODE.load(Ordering::Relaxed) {
                rad *= 3.0;
                color = color::WHITE;
            }
            if prev.0 >= 0 && prev.1 >= 0 {
                let rect = framebuffer.draw_line(
                    y as i32,
                    x as i32,
                    prev.0,
                    prev.1,
                    rad.ceil() as usize,
                    color,
                );
                framebuffer.partial_refresh(
                    &rect,
                    PartialRefreshMode::Async,
                    waveform_mode::WAVEFORM_MODE_DU,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_EXP1,
                    DRAWING_QUANT_BIT,
                    false,
                );
            }
            *prev = (y as i32, x as i32);
        }
        wacom::WacomEvent::InstrumentChange { pen: _, state } => {
            // Stop drawing when instrument has left the vicinity of the screen
            if !state {
                let mut prev = PREV_WACOM.lock().unwrap();
                *prev = (-1, -1);
            }
        }
        wacom::WacomEvent::Hover {
            y: _,
            x: _,
            distance,
            tilt_x: _,
            tilt_y: _,
        } => {
            // If the pen is hovering, don't record its coordinates as the origin of the next line
            if distance > 1 {
                let mut prev = PREV_WACOM.lock().unwrap();
                *prev = (-1, -1);
                UNPRESS_OBSERVED.store(true, Ordering::Relaxed);
            }
        }
        _ => {}
    };
}

#[allow(unused_variables)]
fn on_touch_handler(app: &mut appctx::ApplicationContext, input: multitouch::MultitouchEvent) {
    let framebuffer = app.get_framebuffer_ref();
    match input {
        multitouch::MultitouchEvent::Touch {
            gesture_seq,
            finger_id,
            y,
            x,
        } => {
            let action = { DRAW_ON_TOUCH.lock().unwrap().clone() };
            let rect = match action {
                1 => framebuffer.draw_bezier(
                    (x as f32, y as f32),
                    ((x + 155) as f32, (y + 14) as f32),
                    ((x + 200) as f32, (y + 200) as f32),
                    color::BLACK,
                ),
                2 => framebuffer.draw_circle(y as usize, x as usize, 20, color::BLACK),
                _ => return,
            };
            framebuffer.partial_refresh(
                &rect,
                PartialRefreshMode::Async,
                waveform_mode::WAVEFORM_MODE_DU,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_ALPHA,
                DRAWING_QUANT_BIT,
                false,
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
        gpio::PhysicalButton::RIGHT => {
            let new_state = match app.is_input_device_active(InputDevice::Multitouch) {
                true => {
                    app.deactivate_input_device(InputDevice::Multitouch);
                    "Enable Touch"
                }
                false => {
                    app.activate_input_device(InputDevice::Multitouch);
                    "Disable Touch"
                }
            };

            match app.get_element_by_name("tooltipRight") {
                Some(ref elem) => {
                    if let UIElement::Text {
                        ref mut text,
                        scale: _,
                        foreground: _,
                        border_px: _,
                    } = elem.write().inner
                    {
                        *text = new_state.to_string();
                    }
                }
                None => {}
            }
            app.draw_element("tooltipRight");
            return;
        }
        gpio::PhysicalButton::MIDDLE => {
            app.clear(true);
        }
        gpio::PhysicalButton::LEFT => {
            app.clear(false);
        }
    };

    app.draw_elements();
}

fn on_save_canvas(app: &mut appctx::ApplicationContext, _element: UIElementHandle) {
    let framebuffer = app.get_framebuffer_ref();
    match framebuffer.dump_region(CANVAS_REGION) {
        Err(err) => println!("Failed to dump buffer: {0}", err),
        Ok(buff) => {
            let mut hist = SAVED_CANVAS.lock().unwrap();
            *hist = Some(storage::CompressedCanvasState::new(buff));
        }
    };
}

fn on_invert_canvas(app: &mut appctx::ApplicationContext, _element: UIElementHandle) {
    let framebuffer = app.get_framebuffer_ref();
    match framebuffer.dump_region(CANVAS_REGION) {
        Err(err) => println!("Failed to dump buffer: {0}", err),
        Ok(buff) => {
            let mut dynamic = image::DynamicImage::ImageRgba8(buff);
            dynamic.invert();

            match framebuffer.restore_region(CANVAS_REGION, &dynamic.as_rgba8().unwrap()) {
                Err(e) => println!("Error while restoring region: {0}", e),
                Ok(_) => {
                    framebuffer.partial_refresh(
                        &CANVAS_REGION,
                        PartialRefreshMode::Async,
                        waveform_mode::WAVEFORM_MODE_GC16_FAST,
                        display_temp::TEMP_USE_REMARKABLE_DRAW,
                        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                        0,
                        false,
                    );
                }
            };
        }
    };
}

fn on_load_canvas(app: &mut appctx::ApplicationContext, _element: UIElementHandle) {
    match *SAVED_CANVAS.lock().unwrap() {
        None => {}
        Some(ref compressed_state) => {
            let framebuffer = app.get_framebuffer_ref();
            let decompressed = compressed_state.decompress();

            match framebuffer.restore_region(CANVAS_REGION, &decompressed) {
                Err(e) => println!("Error while restoring region: {0}", e),
                Ok(_) => {
                    framebuffer.partial_refresh(
                        &CANVAS_REGION,
                        PartialRefreshMode::Async,
                        waveform_mode::WAVEFORM_MODE_GC16_FAST,
                        display_temp::TEMP_USE_REMARKABLE_DRAW,
                        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                        0,
                        false,
                    );
                }
            };
        }
    };
}

fn on_touch_exit_to_xochitl(_app: &mut appctx::ApplicationContext, _element: UIElementHandle) {
    Command::new("systemctl")
        .arg("start")
        .arg("xochitl")
        .spawn()
        .unwrap();
    std::process::exit(0);
}

fn on_touch_rustlogo(app: &mut appctx::ApplicationContext, _element: UIElementHandle) {
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
        color::BLACK,
        false,
    );
    framebuffer.partial_refresh(
        &rect,
        PartialRefreshMode::Wait,
        waveform,
        display_temp::TEMP_USE_MAX,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        false,
    );
}

fn on_toggle_eraser(app: &mut appctx::ApplicationContext, element: UIElementHandle) {
    {
        let newstate = !ERASE_MODE.load(Ordering::Relaxed);
        ERASE_MODE.store(newstate, Ordering::Relaxed);
        if let UIElement::Text {
            ref mut text,
            scale: _,
            foreground: _,
            border_px: _,
        } = element.write().inner
        {
            *text = format!("Erase: {0}", if newstate { "On" } else { "Off" })
        }
    }
    app.draw_element("eraseToggle");
}

fn on_decrease_size(app: &mut appctx::ApplicationContext, _: UIElementHandle) {
    let current = SIZE_MULTIPLIER.load(Ordering::Relaxed);
    if current <= 1 {
        return;
    }
    let new = current - 1;
    SIZE_MULTIPLIER.store(new, Ordering::Relaxed);

    let element = app.get_element_by_name("displaySize").unwrap();
    {
        if let UIElement::Text {
            ref mut text,
            scale: _,
            foreground: _,
            border_px: _,
        } = element.write().inner
        {
            *text = format!("size: {0}", new)
        }
    }
    app.draw_element("displaySize");
}

fn on_increase_size(app: &mut appctx::ApplicationContext, _: UIElementHandle) {
    let current = SIZE_MULTIPLIER.load(Ordering::Relaxed);
    if current >= 99 {
        return;
    }
    let new = current + 1;
    SIZE_MULTIPLIER.store(new, Ordering::Relaxed);

    let element = app.get_element_by_name("displaySize").unwrap();
    {
        if let UIElement::Text {
            ref mut text,
            scale: _,
            foreground: _,
            border_px: _,
        } = element.write().inner
        {
            *text = format!("size: {0}", new)
        }
    }
    app.draw_element("displaySize");
}

fn on_change_draw_type(app: &mut appctx::ApplicationContext, element: UIElementHandle) {
    {
        let mut data = DRAW_ON_TOUCH.lock().unwrap();
        *data = (*data + 1) % 3;

        if let UIElement::Text {
            ref mut text,
            scale: _,
            foreground: _,
            border_px: _,
        } = element.write().inner
        {
            *text = format!(
                "Touch Mode: {0}",
                match *data {
                    1 => "Bezier",
                    2 => "Circles",
                    _ => "None",
                }
            )
        }
    }
    // Make sure you aren't trying to draw the element while you are holding a write lock.
    // It doesn't seem to cause a deadlock however it may cause higher lock contention.
    app.draw_element("touchMode");
}

// will have the following size at rest:
//   snapshot raw: 5896 kB
//   zstd: 10 kB
const CANVAS_REGION: mxcfb_rect = mxcfb_rect {
    top: 750,
    left: 0,
    height: 1050,
    width: 1404,
};

lazy_static! {
    static ref DRAW_ON_TOUCH: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    static ref ERASE_MODE: AtomicBool = AtomicBool::new(false);
    static ref SIZE_MULTIPLIER: AtomicUsize = AtomicUsize::new(4);
    static ref UNPRESS_OBSERVED: AtomicBool = AtomicBool::new(false);
    static ref PREV_WACOM: Arc<Mutex<(i32, i32)>> = Arc::new(Mutex::new((-1, -1)));
    static ref G_COUNTER: Mutex<u32> = Mutex::new(0);
    static ref SAVED_CANVAS: Mutex<Option<storage::CompressedCanvasState>> = Mutex::new(None);
}

fn main() {
    env_logger::init();

    // Takes callback functions as arguments
    // They are called with the event and the &mut framebuffer
    let mut app: appctx::ApplicationContext =
        appctx::ApplicationContext::new(on_button_press, on_wacom_input, on_touch_handler);

    // Alternatively we could have called `app.execute_lua("fb.clear()")`
    app.clear(true);

    // A rudimentary way to declare a scene and layout
    app.add_element(
        "logo",
        UIElementWrapper {
            y: 10,
            x: 900,
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
        },
    );

    // Draw the borders for the canvas region
    app.add_element(
        "canvasRegion",
        UIElementWrapper {
            y: CANVAS_REGION.top as usize,
            x: CANVAS_REGION.left as usize,
            refresh: UIConstraintRefresh::RefreshAndWait,
            onclick: None,
            inner: UIElement::Region {
                height: CANVAS_REGION.height as usize,
                width: CANVAS_REGION.width as usize,
                border_px: 2,
                border_color: color::BLACK,
            },
            ..Default::default()
        },
    );

    // Save/Restore Controls
    app.add_element(
        "saveButton",
        UIElementWrapper {
            y: 400,
            x: 1000,
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_save_canvas),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "✓ Save".to_owned(),
                scale: 55,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "restoreButton",
        UIElementWrapper {
            y: 400,
            x: 1210,
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_load_canvas),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "⟲ Load".to_owned(),
                scale: 55,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    // Touch Mode Toggle
    app.add_element(
        "touchMode",
        UIElementWrapper {
            y: 500,
            x: 1000,
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_change_draw_type),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Touch Mode: None".to_owned(),
                scale: 45,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    // Erase Mode Toggle
    app.add_element(
        "eraseToggle",
        UIElementWrapper {
            y: 610,
            x: 1000,
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_toggle_eraser),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Erase: Off".to_owned(),
                scale: 45,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    // Invert Toggle
    app.add_element(
        "invertToggle",
        UIElementWrapper {
            y: 610,
            x: 1240,
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_invert_canvas),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Invert".to_owned(),
                scale: 45,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    // Size Controls
    app.add_element(
        "decreaseSize",
        UIElementWrapper {
            y: 710,
            x: 1000,
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(on_decrease_size),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "-".to_owned(),
                scale: 90,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "displaySize",
        UIElementWrapper {
            y: 710,
            x: 1070,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "size: 4".to_owned(),
                scale: 45,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "increaseSize",
        UIElementWrapper {
            y: 710,
            x: 1250,
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(on_increase_size),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "+".to_owned(),
                scale: 90,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    app.add_element(
        "exitToXochitl",
        UIElementWrapper {
            y: 55,
            x: 550,
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_touch_exit_to_xochitl),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "EXIT TO REMARKABLE".to_owned(),
                scale: 35,
                border_px: 3,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "availAt",
        UIElementWrapper {
            y: 650,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Available at:".to_owned(),
                scale: 70,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "github",
        UIElementWrapper {
            y: 720,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "github.com/canselcik/libremarkable".to_owned(),
                scale: 55,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l1",
        UIElementWrapper {
            y: 350,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Low Latency eInk Display Partial Refresh API".to_owned(),
                scale: 45,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l3",
        UIElementWrapper {
            y: 400,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Capacitive Multitouch Input Support".to_owned(),
                scale: 45,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l2",
        UIElementWrapper {
            y: 450,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Physical Button Support".to_owned(),
                scale: 45,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l4",
        UIElementWrapper {
            y: 500,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Wacom Digitizer Support".to_owned(),
                scale: 45,
                border_px: 0,
            },
            ..Default::default()
        },
    );

    app.add_element(
        "tooltipLeft",
        UIElementWrapper {
            y: 1850,
            x: 15,
            refresh: UIConstraintRefresh::Refresh,
            onclick: None,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Quick Redraw".to_owned(), // maybe quick redraw for the demo or waveform change?
                scale: 50,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "tooltipMiddle",
        UIElementWrapper {
            y: 1850,
            x: 550,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Full Redraw".to_owned(),
                scale: 50,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "tooltipRight",
        UIElementWrapper {
            y: 1850,
            x: 1085,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Disable Touch".to_owned(),
                scale: 50,
                border_px: 0,
            },
            ..Default::default()
        },
    );

    // Create the top bar's time and battery labels. We can mutate these later.
    let dt: DateTime<Local> = Local::now();
    app.add_element(
        "battery",
        UIElementWrapper {
            y: 215,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: format!(
                    "{0:<128}",
                    format!(
                        "{0} — {1}%",
                        battery::human_readable_charging_status().unwrap(),
                        battery::percentage().unwrap()
                    )
                ),
                scale: 44,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "time",
        UIElementWrapper {
            y: 150,
            x: 100,
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: format!("{}", dt.format("%F %r")),
                scale: 75,
                border_px: 0,
            },
            ..Default::default()
        },
    );

    // Draw the scene
    app.draw_elements();

    // Get a &mut to the framebuffer object, exposing many convenience functions
    let appref = app.upgrade_ref();
    let clock_thread = std::thread::spawn(move || {
        loop_update_topbar(appref, 30 * 1000);
    });

    app.execute_lua(
        r#"
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
      left = 620;
      width = 320;
      height = 90;
      borderpx = 3;
      draw_box(top, left, height, width, borderpx, 0);

      -- Draw black text inside the box. Notice the text is bottom aligned.
      fb.draw_text(top+55, left+22, '...also supports Lua', 30, 0);

      -- Update the drawn rect w/ `deep_plot=false` and `wait_for_update_complete=true`
      fb.refresh(top, left, height, width, false, true);
    "#,
    );

    info!("Init complete. Beginning event dispatch...");

    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.dispatch_events(true, true, true);
    clock_thread.join().unwrap();
}
