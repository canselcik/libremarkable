use std;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::cell::UnsafeCell;
use std::ops::DerefMut;

use std::collections::HashSet;

use image;

use input;
use input::ev;

use rb;
use rb::RB;
use rb::RbConsumer;

use framebuffer::common::*;

use ui_extensions::luaext;
use ui_extensions::element::{UIElementWrapper, UIElement,
                             UIConstraintRefresh, ActiveRegionFunction,
                             ActiveRegionHandler};
use hlua;
use hlua::Lua;

use aabb_quadtree::{QuadTree, geom, ItemId};

use framebuffer::core;
use framebuffer::FramebufferBase;
use framebuffer::FramebufferDraw;
use framebuffer::FramebufferRefresh;

use input::InputEvent;
use input::wacom::WacomEvent;
use input::gpio::GPIOEvent;
use input::multitouch::MultitouchEvent;

pub struct ApplicationContext<'a> {
    framebuffer: Box<core::Framebuffer<'a>>,
    running: AtomicBool,
    lua: UnsafeCell<Lua<'a>>,
    on_button: fn(&mut core::Framebuffer, GPIOEvent),
    on_wacom: fn(&mut core::Framebuffer, WacomEvent),
    on_touch: fn(&mut core::Framebuffer, MultitouchEvent),
    active_regions: QuadTree<ActiveRegionHandler>,
    ui_elements: HashSet<Arc<UIElementWrapper>>,
    yres: u32,
    xres: u32,
}

impl<'a> ApplicationContext<'a> {
    pub fn get_framebuffer_ref(&mut self) -> &'static mut core::Framebuffer<'static> {
        unsafe {
            std::mem::transmute::<_, &'static mut core::Framebuffer<'static>>(self.framebuffer.deref_mut())
        }
    }

    pub fn get_lua_ref(&mut self) -> &'a mut Lua<'static> {
        unsafe {
            std::mem::transmute::<_, &'a mut Lua<'static>>(self.lua.get())
        }
    }

    pub fn get_dimensions(self) -> (u32, u32) {
        (self.yres, self.xres)
    }

    pub fn new(on_button: fn(&mut core::Framebuffer, GPIOEvent),
               on_wacom: fn(&mut core::Framebuffer, WacomEvent),
               on_touch: fn(&mut core::Framebuffer, MultitouchEvent),
    ) -> ApplicationContext<'static> {
        let framebuffer = Box::new(core::Framebuffer::new("/dev/fb0"));
        let yres = framebuffer.var_screen_info.yres;
        let xres = framebuffer.var_screen_info.xres;
        let mut res = ApplicationContext {
            framebuffer,
            xres, yres,
            running: AtomicBool::new(false),
            lua: UnsafeCell::new(Lua::new()),
            on_button,
            on_wacom,
            on_touch,
            ui_elements: HashSet::new(),
            active_regions: QuadTree::default(geom::Rect::from_points(&geom::Point { x: 0.0, y: 0.0 },
                                                                           &geom::Point {
                                                                               x: xres as f32,
                                                                               y: yres as f32
                                                                           })),
        };
        let lua = res.get_lua_ref();

        // Enable all std lib
        lua.openlibs();

        // Reluctantly resort to using a static global to associate the lua context with the
        // one and only framebuffer that's going to be used
        unsafe {
            luaext::G_FB = res.framebuffer.deref_mut() as *mut core::Framebuffer
        };

        let mut nms = lua.empty_array("fb");
        // Clears and refreshes the entire screen
        nms.set("clear", hlua::function0(luaext::lua_clear));

        // Refreshes the provided rectangle. Here we are exposing a predefined set of the
        // flags to the Lua API to simplify its use for building interfaces.
        nms.set("refresh", hlua::function6(luaext::lua_refresh));

        // Draws text with rusttype
        nms.set("draw_text", hlua::function5(luaext::lua_draw_text));

        // Sets the pixel to the u8 color value, does no refresh. Refresh done explicitly via calling `refresh`
        nms.set("set_pixel", hlua::function3(luaext::lua_set_pixel));

