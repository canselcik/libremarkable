use std;
use std::cell::UnsafeCell;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use std::collections::HashMap;

use image;
use input::ev;

use framebuffer::common::*;

use hlua;
use hlua::Lua;
use ui_extensions::element::{
    ActiveRegionFunction, ActiveRegionHandler, UIConstraintRefresh, UIElementHandle,
    UIElementWrapper,
};
use ui_extensions::luaext;

use aabb_quadtree::{geom, ItemId, QuadTree};

use framebuffer::cgmath;
use framebuffer::core;
use framebuffer::refresh::PartialRefreshMode;
use framebuffer::FramebufferBase;
use framebuffer::FramebufferDraw;
use framebuffer::FramebufferRefresh;

use input::gpio::GPIOEvent;
use input::multitouch::MultitouchEvent;
use input::wacom::WacomEvent;
use input::{InputDevice, InputEvent};

#[cfg(feature = "enable-runtime-benchmarking")]
use stopwatch;

unsafe impl<'a> Send for ApplicationContext<'a> {}
unsafe impl<'a> Sync for ApplicationContext<'a> {}

pub struct ApplicationContext<'a> {
    framebuffer: Box<core::Framebuffer<'a>>,
    yres: u32,
    xres: u32,

    running: AtomicBool,

    lua: UnsafeCell<Lua<'a>>,

    input_tx: std::sync::mpsc::Sender<InputEvent>,
    input_rx: std::sync::mpsc::Receiver<InputEvent>,

    button_ctx: RwLock<Option<ev::EvDevContext>>,
    on_button: fn(&mut ApplicationContext, GPIOEvent),

    wacom_ctx: RwLock<Option<ev::EvDevContext>>,
    on_wacom: fn(&mut ApplicationContext, WacomEvent),

    touch_ctx: RwLock<Option<ev::EvDevContext>>,
    on_touch: fn(&mut ApplicationContext, MultitouchEvent),

    active_regions: QuadTree<ActiveRegionHandler>,
    ui_elements: HashMap<String, UIElementHandle>,
}

impl<'a> ApplicationContext<'a> {
    pub fn get_framebuffer_ref(&mut self) -> &'static mut core::Framebuffer<'static> {
        unsafe {
            std::mem::transmute::<_, &'static mut core::Framebuffer<'static>>(
                self.framebuffer.deref_mut(),
            )
        }
    }

    /// Perhaps this is bad practice but we know that the ApplicationContext,
    /// just like the Framebuffer will have a static lifetime. We are doing this
    /// so that we can have the event handlers call into the ApplicationContext.
    pub fn upgrade_ref(&mut self) -> &'static mut ApplicationContext<'static> {
        unsafe { std::mem::transmute(self) }
    }

    pub fn get_lua_ref(&mut self) -> &'a mut Lua<'static> {
        unsafe { std::mem::transmute::<_, &'a mut Lua<'static>>(self.lua.get()) }
    }

    pub fn get_dimensions(&self) -> (u32, u32) {
        (self.yres, self.xres)
    }

    pub fn new(
        on_button: fn(&mut ApplicationContext, GPIOEvent),
        on_wacom: fn(&mut ApplicationContext, WacomEvent),
        on_touch: fn(&mut ApplicationContext, MultitouchEvent),
    ) -> ApplicationContext<'static> {
        let framebuffer = box core::Framebuffer::new("/dev/fb0");
        let yres = framebuffer.var_screen_info.yres;
        let xres = framebuffer.var_screen_info.xres;

        let (input_tx, input_rx) = std::sync::mpsc::channel();
        let mut res = ApplicationContext {
            wacom_ctx: RwLock::new(None),
            button_ctx: RwLock::new(None),
            touch_ctx: RwLock::new(None),
            framebuffer,
            xres,
            yres,
            running: AtomicBool::new(false),
            lua: UnsafeCell::new(Lua::new()),
            input_rx,
            input_tx,
            on_button,
            on_wacom,
            on_touch,
            ui_elements: HashMap::new(),
            active_regions: QuadTree::default(geom::Rect::from_points(
                &geom::Point { x: 0.0, y: 0.0 },
                &geom::Point {
                    x: xres as f32,
                    y: yres as f32,
                },
            )),
        };
        let lua = res.get_lua_ref();

        // Enable all std lib
        lua.openlibs();

        // Reluctantly resort to using a static global to associate the lua context with the
        // one and only framebuffer that's going to be used
        unsafe { luaext::G_FB = res.framebuffer.deref_mut() as *mut core::Framebuffer };

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

