use libremarkable::framebuffer::cgmath;
use libremarkable::framebuffer::cgmath::EuclideanSpace;
use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::storage;
use libremarkable::framebuffer::PartialRefreshMode;
use libremarkable::framebuffer::{FramebufferDraw, FramebufferIO, FramebufferRefresh};
use libremarkable::image::GenericImage;
use libremarkable::input::{InputDevice, InputEvent};
use libremarkable::ui_extensions::element::{
    UIConstraintRefresh, UIElement, UIElementHandle, UIElementWrapper,
};
use libremarkable::{appctx, battery, image, input};
use libremarkable::{end_bench, start_bench};

#[cfg(feature = "enable-runtime-benchmarking")]
use libremarkable::stopwatch;

use atomic::Atomic;
use chrono::{DateTime, Local};
use log::info;
use once_cell::sync::Lazy;

use std::collections::VecDeque;
use std::fmt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

#[derive(Copy, Clone, PartialEq)]
enum DrawMode {
    Draw(u32),
    Erase(u32),
}
impl DrawMode {
    fn set_size(self, new_size: u32) -> Self {
        match self {
            DrawMode::Draw(_) => DrawMode::Draw(new_size),
            DrawMode::Erase(_) => DrawMode::Erase(new_size),
        }
    }
    fn color_as_string(self) -> String {
        match self {
            DrawMode::Draw(_) => "Black",
            DrawMode::Erase(_) => "White",
        }
        .into()
    }
    fn get_size(self) -> u32 {
        match self {
            DrawMode::Draw(s) => s,
            DrawMode::Erase(s) => s,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum TouchMode {
    OnlyUI,
    Bezier,
    Circles,
    Diamonds,
    FillDiamonds,
}
impl TouchMode {
    fn toggle(self) -> Self {
        match self {
            TouchMode::OnlyUI => TouchMode::Bezier,
            TouchMode::Bezier => TouchMode::Circles,
            TouchMode::Circles => TouchMode::Diamonds,
            TouchMode::Diamonds => TouchMode::FillDiamonds,
            TouchMode::FillDiamonds => TouchMode::OnlyUI,
        }
    }
}

impl fmt::Display for TouchMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mode = match self {
            TouchMode::OnlyUI => "None",
            TouchMode::Bezier => "Bezier",
            TouchMode::Circles => "Circles",
            TouchMode::Diamonds => "Diamonds",
            TouchMode::FillDiamonds => "FDiamonds",
        };
        write!(f, "{}", mode)
    }
}

// This region will have the following size at rest:
//   raw: 5896 kB
//   zstd: 10 kB
const CANVAS_REGION: mxcfb_rect = mxcfb_rect {
    top: 720,
    left: 0,
    height: 1080,
    width: 1404,
};

type PointAndPressure = (cgmath::Point2<f32>, i32);

static G_TOUCH_MODE: Lazy<Atomic<TouchMode>> = Lazy::new(|| Atomic::new(TouchMode::OnlyUI));
static G_DRAW_MODE: Lazy<Atomic<DrawMode>> = Lazy::new(|| Atomic::new(DrawMode::Draw(2)));
static UNPRESS_OBSERVED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static WACOM_IN_RANGE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static WACOM_RUBBER_SIDE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static WACOM_HISTORY: Lazy<Mutex<VecDeque<PointAndPressure>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));
static G_COUNTER: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
static SAVED_CANVAS: Lazy<Mutex<Option<storage::CompressedCanvasState>>> =
    Lazy::new(|| Mutex::new(None));

// ####################
// ## Button Handlers
// ####################

fn on_save_canvas(app: &mut appctx::ApplicationContext<'_>, _element: UIElementHandle) {
    start_bench!(stopwatch, save_canvas);
    let framebuffer = app.get_framebuffer_ref();
    match framebuffer.dump_region(CANVAS_REGION) {
        Err(err) => println!("Failed to dump buffer: {0}", err),
        Ok(buff) => {
            let mut hist = SAVED_CANVAS.lock().unwrap();
            *hist = Some(storage::CompressedCanvasState::new(
                buff.as_slice(),
                CANVAS_REGION.height,
                CANVAS_REGION.width,
            ));
        }
    };
    end_bench!(save_canvas);
}

