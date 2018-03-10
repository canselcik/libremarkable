use std;
use image;
use fb;
use ev;
use rb;
use rb::RB;
use mxc_types;
use mxc_types::waveform_mode;
use mxc_types::dither_mode;
use mxc_types::update_mode;
use mxc_types::display_temp;
use std::time::Duration;
use rb::RbConsumer;
use unifiedinput;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub enum UIConstraintRefresh {
    NoRefresh,
    Refresh,
    RefreshAndWait
}

#[derive(Clone)]
pub enum UIElement {
    Text {
        text: String,
        scale: usize,
        y: usize,
        x: usize,
        refresh: UIConstraintRefresh,
    },
    Image {
        img: image::DynamicImage,
        y: usize,
        x: usize,
        refresh: UIConstraintRefresh,
    }
}

pub struct ApplicationContext<'a> {
    framebuffer: fb::Framebuffer<'a>,
    running: AtomicBool,
    on_button: fn(&mut fb::Framebuffer, unifiedinput::GPIOEvent),
    on_wacom: fn(&mut fb::Framebuffer, unifiedinput::WacomEvent),
    on_touch: fn(&mut fb::Framebuffer, unifiedinput::MultitouchEvent),
}

impl<'a> ApplicationContext<'a> {
    pub fn get_framebuffer(&mut self) -> &'a mut fb::Framebuffer<'static> {
        unsafe {
            std::mem::transmute_copy(&&self.framebuffer)
        }
    }

    pub fn new(on_button: fn(&mut fb::Framebuffer, unifiedinput::GPIOEvent),
               on_wacom: fn(&mut fb::Framebuffer, unifiedinput::WacomEvent),
               on_touch: fn(&mut fb::Framebuffer, unifiedinput::MultitouchEvent),
    ) -> ApplicationContext<'static> {

        ApplicationContext {
            framebuffer: fb::Framebuffer::new("/dev/fb0"),
            running: AtomicBool::new(false),
            on_button,
            on_wacom,
            on_touch,
        }
    }

    pub fn display_text(
        &mut self,
        y: usize,
        x: usize,
        scale: usize,
        text: String,
        refresh: UIConstraintRefresh,
    ) {
        let draw_area: mxc_types::mxcfb_rect =
            self.framebuffer.draw_text(y, x, text, scale, mxc_types::REMARKABLE_DARKEST);
        let marker = match refresh {
            UIConstraintRefresh::Refresh | UIConstraintRefresh::RefreshAndWait => self.framebuffer.refresh(
                draw_area,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_GC16_FAST,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                0,
            ),
            _ => return,
        };
        match refresh {
            UIConstraintRefresh::RefreshAndWait => self.framebuffer.wait_refresh_complete(marker),
            _ => {},
        };
    }

    pub fn display_image(&mut self, img: &image::DynamicImage, y: usize, x: usize, refresh: UIConstraintRefresh) {
        let rect = self.framebuffer.draw_image(&img, y, x);
        let marker = match refresh {
            UIConstraintRefresh::Refresh | UIConstraintRefresh::RefreshAndWait => self.framebuffer.refresh(
                rect,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_GC16_FAST,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                0,
            ),
            _ => return,
        };
        match refresh {
            UIConstraintRefresh::RefreshAndWait => self.framebuffer.wait_refresh_complete(marker),
            _ => {},
        };
    }

    pub fn draw_elements(&mut self, elements: &Vec<UIElement>) {
        for element in elements.iter() {
            match element {
                &UIElement::Text{ref text, y, x, scale, ref refresh} => {
                    self.display_text(y, x, scale, text.to_string(), refresh.clone())
                },
                &UIElement::Image{ref img, y, x, ref refresh} => {
                    self.display_image(&img, y, x, refresh.clone())
                },
            }
        }
    }

    pub fn clear(&mut self, deep: bool) {
        let (yres, xres) = (
            self.framebuffer.var_screen_info.yres,
            self.framebuffer.var_screen_info.xres,
        );
        self.framebuffer.clear();

        let (update_mode, waveform_mode) = match deep {
            false => (update_mode::UPDATE_MODE_PARTIAL, waveform_mode::WAVEFORM_MODE_GC16_FAST),
            true  => (update_mode::UPDATE_MODE_FULL, waveform_mode::WAVEFORM_MODE_INIT),
        };

        let marker = self.framebuffer.refresh(
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
        match deep {
            false => self.framebuffer.wait_refresh_complete(marker),
            true  => std::thread::sleep(Duration::from_millis(150)),
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn dispatch_events(&mut self, ringbuffer_size: usize, event_read_chunksize: usize) {
        let ringbuffer= rb::SpscRb::new(ringbuffer_size);
        let producer = ringbuffer.producer();
        let unified =  unsafe {
            std::mem::transmute::<unifiedinput::UnifiedInputHandler, unifiedinput::UnifiedInputHandler<'static>>
             (unifiedinput::UnifiedInputHandler::new(false, &producer))
        };

        let w: &mut unifiedinput::UnifiedInputHandler = unsafe { std::mem::transmute_copy(&&unified) };
        let wacom_thread = ev::start_evdev("/dev/input/event0".to_owned(), w);

        let t: &mut unifiedinput::UnifiedInputHandler = unsafe { std::mem::transmute_copy(&&unified) };
        let touch_thread = ev::start_evdev("/dev/input/event1".to_owned(), t);

        let g: &mut unifiedinput::UnifiedInputHandler = unsafe { std::mem::transmute_copy(&&unified) };
        let gpio_thread = ev::start_evdev("/dev/input/event2".to_owned(), g);

        // Now we consume the input events;
        let consumer = ringbuffer.consumer();
        let mut buf = vec![unifiedinput::InputEvent::Unknown {}; event_read_chunksize];

        self.running.store(true, Ordering::Relaxed);
        while self.running.load(Ordering::Relaxed) {
            let _read = consumer.read_blocking(&mut buf).unwrap();
            for &ev in buf.iter() {
                match ev {
                    unifiedinput::InputEvent::GPIO{event} => {
                        (self.on_button)(&mut self.framebuffer, event);
                    },
                    unifiedinput::InputEvent::MultitouchEvent{event} => {
                        (self.on_touch)(&mut self.framebuffer, event);
                    },
                    unifiedinput::InputEvent::WacomEvent{event} => {
                        (self.on_wacom)(&mut self.framebuffer, event);
                    },
                    _ => {},
                }
            }
        }

        // Wait for all threads to join
        gpio_thread.join().unwrap();
        wacom_thread.join().unwrap();
        touch_thread.join().unwrap();
    }
}