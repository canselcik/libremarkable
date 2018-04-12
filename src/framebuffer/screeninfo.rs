/// Bitfield which is a part of VarScreeninfo.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Bitfield {
    pub offset: u32,
    pub length: u32,
    pub msb_right: u32,
}

/// Struct as defined in /usr/include/linux/fb.h
#[repr(C)]
#[derive(Clone, Debug)]
pub struct VarScreeninfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: Bitfield,
    pub green: Bitfield,
    pub blue: Bitfield,
    pub transp: Bitfield,
    pub nonstd: u32,
    pub activate: u32,
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

/// Struct as defined in /usr/include/linux/fb.h Note: type is a keyword in Rust and therefore has been
/// changed to fb_type.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct FixScreeninfo {
    pub id: [u8; 16],
    pub smem_start: usize,
    pub smem_len: u32,
    pub fb_type: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: usize,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

impl ::std::default::Default for Bitfield {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl ::std::default::Default for VarScreeninfo {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

impl ::std::default::Default for FixScreeninfo {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct fb_bitfield {
    /// beginning of bitfield
    pub offset: u32,

    /// length of bitfield
    pub length: u32,

    /// != 0 : Most significant bit is right
    pub msb_right: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct fb_var_screeninfo {
    /// visible resolution
    pub xres: u32,
    pub yres: u32,
    /// virtual resolution
    pub xres_virtual: u32,
    pub yres_virtual: u32,

    /// offset from virtual to visible
    pub xoffset: u32,
    pub yoffset: u32,

    /// resolution
    pub bits_per_pixel: u32,

    pub grayscale: u32,

    /// 0 = color, 1 = grayscale,  >1 = FOURCC
    /// bitfield in fb mem if true color,
    /// else only length is significant
    pub red: fb_bitfield,
    pub green: fb_bitfield,
    pub blue: fb_bitfield,
    pub transp: fb_bitfield,

    /// != 0 Non standard pixel format
    pub nonstd: u32,

    /// see FB_ACTIVATE_*
    pub activate: u32,

    /// height of picture in mm
    pub height: u32,

    /// width of picture in mm
    pub width: u32,

    /// (OBSOLETE) see fb_info.flags
    pub accel_flags: u32,

    /// Timing: All values in pixclocks, except pixclock (of course)
    /// pixel clock in ps (pico seconds)
    pub pixclock: u32,

    /// time from sync to picture
    pub left_margin: u32,

    /// time from picture to sync
    pub right_margin: u32,

    /// time from sync to picture
    pub upper_margin: u32,
    pub lower_margin: u32,

    /// length of horizontal sync
    pub hsync_len: u32,

    /// length of vertical sync
    pub vsync_len: u32,

    /// see FB_SYNC_*
    pub sync: u32,

    /// see FB_VMODE_*
    pub vmode: u32,

    /// angle we rotate counter clockwise
    pub rotate: u32,

    /// colorspace for FOURCC-based modes
    pub colorspace: u32,

    /// Reserved for future compatibility
    pub reserved: [u32; 4],
}