fn on_zoom_out(app: &mut appctx::ApplicationContext<'_>, _element: UIElementHandle) {
    start_bench!(stopwatch, zoom_out);
    let framebuffer = app.get_framebuffer_ref();
    match framebuffer.dump_region(CANVAS_REGION) {
        Err(err) => println!("Failed to dump buffer: {0}", err),
        Ok(buff) => {
            let resized = image::DynamicImage::ImageRgb8(
                storage::rgbimage_from_u8_slice(
                    CANVAS_REGION.width,
                    CANVAS_REGION.height,
                    buff.as_slice(),
                )
                .unwrap(),
            )
            .resize(
                (CANVAS_REGION.width as f32 / 1.25f32) as u32,
                (CANVAS_REGION.height as f32 / 1.25f32) as u32,
                image::imageops::Nearest,
            );

            // Get a clean image the size of the canvas
            let mut new_image =
                image::DynamicImage::new_rgb8(CANVAS_REGION.width, CANVAS_REGION.height);
            new_image.invert();

            // Copy the resized image into the subimage
            new_image
                .copy_from(&resized, CANVAS_REGION.width / 8, CANVAS_REGION.height / 8)
                .unwrap();

            framebuffer.draw_image(
                new_image.as_rgb8().unwrap(),
                CANVAS_REGION.top_left().cast().unwrap(),
            );
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
    end_bench!(zoom_out);
}

fn on_blur_canvas(app: &mut appctx::ApplicationContext<'_>, _element: UIElementHandle) {
    start_bench!(stopwatch, blur_canvas);
    let framebuffer = app.get_framebuffer_ref();
    match framebuffer.dump_region(CANVAS_REGION) {
        Err(err) => println!("Failed to dump buffer: {0}", err),
        Ok(buff) => {
            let dynamic = image::DynamicImage::ImageRgb8(
                storage::rgbimage_from_u8_slice(
                    CANVAS_REGION.width,
                    CANVAS_REGION.height,
                    buff.as_slice(),
                )
                .unwrap(),
            )
            .blur(0.6f32);

            framebuffer.draw_image(
                dynamic.as_rgb8().unwrap(),
                CANVAS_REGION.top_left().cast().unwrap(),
            );
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
    end_bench!(blur_canvas);
}

fn on_invert_canvas(app: &mut appctx::ApplicationContext<'_>, element: UIElementHandle) {
    start_bench!(stopwatch, invert);
    let framebuffer = app.get_framebuffer_ref();
    match framebuffer.dump_region(CANVAS_REGION) {
        Err(err) => println!("Failed to dump buffer: {0}", err),
        Ok(mut buff) => {
            buff.iter_mut().for_each(|p| {
                *p = !(*p);
            });
            match framebuffer.restore_region(CANVAS_REGION, &buff) {
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
    end_bench!(invert);

    // Invert the draw color as well for more natural UX
    on_toggle_eraser(app, element);
}

fn on_load_canvas(app: &mut appctx::ApplicationContext<'_>, _element: UIElementHandle) {
    start_bench!(stopwatch, load_canvas);
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
    end_bench!(load_canvas);
}

fn on_touch_rustlogo(app: &mut appctx::ApplicationContext<'_>, _element: UIElementHandle) {
    let framebuffer = app.get_framebuffer_ref();
    let new_press_count = {
        let mut v = G_COUNTER.lock().unwrap();
        *v += 1;
        *v
    };

    // First drawing with GC16_FAST to draw it thoroughly and then
    // alternating between DU which has more artifacts but is faster.
    let waveform = if new_press_count % 2 == 0 {
        waveform_mode::WAVEFORM_MODE_DU
    } else {
        waveform_mode::WAVEFORM_MODE_GC16_FAST
    };

    let rect = framebuffer.draw_text(
        cgmath::Point2 {
            x: 1140.0,
            y: 240.0,
        },
        &format!("{0}", new_press_count),
        65.0,
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

fn on_toggle_eraser(app: &mut appctx::ApplicationContext<'_>, _: UIElementHandle) {
    let (new_mode, name) = match G_DRAW_MODE.load(Ordering::Relaxed) {
        DrawMode::Erase(s) => (DrawMode::Draw(s), "Black".to_owned()),
        DrawMode::Draw(s) => (DrawMode::Erase(s), "White".to_owned()),
    };
    G_DRAW_MODE.store(new_mode, Ordering::Relaxed);

    let indicator = app.get_element_by_name("colorIndicator");
    if let UIElement::Text { ref mut text, .. } = indicator.unwrap().write().inner {
        *text = name;
    }
    app.draw_element("colorIndicator");
}

fn on_change_touchdraw_mode(app: &mut appctx::ApplicationContext<'_>, _: UIElementHandle) {
    let new_val = G_TOUCH_MODE.load(Ordering::Relaxed).toggle();
    G_TOUCH_MODE.store(new_val, Ordering::Relaxed);

    let indicator = app.get_element_by_name("touchModeIndicator");
    if let UIElement::Text { ref mut text, .. } = indicator.unwrap().write().inner {
        *text = new_val.to_string();
    }
    // Make sure you aren't trying to draw the element while you are holding a write lock.
    // It doesn't seem to cause a deadlock however it may cause higher lock contention.
    app.draw_element("touchModeIndicator");
}

// ####################
// ## Miscellaneous
// ####################

/// Called on button press on rm2 or left gpio on rm1
fn quick_redraw(app: &mut appctx::ApplicationContext<'_>) {
    app.clear(false);
    app.draw_elements();
}

/// Called on button press on rm2 or middle gpio on rm1
fn full_redraw(app: &mut appctx::ApplicationContext<'_>) {
    app.clear(true);
    app.draw_elements();
}

/// Called on button press (pen can press, too) on rm2 or right gpio on rm1
fn toggle_touch(app: &mut appctx::ApplicationContext<'_>) {
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

    if let Some(ref elem) = app.get_element_by_name("toggleTouch") {
        if let UIElement::Text {
            ref mut text,
            scale: _,
            foreground: _,
            border_px: _,
        } = elem.write().inner
        {
            *text = new_state.to_owned();
        }
    }
    app.draw_element("toggleTouch");
}

fn draw_color_test_rgb(app: &mut appctx::ApplicationContext<'_>, _element: UIElementHandle) {
    let fb = app.get_framebuffer_ref();

    let img_rgb565 = image::load_from_memory(include_bytes!("../assets/colorspace.png")).unwrap();
    fb.draw_image(
        img_rgb565.as_rgb8().unwrap(),
        CANVAS_REGION.top_left().cast().unwrap(),
    );
    fb.partial_refresh(
        &CANVAS_REGION,
        PartialRefreshMode::Wait,
        waveform_mode::WAVEFORM_MODE_GC16,
        display_temp::TEMP_USE_PAPYRUS,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        false,
    );
}

fn change_brush_width(app: &mut appctx::ApplicationContext<'_>, delta: i32) {
    let current = G_DRAW_MODE.load(Ordering::Relaxed);
    let current_size = current.get_size() as i32;
    let proposed_size = current_size + delta;
    let new_size = proposed_size.clamp(1, 99);
    if new_size == current_size {
        return;
    }

    G_DRAW_MODE.store(current.set_size(new_size as u32), Ordering::Relaxed);

    let element = app.get_element_by_name("displaySize").unwrap();
    if let UIElement::Text { ref mut text, .. } = element.write().inner {
        *text = format!("size: {0}", new_size);
    }
    app.draw_element("displaySize");
}

fn loop_update_topbar(app: &mut appctx::ApplicationContext<'_>, millis: u64) {
    let time_label = app.get_element_by_name("time").unwrap();
    let battery_label = app.get_element_by_name("battery").unwrap();
    loop {
        // Get the datetime
        let dt: DateTime<Local> = Local::now();

        if let UIElement::Text { ref mut text, .. } = time_label.write().inner {
            *text = format!("{}", dt.format("%F %r"));
        }

        if let UIElement::Text { ref mut text, .. } = battery_label.write().inner {
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

// ####################
// ## Input Handlers
// ####################

fn on_wacom_input(app: &mut appctx::ApplicationContext<'_>, input: input::WacomEvent) {
    match input {
        input::WacomEvent::Draw {
            position,
            pressure,
            tilt: _,
        } => {
            let mut wacom_stack = WACOM_HISTORY.lock().unwrap();

            // This is so that we can click the buttons outside the canvas region
            // normally meant to be touched with a finger using our stylus
            if !CANVAS_REGION.contains_point(&position.cast().unwrap()) {
                wacom_stack.clear();
                if UNPRESS_OBSERVED.fetch_and(false, Ordering::Relaxed) {
                    let region = app
                        .find_active_region(position.y.round() as u16, position.x.round() as u16);
                    let element = region.map(|(region, _)| region.element.clone());
                    if let Some(element) = element {
                        (region.unwrap().0.handler)(app, element)
                    }
                }
                return;
            }

            let (mut col, mut mult) = match G_DRAW_MODE.load(Ordering::Relaxed) {
                DrawMode::Draw(s) => (color::BLACK, s),
                DrawMode::Erase(s) => (color::WHITE, s * 3),
            };
            if WACOM_RUBBER_SIDE.load(Ordering::Relaxed) {
                col = match col {
                    color::WHITE => color::BLACK,
                    _ => color::WHITE,
                };
                mult = 50; // Rough size of the rubber end
            }

            wacom_stack.push_back((position.cast().unwrap(), i32::from(pressure)));

            while wacom_stack.len() >= 3 {
                let framebuffer = app.get_framebuffer_ref();
                let points = [
                    wacom_stack.pop_front().unwrap(),
                    *wacom_stack.front().unwrap(),
                    *wacom_stack.get(1).unwrap(),
                ];
                let radii: Vec<f32> = points
                    .iter()
                    .map(|point| ((mult as f32 * (point.1 as f32) / 2048.) / 2.0))
                    .collect();
                // calculate control points
                let start_point = points[2].0.midpoint(points[1].0);
                let ctrl_point = points[1].0;
                let end_point = points[1].0.midpoint(points[0].0);
                // calculate diameters
                let start_width = radii[2] + radii[1];
                let ctrl_width = radii[1] * 2.0;
                let end_width = radii[1] + radii[0];
                let rect = framebuffer.draw_dynamic_bezier(
                    (start_point, start_width),
                    (ctrl_point, ctrl_width),
                    (end_point, end_width),
                    10,
                    col,
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
        }
        input::WacomEvent::InstrumentChange { pen, state } => {
            match pen {
                // Whether the pen is in range
                input::WacomPen::ToolPen => {
                    WACOM_IN_RANGE.store(state, Ordering::Relaxed);
                    WACOM_RUBBER_SIDE.store(false, Ordering::Relaxed);
                }
                input::WacomPen::ToolRubber => {
                    WACOM_IN_RANGE.store(state, Ordering::Relaxed);
                    WACOM_RUBBER_SIDE.store(true, Ordering::Relaxed);
                }
                // Whether the pen is actually making contact
                input::WacomPen::Touch => {
                    // Stop drawing when instrument has left the vicinity of the screen
                    if !state {
                        let mut wacom_stack = WACOM_HISTORY.lock().unwrap();
                        wacom_stack.clear();
                    }
                }
                _ => {}
            }
        }
        input::WacomEvent::Hover {
            position: _,
            distance,
            tilt: _,
        } => {
            // If the pen is hovering, don't record its coordinates as the origin of the next line
            if distance > 1 {
                let mut wacom_stack = WACOM_HISTORY.lock().unwrap();
                wacom_stack.clear();
                UNPRESS_OBSERVED.store(true, Ordering::Relaxed);
            }
        }
        _ => {}
    };
}

fn on_touch_handler(app: &mut appctx::ApplicationContext<'_>, input: input::MultitouchEvent) {
    let framebuffer = app.get_framebuffer_ref();
    match input {
        input::MultitouchEvent::Press { finger } | input::MultitouchEvent::Move { finger } => {
            if !CANVAS_REGION.contains_point(&finger.pos.cast().unwrap()) {
                return;
            }
            let rect = match G_TOUCH_MODE.load(Ordering::Relaxed) {
                TouchMode::Bezier => {
                    let position_float = finger.pos.cast().unwrap();
                    let points = [
                        (cgmath::vec2(-40.0, 0.0), 2.5),
                        (cgmath::vec2(40.0, -60.0), 5.5),
                        (cgmath::vec2(0.0, 0.0), 3.5),
                        (cgmath::vec2(-40.0, 60.0), 6.5),
                        (cgmath::vec2(-10.0, 50.0), 5.0),
                        (cgmath::vec2(10.0, 45.0), 4.5),
                        (cgmath::vec2(30.0, 55.0), 3.5),
                        (cgmath::vec2(50.0, 65.0), 3.0),
                        (cgmath::vec2(70.0, 40.0), 0.0),
                    ];
                    let mut rect = mxcfb_rect::invalid();
                    for window in points.windows(3).step_by(2) {
                        rect = rect.merge_rect(&framebuffer.draw_dynamic_bezier(
                            (position_float + window[0].0, window[0].1),
                            (position_float + window[1].0, window[1].1),
                            (position_float + window[2].0, window[2].1),
                            100,
                            color::BLACK,
                        ));
                    }
                    rect
                }
                TouchMode::Circles => {
                    framebuffer.draw_circle(finger.pos.cast().unwrap(), 20, color::BLACK)
                }

                m @ TouchMode::Diamonds | m @ TouchMode::FillDiamonds => {
                    let position_int = finger.pos.cast().unwrap();
                    framebuffer.draw_polygon(
                        &[
                            position_int + cgmath::vec2(-10, 0),
                            position_int + cgmath::vec2(0, 20),
                            position_int + cgmath::vec2(10, 0),
                            position_int + cgmath::vec2(0, -20),
                        ],
                        match m {
                            TouchMode::Diamonds => false,
                            TouchMode::FillDiamonds => true,
                            _ => false,
                        },
                        color::BLACK,
                    )
                }
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

fn on_button_press(app: &mut appctx::ApplicationContext<'_>, input: input::GPIOEvent) {
    let (btn, new_state) = match input {
        input::GPIOEvent::Press { button } => (button, true),
        input::GPIOEvent::Unpress { button } => (button, false),
        _ => return,
    };

    // Ignoring the unpressed event
    if !new_state {
        return;
    }

    // Simple but effective accidental button press filtering
    if WACOM_IN_RANGE.load(Ordering::Relaxed) {
        return;
    }

    match btn {
        input::PhysicalButton::LEFT => quick_redraw(app),
        input::PhysicalButton::MIDDLE => full_redraw(app),
        input::PhysicalButton::RIGHT => toggle_touch(app),

        input::PhysicalButton::POWER => {
            Command::new("systemctl")
                .arg("start")
                .arg("xochitl")
                .spawn()
                .unwrap();
            std::process::exit(0);
        }
        input::PhysicalButton::WAKEUP => {
            println!("WAKEUP button(?) pressed(?)");
        }
    };
}

fn main() {
    env_logger::init();

    // Takes callback functions as arguments
    // They are called with the event and the &mut framebuffer
    let mut app: appctx::ApplicationContext<'_> = appctx::ApplicationContext::default();

    // Alternatively we could have called `app.execute_lua("fb.clear()")`
    app.clear(true);

    // A rudimentary way to declare a scene and layout
    app.add_element(
        "logo",
        UIElementWrapper {
            position: cgmath::Point2 { x: 900, y: 10 },
            refresh: UIConstraintRefresh::Refresh,

            /* We could have alternatively done this:

               // Create a clickable region for multitouch input and associate it with its handler fn
               app.create_active_region(10, 900, 240, 480, on_touch_rustlogo);
            */
            onclick: Some(on_touch_rustlogo),
            inner: UIElement::Image {
                img: image::load_from_memory(include_bytes!("../assets/rustlang.png")).unwrap(),
            },
            ..Default::default()
        },
    );

    // Draw the borders for the canvas region
    app.add_element(
        "canvasRegion",
        UIElementWrapper {
            position: CANVAS_REGION.top_left().cast().unwrap() + cgmath::vec2(0, -2),
            refresh: UIConstraintRefresh::RefreshAndWait,
            onclick: None,
            inner: UIElement::Region {
                size: CANVAS_REGION.size().cast().unwrap() + cgmath::vec2(1, 3),
                border_px: 2,
                border_color: color::BLACK,
            },
            ..Default::default()
        },
    );

    app.add_element(
        "colortest-rgb",
        UIElementWrapper {
            position: cgmath::Point2 { x: 960, y: 300 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(draw_color_test_rgb),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Show RGB Test Image".to_owned(),
                scale: 35.0,
                border_px: 3,
            },
            ..Default::default()
        },
    );

    // Zoom Out Button
    app.add_element(
        "zoomoutButton",
        UIElementWrapper {
            position: cgmath::Point2 { x: 960, y: 370 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_zoom_out),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Zoom Out".to_owned(),
                scale: 45.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    // Blur Toggle
    app.add_element(
        "blurToggle",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1155, y: 370 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_blur_canvas),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Blur".to_owned(),
                scale: 45.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    // Invert Toggle
    app.add_element(
        "invertToggle",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1247, y: 370 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_invert_canvas),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Invert".to_owned(),
                scale: 45.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    // Save/Restore Controls
    app.add_element(
        "saveButton",
        UIElementWrapper {
            position: cgmath::Point2 { x: 960, y: 440 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_save_canvas),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Save".to_owned(),
                scale: 45.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    app.add_element(
        "restoreButton",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1080, y: 440 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_load_canvas),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Load".to_owned(),
                scale: 45.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    // Touch Mode Toggle
    app.add_element(
        "touchMode",
        UIElementWrapper {
            position: cgmath::Point2 { x: 960, y: 510 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_change_touchdraw_mode),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Touch Mode".to_owned(),
                scale: 45.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "touchModeIndicator",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1210, y: 510 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: None,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "None".to_owned(),
                scale: 40.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );

    // Color Mode Toggle
    app.add_element(
        "colorToggle",
        UIElementWrapper {
            position: cgmath::Point2 { x: 960, y: 580 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: Some(on_toggle_eraser),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Draw Color".to_owned(),
                scale: 45.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "colorIndicator",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1210, y: 580 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: None,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: G_DRAW_MODE.load(Ordering::Relaxed).color_as_string(),
                scale: 40.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );

    // Size Controls
    app.add_element(
        "decreaseSizeSkip",
        UIElementWrapper {
            position: cgmath::Point2 { x: 960, y: 670 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|appctx, _| {
                change_brush_width(appctx, -10);
            }),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "--".to_owned(),
                scale: 90.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "decreaseSize",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1030, y: 670 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|appctx, _| {
                change_brush_width(appctx, -1);
            }),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "-".to_owned(),
                scale: 90.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "displaySize",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1080, y: 670 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: format!("size: {0}", G_DRAW_MODE.load(Ordering::Relaxed).get_size()),
                scale: 45.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "increaseSize",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1240, y: 670 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|appctx, _| {
                change_brush_width(appctx, 1);
            }),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "+".to_owned(),
                scale: 60.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "increaseSizeSkip",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1295, y: 670 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|appctx, _| {
                change_brush_width(appctx, 10);
            }),
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "++".to_owned(),
                scale: 60.0,
                border_px: 5,
            },
            ..Default::default()
        },
    );

    app.add_element(
        "exitToXochitl",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 50 },
            refresh: UIConstraintRefresh::Refresh,

            onclick: None,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Press POWER to return to reMarkable".to_owned(),
                scale: 35.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "availAt",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 620 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Available at:".to_owned(),
                scale: 70.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "github",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 690 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "github.com/canselcik/libremarkable".to_owned(),
                scale: 55.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l1",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 350 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Low Latency eInk Display Partial Refresh API".to_owned(),
                scale: 45.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l3",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 400 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Capacitive Multitouch Input Support".to_owned(),
                scale: 45.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l2",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 450 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Physical Button Support".to_owned(),
                scale: 45.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "l4",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 500 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Wacom Digitizer Support".to_owned(),
                scale: 45.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );

    let is_rm_2 = libremarkable::device::CURRENT_DEVICE.model == libremarkable::device::Model::Gen2;

    app.add_element(
        "quickRedraw",
        UIElementWrapper {
            position: cgmath::Point2 { x: 15, y: 1850 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: if is_rm_2 {
                Some(|app, _| quick_redraw(app))
            } else {
                None
            },
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Quick Redraw".to_owned(), // maybe quick redraw for the demo or waveform change?
                scale: 50.0,
                border_px: if is_rm_2 { 5 } else { 0 },
            },
            ..Default::default()
        },
    );
    app.add_element(
        "fullRedraw",
        UIElementWrapper {
            position: cgmath::Point2 { x: 565, y: 1850 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: if is_rm_2 {
                Some(|app, _| full_redraw(app))
            } else {
                None
            },
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Full Redraw".to_owned(),
                scale: 50.0,
                border_px: if is_rm_2 { 5 } else { 0 },
            },
            ..Default::default()
        },
    );
    app.add_element(
        "toggleTouch",
        UIElementWrapper {
            position: cgmath::Point2 { x: 1112, y: 1850 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: if is_rm_2 {
                Some(|app, _| toggle_touch(app))
            } else {
                None
            },
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: "Disable Touch".to_owned(),
                scale: 50.0,
                border_px: if is_rm_2 { 5 } else { 0 },
            },
            ..Default::default()
        },
    );
    // Create the top bar's time and battery labels. We can mutate these later.
    let dt: DateTime<Local> = Local::now();
    app.add_element(
        "battery",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 215 },
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
                scale: 44.0,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "time",
        UIElementWrapper {
            position: cgmath::Point2 { x: 30, y: 150 },
            refresh: UIConstraintRefresh::Refresh,
            inner: UIElement::Text {
                foreground: color::BLACK,
                text: format!("{}", dt.format("%F %r")),
                scale: 75.0,
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

      top = 430;
      left = 570;
      width = 320;
      height = 90;
      borderpx = 3;
      draw_box(top, left, height, width, borderpx, 255);

      -- Draw black text inside the box. Notice the text is bottom aligned.
      fb.draw_text(top+55, left+22, '...also supports Lua', 30, 255);

      -- Update the drawn rect w/ `deep_plot=false` and `wait_for_update_complete=true`
      fb.refresh(top, left, height, width, false, true);
    "#,
    );

    info!("Init complete. Beginning event dispatch...");

    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.start_event_loop(true, true, true, |ctx, evt| match evt {
        InputEvent::WacomEvent { event } => on_wacom_input(ctx, event),
        InputEvent::MultitouchEvent { event } => on_touch_handler(ctx, event),
        InputEvent::GPIO { event } => on_button_press(ctx, event),
        _ => {}
    });
    clock_thread.join().unwrap();
}
