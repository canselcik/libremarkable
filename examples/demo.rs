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
use std::sync::Arc;

use libremarkable::image;
use libremarkable::fb;
use libremarkable::mxc_types;
use libremarkable::mxc_types::display_temp;
use libremarkable::mxc_types::dither_mode;
use libremarkable::mxc_types::update_mode;
use libremarkable::mxc_types::waveform_mode;
use libremarkable::unifiedinput;
use libremarkable::uix;
use libremarkable::uix::UIConstraintRefresh;
use libremarkable::uix::UIElement;

use libremarkable::fbdraw::FramebufferDraw;
use libremarkable::refresh::FramebufferRefresh;
use libremarkable::battery;

fn loop_update_topbar(framebuffer: &mut fb::Framebuffer, y: usize, x: usize, scale: usize, millis: u64) {
    let mut time_draw_area: Option<mxc_types::mxcfb_rect> = None;
    let mut battery_draw_area: Option<mxc_types::mxcfb_rect> = None;
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
                    mxc_types::REMARKABLE_BRIGHTEST,
                );
                framebuffer.fill_rect(
                    battery_area.top as usize,
                    battery_area.left as usize,
                    battery_area.height as usize,
                    battery_area.width as usize,
                    mxc_types::REMARKABLE_BRIGHTEST,
                )
            },
            _ => {},
        };

        // Create the draw_areas
        time_draw_area = Some(framebuffer.draw_text(y, x,
            format!("{}", dt.format("%F %r")),
            scale,
            mxc_types::REMARKABLE_DARKEST,
        ));
        battery_draw_area = Some(framebuffer.draw_text(
            y + 65, x,
            format!("{0} — {1}%",
                    battery::human_readable_charging_status().unwrap(),
                    battery::percentage().unwrap()),
            2 * scale / 3, mxc_types::REMARKABLE_DARKEST
        ));

        // Now refresh the regions
        match (time_draw_area, battery_draw_area) {
            (Some(ref time_area), Some(ref battery_area)) => {
                framebuffer.refresh(
                    time_area,
                    update_mode::UPDATE_MODE_PARTIAL,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_AMBIENT,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    0,
                );
                // Refresh the battery status region on the screen
                let marker = framebuffer.refresh(
                    battery_area,
                    update_mode::UPDATE_MODE_PARTIAL,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_AMBIENT,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    0,
                );
                framebuffer.wait_refresh_complete(marker);
            },
            _ => {},
        }

        sleep(Duration::from_millis(millis));
    }
}

fn on_wacom_input(framebuffer: &mut fb::Framebuffer, input: unifiedinput::WacomEvent) {
    let mut prev = PREV_WACOM.lock().unwrap();
    match input {
        unifiedinput::WacomEvent::Draw { y, x, pressure, tilt_x: _, tilt_y: _ } => {
            let mut rad = 3.8 * (pressure as f32) / 4096.;
            if prev.0 >= 0 && prev.1 >= 0 {
                let rect = framebuffer.draw_line(
                    y as i32, x as i32,prev.0, prev.1,
                    rad.ceil() as usize,mxc_types::REMARKABLE_DARKEST
                );
                framebuffer.refresh(
                    &rect,
                    update_mode::UPDATE_MODE_PARTIAL,
                    waveform_mode::WAVEFORM_MODE_DU,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_EXP1,
                    mxc_types::DRAWING_QUANT_BIT,
                    0,
                );
            }
            *prev = (y as i32, x as i32);
        }
        unifiedinput::WacomEvent::InstrumentChange { pen: _, state } => {
            // Stop drawing when instrument has left the vicinity of the screen
            if !state {
                *prev = (-1, -1);
            }
        }
        unifiedinput::WacomEvent::Hover { y: _, x: _, distance, tilt_x: _, tilt_y: _ } => {
            // If the pen is hovering, don't record its coordinates as the origin of the next line
            if distance > 1 {
                *prev = (-1, -1);
            }
        }
        _ => {}
    };
}

