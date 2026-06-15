#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
// mxcfb.h defines one ioctl op that overflows an int, which is the type used in musl.
#![allow(overflowing_literals)]

include!(concat!(env!("OUT_DIR"), "/mxcfb.rs"));

impl ::std::default::Default for mxcfb_rect {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl PartialEq for mxcfb_rect {
    fn eq(&self, other: &Self) -> bool {
        self.left == other.left
            && self.top == other.top
            && self.width == other.width
            && self.height == other.height
    }
}

impl mxcfb_rect {
    pub fn top_left(&self) -> cgmath::Point2<u32> {
        cgmath::Point2 {
            x: self.left,
            y: self.top,
        }
    }
    pub fn size(&self) -> cgmath::Vector2<u32> {
        cgmath::Vector2 {
            x: self.width,
            y: self.height,
        }
    }
    pub fn from(pos: cgmath::Point2<u32>, size: cgmath::Vector2<u32>) -> mxcfb_rect {
        mxcfb_rect {
            top: pos.y,
            left: pos.x,
            height: size.y,
            width: size.x,
        }
    }

    pub fn invalid() -> Self {
        mxcfb_rect {
            top: 9999,
            left: 9999,
            height: 0,
            width: 0,
        }
    }

    pub fn contains_point(&self, p: &cgmath::Point2<u32>) -> bool {
        !(p.x < self.left
            || p.x > (self.left + self.width)
            || p.y < self.top
            || p.y > (self.top + self.height))
    }

    pub fn contains_rect(&self, rect: &mxcfb_rect) -> bool {
        self.contains_point(&cgmath::Point2 {
            x: rect.left,
            y: rect.top,
        }) && self.contains_point(&cgmath::Point2 {
            x: rect.left + rect.width,
            y: rect.top + rect.height,
        })
    }

    pub fn merge_pixel(&self, p: &cgmath::Point2<u32>) -> mxcfb_rect {
        let top = std::cmp::min(self.top, p.y);
        let left = std::cmp::min(self.left, p.x);
        let bottom = std::cmp::max(self.top + self.height, p.y);
        let right = std::cmp::max(self.left + self.width, p.x);
        mxcfb_rect {
            left,
            top,
            width: right - left,
            height: bottom - top,
        }
    }

    pub fn merge_rect(&self, rect: &mxcfb_rect) -> mxcfb_rect {
        let self_is_empty = self.height == 0 || self.width == 0;
        let rect_is_empty = rect.height == 0 || rect.width == 0;
        if self_is_empty && rect_is_empty {
            mxcfb_rect::invalid()
        } else if self_is_empty {
            *rect
        } else if rect_is_empty {
            *self
        } else {
            let top = std::cmp::min(self.top, rect.top);
            let left = std::cmp::min(self.left, rect.left);
            let bottom = std::cmp::max(self.top + self.height, rect.top + rect.height);
            let right = std::cmp::max(self.left + self.width, rect.left + rect.width);
            mxcfb_rect {
                left,
                top,
                width: right - left,
                height: bottom - top,
            }
        }
    }

    pub fn expand(&self, margin: u32) -> mxcfb_rect {
        mxcfb_rect {
            left: self.left.saturating_sub(margin),
            top: self.top.saturating_sub(margin),
            width: self.width + (2 * margin),
            height: self.height + (2 * margin),
        }
    }
}

impl Default for mxcfb_update_data {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl Default for fb_fix_screeninfo {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl Default for fb_var_screeninfo {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