        return res;
    }

    pub fn execute_lua(&mut self, code: &str) {
        let lua = self.get_lua_ref();
        match lua.execute::<hlua::AnyLuaValue>(&code) {
            Err(e) => warn!("Error in Lua Context: {:?}", e),
            Ok(_) => {},
        };
    }

    fn display_text(
        &mut self,
        y: usize,
        x: usize,
        scale: usize,
        text: String,
        refresh: UIConstraintRefresh,
        onclick: &Option<ActiveRegionHandler>,
    ) {
        let framebuffer = self.get_framebuffer_ref();
        let draw_area: mxcfb_rect = framebuffer.draw_text(y, x, text, scale, REMARKABLE_DARKEST);
        let marker = match refresh {
            UIConstraintRefresh::Refresh | UIConstraintRefresh::RefreshAndWait => framebuffer.refresh(
                &draw_area,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_GC16_FAST,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                0,
            ),
            _ => return,
        };

        // We need to wait until now because we don't know the size of the active region before we
        // actually go ahead and draw it.
        match onclick {
            &Some(ref handler) => {
                if self.find_active_region(y as u16, x as u16).is_none() {
                    self.create_active_region(draw_area.top as u16,
                                              draw_area.left as u16,
                                              draw_area.height as u16,
                                              draw_area.width as u16,
                                              handler.handler, Arc::clone(&handler.element));
                }
            },
            &None => {},
        }
        match refresh {
            UIConstraintRefresh::RefreshAndWait => framebuffer.wait_refresh_complete(marker),
            _ => {},
        };
    }

    fn display_image(&mut self,
                         img: &image::DynamicImage,
                         y: usize,
                         x: usize,
                         refresh: UIConstraintRefresh,
                         onclick: &Option<ActiveRegionHandler>,
    ) {
        let framebuffer = self.get_framebuffer_ref();
        let draw_area = framebuffer.draw_image(&img, y, x);
        let marker = match refresh {
            UIConstraintRefresh::Refresh | UIConstraintRefresh::RefreshAndWait => framebuffer.refresh(
                &draw_area,
                update_mode::UPDATE_MODE_PARTIAL,
                waveform_mode::WAVEFORM_MODE_GC16_FAST,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                0,
            ),
            _ => return,
        };
        match onclick {
            &Some(ref handler) => {
                if self.find_active_region(y as u16, x as u16).is_none() {
                    self.create_active_region(draw_area.top as u16,
                                              draw_area.left as u16,
                                              draw_area.height as u16,
                                              draw_area.width as u16,
                                              handler.handler, Arc::clone(&handler.element));
                }
            },
            &None => {},
        }
        match refresh {
            UIConstraintRefresh::RefreshAndWait => framebuffer.wait_refresh_complete(marker),
            _ => {},
        };
    }

    pub fn add_element(&mut self, element: Arc<UIElementWrapper>) -> bool {
        // Insert already checks if this is already present in the hashset
        self.ui_elements.insert(element)
    }

    pub fn remove_element(&mut self, element: Arc<UIElementWrapper>) -> bool {
        // If there is an active region, remove it.
        if element.onclick.is_some() {
            self.remove_active_region_at_point(element.y as u16, element.x as u16);
        }
        // Remove the element itself
        self.ui_elements.remove(&element)
    }

    pub fn draw_elements(&mut self) {
        // Cloning here shouldn't be all that costly since it is just a hashset of Arc
        let elems = self.ui_elements.clone();
        for element in elems.iter() {
            let (x, y) = (element.x, element.y);
            let refresh = element.refresh.clone();
            let handler = match element.onclick {
                Some(handler) => Some(ActiveRegionHandler { handler, element: Arc::clone(element) }),
                _ => None,
            };

            match element.inner {
                UIElement::Text{ref text, scale} => {
                    self.display_text(y, x, scale, text.to_string(), refresh, &handler);
                },
                UIElement::Image{ref img} => {
                    self.display_image(&img, y, x, refresh, &handler);
                },
                UIElement::Unspecified => {},
            }
        }
    }

    pub fn clear(&mut self, deep: bool) {
        let framebuffer = self.get_framebuffer_ref();
        let (yres, xres) = (
            framebuffer.var_screen_info.yres,
            framebuffer.var_screen_info.xres,
        );
        framebuffer.clear();

        let (update_mode, waveform_mode) = match deep {
            false => (update_mode::UPDATE_MODE_PARTIAL, waveform_mode::WAVEFORM_MODE_GC16_FAST),
            true  => (update_mode::UPDATE_MODE_FULL, waveform_mode::WAVEFORM_MODE_INIT),
        };

        let marker = framebuffer.refresh(
            &mxcfb_rect {
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
        framebuffer.wait_refresh_complete(marker);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn dispatch_events(&mut self, ringbuffer_size: usize, event_read_chunksize: usize) {
        let mut framebuffer = self.get_framebuffer_ref();

        let ringbuffer= rb::SpscRb::new(ringbuffer_size);
        let producer = ringbuffer.producer();
        let unified =  unsafe {
            std::mem::transmute::<input::UnifiedInputHandler, input::UnifiedInputHandler<'static>>
             (input::UnifiedInputHandler::new(&producer))
        };

        let w: &mut input::UnifiedInputHandler = unsafe { std::mem::transmute_copy(&&unified) };
        let wacom_thread = ev::start_evdev("/dev/input/event0".to_owned(), w);

        let t: &mut input::UnifiedInputHandler = unsafe { std::mem::transmute_copy(&&unified) };
        let touch_thread = ev::start_evdev("/dev/input/event1".to_owned(), t);

        let g: &mut input::UnifiedInputHandler = unsafe { std::mem::transmute_copy(&&unified) };
        let gpio_thread = ev::start_evdev("/dev/input/event2".to_owned(), g);

        // Now we consume the input events;
        let consumer = ringbuffer.consumer();
        let mut buf = vec![InputEvent::Unknown {}; event_read_chunksize];

        self.running.store(true, Ordering::Relaxed);

        let mut last_active_region_gesture_id : i32 = -1;
        while self.running.load(Ordering::Relaxed) {
            let _read = consumer.read_blocking(&mut buf).unwrap();
            for &ev in buf.iter() {
                match ev {
                    InputEvent::GPIO{event} => {
                        (self.on_button)(&mut framebuffer, event);
                    },
                    InputEvent::MultitouchEvent{event} => {
                        // Check for and notify clickable active regions for multitouch events
                        match event {
                            MultitouchEvent::Touch {gesture_seq, finger_id: _, y, x} => {
                                let gseq = gesture_seq as i32;
                                if last_active_region_gesture_id != gseq {
                                    match self.find_active_region(y, x) {
                                        Some((h, _)) => {
                                            (h.handler)(framebuffer, Arc::clone(&h.element));
                                        }
                                        _ => {},
                                    };
                                    last_active_region_gesture_id = gseq;
                                }
                            },
                            _ => {},
                        };
                        (self.on_touch)(&mut framebuffer, event);
                    },
                    InputEvent::WacomEvent{event} => {
                        (self.on_wacom)(&mut framebuffer, event);
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

    fn find_active_region(&self, y: u16, x: u16) -> Option<(&ActiveRegionHandler, ItemId)> {
        let matches = self.active_regions.query(
            geom::Rect::centered_with_radius(&geom::Point{ y: y as f32, x: x as f32 }, 2.0)
        );
        match matches.len() {
            0 => None,
            _ => {
                let res = matches.first().unwrap();
                Some((res.0, res.2.clone()))
            },
        }
    }

    pub fn remove_active_region_at_point(&mut self, y: u16, x: u16) -> bool {
        match self.find_active_region(y, x) {
            Some((_, itemid)) => {
                match self.active_regions.remove(itemid) {
                    Some(_) => true,
                    _ => false,
                }
            }
            _ => false,
        }

    }

    pub fn create_active_region(&mut self, y: u16, x: u16, height: u16, width: u16,
                                handler: ActiveRegionFunction,
                                element: Arc<UIElementWrapper>) {
        self.active_regions.insert_with_box(
            ActiveRegionHandler {
                handler, element,
            },
            geom::Rect::from_points(
                &geom::Point { x: x as f32, y: y as f32 },
                &geom::Point { x: (x+width) as f32, y: (y+height) as f32 }
            )
        );
    }
}