#[allow(unused_variables)]
fn on_touch_handler(framebuffer: &mut fb::Framebuffer, input: unifiedinput::MultitouchEvent) {
    match input {
        unifiedinput::MultitouchEvent::Touch { gesture_seq, finger_id, y, x } => {
            let action = { DRAW_ON_TOUCH.lock().unwrap().clone() };
            let rect = match action {
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
                &rect,
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

    let (yres, xres) = (framebuffer.var_screen_info.yres, framebuffer.var_screen_info.xres);
    let offset = 45 * yres / 100;
    let height = yres - offset;
    let x_offset = match btn {
        unifiedinput::PhysicalButton::LEFT => {
            if new_state {
                let mut data = DRAW_ON_TOUCH.lock().unwrap();
                *data = (*data + 1) % 3;
            }
            return; // 50
        },
        unifiedinput::PhysicalButton::MIDDLE => {
            if new_state {
                framebuffer.fill_rect(
                    offset as usize,
                    0,
                    height as usize,
                    xres as usize,
                    mxc_types::REMARKABLE_BRIGHTEST
                );
                let rect = mxc_types::mxcfb_rect {
                    top: offset,
                    left: 0,
                    height,
                    width: xres,
                };
                framebuffer.refresh(
                    &rect,
                    update_mode::UPDATE_MODE_FULL,
                    waveform_mode::WAVEFORM_MODE_INIT,
                    display_temp::TEMP_USE_AMBIENT,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0, 0,
                );
            }
            return;
        }
        unifiedinput::PhysicalButton::RIGHT => 1250,
    };

    framebuffer.fill_rect(1500, x_offset, 125, 125, color);
    framebuffer.refresh(
        &mxc_types::mxcfb_rect {
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

static mut G_COUNTER: u32 = 0;
fn on_touch_rustlogo(framebuffer: &mut fb::Framebuffer) {
    unsafe {
        // First drawing with GC16_FAST to draw it thoroughly and then
        // alternating between DU which has more artifacts but is faster.
        let waveform= if G_COUNTER % 2 == 0 {
            waveform_mode::WAVEFORM_MODE_GC16_FAST
        }
        else {
            waveform_mode::WAVEFORM_MODE_DU
        };

        G_COUNTER += 1;
        let rect = framebuffer.draw_text(
            240,
            1140,
            format!("{0}", G_COUNTER),
            65,
            mxc_types::REMARKABLE_DARKEST
        );
        let marker = framebuffer.refresh(&rect,
                            update_mode::UPDATE_MODE_PARTIAL,
                            waveform,
                            display_temp::TEMP_USE_MAX,
                            dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                            0, 0);
        framebuffer.wait_refresh_complete(marker);
    }
}

lazy_static! {
    // 0 -> None
    // 1 -> Circles
    // 2 -> Bezier
    static ref DRAW_ON_TOUCH: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
    static ref PREV_WACOM: Arc<Mutex<(i32, i32)>> = Arc::new(Mutex::new((-1, -1)));
}
fn main() {
    env_logger::init();

    // Takes callback functions as arguments
    // They are called with the event and the &mut framebuffer
    let mut app = uix::ApplicationContext::new(
        on_button_press,
        on_wacom_input,
        on_touch_handler,
    );

    // Alternatively we could have called `app.execute_lua("fb.clear()")`
    app.clear(true);

    // A rudimentary way to declare a scene and layout
    app.draw_elements(&vec![
        UIElement::Image {
            img: image::load_from_memory(include_bytes!("../assets/rustlang.bmp")).unwrap(),
            y: 10, x: 900,
            refresh: UIConstraintRefresh::Refresh,

            /* We could have alternatively done this:
               // Create a clickable region for multitouch input and associate it with its handler fn
               app.create_active_region(10, 900, 240, 480, on_touch_rustlogo);
            */
            onclick: Some(uix::ActiveRegionHandler(on_touch_rustlogo)),
        },
        UIElement::Text {
            text: "Available at:".to_owned(),
            y: 650, x: 120,
            scale: 70,
            onclick: None,
            refresh: UIConstraintRefresh::Refresh
        },
        UIElement::Text {
            text: "github.com/canselcik/libremarkable".to_owned(),
            y: 750, x: 100,
            scale: 60,
            onclick: None,
            refresh: UIConstraintRefresh::Refresh
        },
        UIElement::Text {
            text: "Low Latency eInk Display Partial Refresh API".to_owned(),
            y: 350, x: 120,
            scale: 55,
            onclick: None,
            refresh: UIConstraintRefresh::Refresh
        },
        UIElement::Text {
            text: "Physical Button Support".to_owned(),
            y: 470, x: 120,
            scale: 55,
            onclick: None,
            refresh: UIConstraintRefresh::Refresh
        },
        UIElement::Text {
            text: "Capacitive Multitouch Input Support".to_owned(),
            y: 410, x: 120,
            scale: 55,
            onclick: None,
            refresh: UIConstraintRefresh::Refresh
        },
        UIElement::Text {
            text: "Wacom Digitizer Support".to_owned(),
            y: 530, x: 120,
            scale: 55,
            onclick: None,
            refresh: UIConstraintRefresh::RefreshAndWait
        },
    ]);

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
