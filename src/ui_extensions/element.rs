use std;
use std::sync::{Arc, RwLock};
use std::hash::{Hash, Hasher};

use image;

use framebuffer::core;
use framebuffer::common;
use framebuffer::FramebufferDraw;
use framebuffer::common::REMARKABLE_BRIGHTEST;

use appctx;

pub type ActiveRegionFunction = fn(&mut core::Framebuffer, Arc<RwLock<UIElementWrapper>>);

#[derive(Clone)]
pub struct ActiveRegionHandler {
    pub handler: ActiveRegionFunction,
    pub element: Arc<RwLock<UIElementWrapper>>,
}

impl<'a> std::fmt::Debug for ActiveRegionHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{0:p}", self)
    }
}



#[derive(Clone)]
pub enum UIConstraintRefresh {
    NoRefresh,
    Refresh,
    RefreshAndWait
}

impl Default for UIConstraintRefresh {
    fn default() -> UIConstraintRefresh { UIConstraintRefresh::Refresh }
}


#[derive(Clone, Default)]
pub struct UIElementWrapper {
    pub y: usize,
    pub x: usize,
    pub refresh: UIConstraintRefresh,
    pub last_drawn_rect: Option<common::mxcfb_rect>,
    pub onclick: Option<ActiveRegionFunction>,
    pub inner: UIElement,
}

impl Hash for UIElementWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl PartialEq for UIElementWrapper {
    fn eq(&self, other: &UIElementWrapper) -> bool {
        self.x == other.x &&
            self.y == other.y
    }
}

impl Eq for UIElementWrapper {}



#[derive(Clone)]
pub enum UIElement {
    Text {
        text: String,
        scale: usize,
    },
    Image {
        img: image::DynamicImage,
    },
    Unspecified,
}

impl UIElementWrapper {
    pub fn draw(&mut self, app: &mut appctx::ApplicationContext, handler: Option<ActiveRegionHandler>) {
        let (x, y) = (self.x, self.y);
        let refresh = self.refresh.clone();
        let framebuffer = app.get_framebuffer_ref();

        match self.last_drawn_rect {
            Some(rect) => {
                // Clear the background on the last occupied region
                framebuffer.fill_rect(rect.top as usize,
                                      rect.left as usize,
                                      rect.height as usize,
                                      rect.width as usize,
                                      REMARKABLE_BRIGHTEST);
            },
            None => {},
        }

        match self.inner {
            UIElement::Text{ref text, scale} => {
                self.last_drawn_rect = Some(app.display_text(y, x,
                                                        scale,
                                                        text.to_string(),
                                                        refresh,
                                                        &handler));
            },
            UIElement::Image{ref img} => {
                self.last_drawn_rect = Some(app.display_image(&img, y, x, refresh, &handler));
            },
            UIElement::Unspecified => {},
        };
    }
}

impl Default for UIElement {
    fn default() -> UIElement { UIElement::Unspecified }
}