use std;
use std::sync::Arc;
use std::hash::{Hash, Hasher};

use image;

use framebuffer::core;
use framebuffer::common;

pub type ActiveRegionFunction = fn(&mut core::Framebuffer, Arc<UIElementWrapper>);

#[derive(Clone)]
pub struct ActiveRegionHandler {
    pub handler: ActiveRegionFunction,
    pub element: Arc<UIElementWrapper>,
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
    pub name: String,
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
        self.name.hash(state);
    }
}

impl PartialEq for UIElementWrapper {
    fn eq(&self, other: &UIElementWrapper) -> bool {
        self.x == other.x &&
            self.y == other.y &&
            self.name == other.name
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

impl Default for UIElement {
    fn default() -> UIElement { UIElement::Unspecified }
}