        res
    }

    pub fn execute_lua(&mut self, code: &str) {
        let lua = self.get_lua_ref();
        if let Err(e) = lua.execute::<hlua::AnyLuaValue>(&code) {
            warn!("Error in Lua Context: {:?}", e);
        }
    }

    pub fn display_text(
        &mut self,
        position: cgmath::Point2<f32>,
        c: color,
        scale: f32,
        border_px: u32,
        border_padding: u32,
        text: String,
        refresh: UIConstraintRefresh,
    ) -> mxcfb_rect {
        let framebuffer = self.get_framebuffer_ref();
        let mut draw_area: mxcfb_rect = framebuffer.draw_text(position, text, scale, c, false);

        // Draw the border if border_px is set to a non-default value
        if border_px > 0 {
            draw_area = draw_area.expand(border_padding);
            framebuffer.draw_rect(
                draw_area.top_left().cast().unwrap(),
                draw_area.size(),
                border_px,
                c,
            );
        }

        let marker = match refresh {
            UIConstraintRefresh::Refresh | UIConstraintRefresh::RefreshAndWait => framebuffer
                .partial_refresh(
                    &draw_area,
                    PartialRefreshMode::Async,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    false,
                ),
            _ => return draw_area,
        };

        if let UIConstraintRefresh::RefreshAndWait = refresh {
            framebuffer.wait_refresh_complete(marker);
        }
        draw_area.expand(border_px)
    }

    pub fn display_rect(
        &mut self,
        position: cgmath::Point2<i32>,
        size: cgmath::Vector2<u32>,
        border_px: u32,
        border_color: color,
        refresh: UIConstraintRefresh,
    ) -> mxcfb_rect {
        let framebuffer = self.get_framebuffer_ref();

        framebuffer.draw_rect(position, size, border_px as u32, border_color);
        let draw_area = mxcfb_rect::from(position.cast().unwrap(), size);
        let marker = match refresh {
            UIConstraintRefresh::Refresh | UIConstraintRefresh::RefreshAndWait => framebuffer
                .partial_refresh(
                    &draw_area,
                    PartialRefreshMode::Async,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    false,
                ),
            _ => return draw_area,
        };

        if let UIConstraintRefresh::RefreshAndWait = refresh {
            framebuffer.wait_refresh_complete(marker);
        }
        draw_area
    }

    pub fn display_image(
        &mut self,
        img: &image::DynamicImage,
        position: cgmath::Point2<i32>,
        refresh: UIConstraintRefresh,
    ) -> mxcfb_rect {
        let framebuffer = self.get_framebuffer_ref();
        let draw_area = match img {
            image::DynamicImage::ImageRgb8(ref rgb) => framebuffer.draw_image(rgb, position),
            other => framebuffer.draw_image(&other.to_rgb(), position),
        };
        let marker = match refresh {
            UIConstraintRefresh::Refresh | UIConstraintRefresh::RefreshAndWait => framebuffer
                .partial_refresh(
                    &draw_area,
                    PartialRefreshMode::Async,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    false,
                ),
            _ => return draw_area,
        };

        if let UIConstraintRefresh::RefreshAndWait = refresh {
            framebuffer.wait_refresh_complete(marker);
        }
        draw_area
    }

    pub fn add_element(
        &mut self,
        name: &str,
        element: UIElementWrapper,
    ) -> Option<UIElementHandle> {
        if self.ui_elements.contains_key(name) {
            return None;
        }

        let elem = UIElementHandle::new(element);
        self.ui_elements.insert(name.to_owned(), elem.clone());
        Some(elem)
    }

    pub fn remove_element(&mut self, name: &str) -> bool {
        self.ui_elements.remove(name).is_some()
    }

    pub fn remove_elements(&mut self) {
        self.ui_elements.clear();
    }

    pub fn draw_element(&mut self, name: &str) -> bool {
        let appref = self.upgrade_ref();
        match self.ui_elements.get(name) {
            None => false,
            Some(element) => {
                let h = element.read().onclick;
                let handler = match h {
                    Some(handler) => Some(ActiveRegionHandler {
                        handler,
                        element: element.clone(),
                    }),
                    _ => None,
                };
                element.write().draw(appref, &handler);
                true
            }
        }
    }

    pub fn get_element_by_name(&mut self, name: &str) -> Option<UIElementHandle> {
        self.ui_elements.get(name).cloned()
    }

    pub fn draw_elements(&mut self) {
        start_bench!(stopwatch, draw_elements);
        let mut elems: std::vec::Vec<UIElementHandle> = self
            .ui_elements
            .iter()
            .map(|(_key, value)| value.clone())
            .collect();

        for element in &mut elems {
            let h = element.read().onclick;
            let handler = match h {
                Some(handler) => Some(ActiveRegionHandler {
                    handler,
                    element: element.clone(),
                }),
                _ => None,
            };
            element.write().draw(self, &handler);
        }
        end_bench!(draw_elements);
    }

    /// Briefly flash the element's `last_drawn_rect`
    pub fn flash_element(&mut self, name: &str) {
        let framebuffer = self.get_framebuffer_ref();
        if let Some(locked_element) = self.get_element_by_name(name) {
            let mut element = locked_element.write();
            if let Some(rect) = element.last_drawn_rect {
                framebuffer.fill_rect(rect.top_left().cast().unwrap(), rect.size(), color::BLACK);
                framebuffer.partial_refresh(
                    &rect,
                    PartialRefreshMode::Wait,
                    waveform_mode::WAVEFORM_MODE_DU,
                    display_temp::TEMP_USE_AMBIENT,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    false,
                );

                // We can pass None as the `handler` here as we know this flashing is not
                // changing the positioning of the `UIElementWrapper`.
                element.draw(self, &None);
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

        if deep {
            framebuffer.full_refresh(
                waveform_mode::WAVEFORM_MODE_INIT,
                display_temp::TEMP_USE_AMBIENT,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                true,
            );
        } else {
            framebuffer.partial_refresh(
                &mxcfb_rect {
                    top: 0,
                    left: 0,
                    height: yres,
                    width: xres,
                },
                PartialRefreshMode::Wait,
                waveform_mode::WAVEFORM_MODE_GC16_FAST,
                display_temp::TEMP_USE_AMBIENT,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                false,
            );
        }
    }

    /// Sets an atomic flag to disable event dispatch. Exiting event dispatch loop will cause
    /// dispatch_events(..) function to reach completion.
    pub fn stop(&mut self) {
        // Deactivate the input devices even though they may not be active.
        // This will stop the production of the input events.
        self.deactivate_input_device(InputDevice::Multitouch);
        self.deactivate_input_device(InputDevice::GPIO);
        self.deactivate_input_device(InputDevice::Wacom);

        // This will make us stop consuming and dispatching the InputEvents.
        self.running.store(false, Ordering::Relaxed);
    }

    /// Returns true if the device is now signalled to be disabled.
    /// If it was disabled prior to calling this function, this function
    /// will immediately return `true`.
    ///
    /// This function does not block until teardown is completed.
    /// It will simply return after signalling the epoller, which is
    /// guaranteed to exit completely, and can be verified by
    /// calling `exited()`.
    pub fn deactivate_input_device(&mut self, t: InputDevice) -> bool {
        // Return true if already disabled
        if !self.is_input_device_active(t) {
            return true;
        }

        // Now we know that the device is active, we can move the context out of
        // the option and stop it.
        let mut dev = match t {
            InputDevice::Wacom => self.wacom_ctx.write().unwrap(),
            InputDevice::Multitouch => self.touch_ctx.write().unwrap(),
            InputDevice::GPIO => self.button_ctx.write().unwrap(),
            _ => return false,
        };

        let mut unwrapped = dev.take().unwrap();
        unwrapped.stop();

        true
    }

    /// Returns true if the device is now enabled. If it was enabled prior
    /// to calling this function, this function will return `true`.
    pub fn activate_input_device(&mut self, t: InputDevice) -> bool {
        // Return true if already enabled
        if self.is_input_device_active(t) {
            return true;
        }

        // Now we know it isn't active, let's create and spawn
        // the producer thread
        let mut dev = match t {
            InputDevice::Wacom => self.wacom_ctx.write().unwrap(),
            InputDevice::Multitouch => self.touch_ctx.write().unwrap(),
            InputDevice::GPIO => self.button_ctx.write().unwrap(),
            _ => return false,
        };

        *dev = Some(ev::EvDevContext::new(t, self.input_tx.clone()));
        match dev.as_mut() {
            Some(ref mut device) => {
                device.start();
                true
            }
            None => false,
        }
    }

    /// Returns true if the given `InputDevice` is active, as in
    /// there is an `EvDevContext` for it and that context has a
    /// currently running `epoll` thread
    pub fn is_input_device_active(&self, t: InputDevice) -> bool {
        let ctx = match t {
            InputDevice::Unknown => return false,
            InputDevice::GPIO => self.button_ctx.read().unwrap(),
            InputDevice::Multitouch => self.touch_ctx.read().unwrap(),
            InputDevice::Wacom => self.wacom_ctx.read().unwrap(),
        };
        match *ctx {
            Some(ref c) => !c.exited() && !c.exit_requested(),
            None => false,
        }
    }

    pub fn event_receiver(&self) -> &std::sync::mpsc::Receiver<InputEvent> {
        &self.input_rx
    }

    pub fn dispatch_events(
        &mut self,
        activate_wacom: bool,
        activate_multitouch: bool,
        activate_buttons: bool,
    ) {
        let appref = self.upgrade_ref();

        if activate_wacom {
            self.activate_input_device(InputDevice::Wacom);
        }
        if activate_multitouch {
            self.activate_input_device(InputDevice::Multitouch);
        }
        if activate_buttons {
            self.activate_input_device(InputDevice::GPIO);
        }

        // Now we consume the input events
        self.running.store(true, Ordering::Relaxed);

        let mut last_active_region_gesture_id: i32 = -1;
        while self.running.load(Ordering::Relaxed) {
            match self.input_rx.recv() {
                Err(e) => println!("Error in input event consumer: {0}", e),
                Ok(event) => match event {
                    InputEvent::GPIO { event } => {
                        (self.on_button)(appref, event);
                    },
                    InputEvent::MultitouchEvent { event } => {
                        // Check for and notify clickable active regions for multitouch events
                        if let MultitouchEvent::Press { finger } | MultitouchEvent::Move { finger } = event
                        {
                            let gseq = i32::from(finger.tracking_id);
                            if last_active_region_gesture_id != gseq {
                                if let Some((h, _)) =
                                    self.find_active_region(finger.pos.y, finger.pos.x)
                                {
                                    (h.handler)(appref, h.element.clone());
                                }
                                last_active_region_gesture_id = gseq;
                            }
                        }
                        (self.on_touch)(appref, event);
                    },
                    InputEvent::WacomEvent { event } => {
                        (self.on_wacom)(appref, event);
                    },
                    _ => {}
                },
            };
        }
    }

    pub fn handle_event(&mut self, event: InputEvent) {
        let appref = self.upgrade_ref();

        // Now we consume the input events
        self.running.store(true, Ordering::Relaxed);

        let mut last_active_region_gesture_id: i32 = -1;
        if self.running.load(Ordering::Relaxed) {
            match event {
                InputEvent::GPIO { event } => {
                    (self.on_button)(appref, event);
                },
                InputEvent::MultitouchEvent { event } => {
                    // Check for and notify clickable active regions for multitouch events
                    if let MultitouchEvent::Press { finger } | MultitouchEvent::Move { finger } = event
                    {
                        let gseq = i32::from(finger.tracking_id);
                        if last_active_region_gesture_id != gseq {
                            if let Some((h, _)) =
                                self.find_active_region(finger.pos.y, finger.pos.x)
                            {
                                (h.handler)(appref, h.element.clone());
                            }
                            last_active_region_gesture_id = gseq;
                        }
                    }
                    (self.on_touch)(appref, event);
                },
                InputEvent::WacomEvent { event } => {
                    (self.on_wacom)(appref, event);
                },
                _ => {}
            }
        }
    }

    pub fn find_active_region(&self, y: u16, x: u16) -> Option<(&ActiveRegionHandler, ItemId)> {
        let matches = self.active_regions.query(geom::Rect::centered_with_radius(
            &geom::Point {
                y: f32::from(y),
                x: f32::from(x),
            },
            2.0,
        ));
        matches.first().map(|res| { (res.0, res.2) })
    }

    pub fn remove_active_region_at_point(&mut self, y: u16, x: u16) -> bool {
        match self.find_active_region(y, x) {
            Some((_, itemid)) => self.active_regions.remove(itemid).is_some(),
            _ => false,
        }
    }

    pub fn create_active_region(
        &mut self,
        y: u16,
        x: u16,
        height: u16,
        width: u16,
        handler: ActiveRegionFunction,
        element: UIElementHandle,
    ) {
        self.active_regions.insert_with_box(
            ActiveRegionHandler { handler, element },
            geom::Rect::from_points(
                &geom::Point {
                    x: f32::from(x),
                    y: f32::from(y),
                },
                &geom::Point {
                    x: f32::from(x + width),
                    y: f32::from(y + height),
                },
            ),
        );
    }
}
