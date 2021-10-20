use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::framebuffer::cgmath;
use crate::framebuffer::common;
use crate::framebuffer::common::{color, mxcfb_rect};
use crate::framebuffer::refresh::PartialRefreshMode;
use crate::framebuffer::FramebufferDraw;
use crate::framebuffer::FramebufferRefresh;

use crate::appctx;

pub type ActiveRegionFunction = fn(&mut appctx::ApplicationContext<'_>, UIElementHandle);

#[derive(Clone)]
pub struct ActiveRegionHandler {
    pub handler: ActiveRegionFunction,
    pub element: UIElementHandle,
}

impl<'a> std::fmt::Debug for ActiveRegionHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0:p}", self)
    }
}

#[derive(Clone, Copy)]
pub enum UIConstraintRefresh {
    NoRefresh,
    Refresh,
    RefreshAndWait,
}

impl Default for UIConstraintRefresh {
    fn default() -> UIConstraintRefresh {
        UIConstraintRefresh::Refresh
    }
}

#[derive(Clone)]
pub struct UIElementHandle(Arc<RwLock<UIElementWrapper>>);

#[derive(Clone)]
pub struct UIElementWrapper {
    pub position: cgmath::Point2<i32>,
    pub refresh: UIConstraintRefresh,
    pub last_drawn_rect: Option<common::mxcfb_rect>,
    pub onclick: Option<ActiveRegionFunction>,
    pub inner: UIElement,
}

impl Default for UIElementWrapper {
    fn default() -> UIElementWrapper {
        UIElementWrapper {
            position: cgmath::Point2 { x: 0, y: 0 },
            refresh: UIConstraintRefresh::default(),
            last_drawn_rect: Option::default(),
            onclick: Option::default(),
            inner: UIElement::default(),
        }
    }
}

impl Hash for UIElementWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.position.x.hash(state);
        self.position.y.hash(state);
    }
}

impl PartialEq for UIElementWrapper {
    fn eq(&self, other: &UIElementWrapper) -> bool {
        self.position == other.position
    }
}

impl Eq for UIElementWrapper {}

#[derive(Clone)]
pub enum UIElement {
    Text {
        text: String,
        scale: f32,
        foreground: color,
        border_px: u32,
    },
    Image {
        img: image::DynamicImage,
    },
    Region {
        size: cgmath::Vector2<u32>,
        border_color: color,
        border_px: u32,
    },
    Unspecified,
}

impl UIElementHandle {
    pub fn read(&self) -> RwLockReadGuard<'_, UIElementWrapper> {
        self.0.read().unwrap()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, UIElementWrapper> {
        self.0.write().unwrap()
    }

    pub fn new(elem: UIElementWrapper) -> UIElementHandle {
        UIElementHandle(Arc::new(RwLock::new(elem)))
    }
}

impl UIElementWrapper {
    pub fn draw(
        &mut self,
        app: &mut appctx::ApplicationContext<'_>,
        handler: &Option<ActiveRegionHandler>,
    ) {
        let refresh = self.refresh;
        let framebuffer = app.get_framebuffer_ref();

        let old_filled_rect = match self.last_drawn_rect {
            Some(rect) => {
                // Clear the background on the last occupied region
                framebuffer.fill_rect(rect.top_left().cast().unwrap(), rect.size(), color::WHITE);

                // We have filled the old_filled_rect, now we need to also refresh that but if
                // only if it isn't at the same spot. Otherwise we will be refreshing it for no
                // reason and showing a blank frame. There is of course still a caveat since we don't
                // know the dimensions of a drawn text before it is actually drawn.
                // TODO: Take care of the point above ^
                if rect.top_left() != self.position.cast().unwrap() {
                    framebuffer.partial_refresh(
                        &rect,
                        PartialRefreshMode::Wait,
                        common::waveform_mode::WAVEFORM_MODE_DU,
                        common::display_temp::TEMP_USE_REMARKABLE_DRAW,
                        common::dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                        0,
                        false,
                    );
                }

                rect
            }
            None => mxcfb_rect::invalid(),
        };

        // TODO: Move this to inside the app and then have it call the UIElement's draw
        // TODO: Also perhaps make border_padding configurable
        let rect = match self.inner {
            UIElement::Text {
                ref text,
                scale,
                foreground,
                border_px,
            } => app.display_text(
                self.position.cast().unwrap(),
                foreground,
                scale,
                border_px as u32,
                8,
                text,
                refresh,
            ),
            UIElement::Image { ref img } => {
                app.display_image(img, self.position.cast().unwrap(), refresh)
            }
            UIElement::Region {
                size,
                border_color,
                border_px,
            } => app.display_rect(
                self.position.cast().unwrap(),
                size.cast().unwrap(),
                border_px,
                border_color,
                refresh,
            ),
            UIElement::Unspecified => return,
        };

        // If no changes, no need to change the active region
        if old_filled_rect != rect {
            if let Some(ref h) = handler {
                if old_filled_rect != mxcfb_rect::invalid() {
                    app.remove_active_region_at_point(
                        old_filled_rect.top as u16,
                        old_filled_rect.left as u16,
                    );
                }

                if app
                    .find_active_region(self.position.y as u16, self.position.x as u16)
                    .is_none()
                {
                    app.create_active_region(
                        rect.top as u16,
                        rect.left as u16,
                        rect.height as u16,
                        rect.width as u16,
                        h.handler,
                        h.element.clone(),
                    );
                }
            }
        }

        if let Some(last_rect) = self.last_drawn_rect {
            if last_rect != rect {
                framebuffer.partial_refresh(
                    &last_rect,
                    PartialRefreshMode::Async,
                    common::waveform_mode::WAVEFORM_MODE_DU,
                    common::display_temp::TEMP_USE_REMARKABLE_DRAW,
                    common::dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    false,
                );
            }
        }

        // We need to wait until now because we don't know the size of the active region before we
        // actually go ahead and draw it.
        self.last_drawn_rect = Some(rect);
    }
}

impl Default for UIElement {
    fn default() -> UIElement {
        UIElement::Unspecified
    }
}
