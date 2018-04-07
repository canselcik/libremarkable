#![allow(dead_code)]

use libc;
use libc::ioctl;
use mmap;
use mmap::MemoryMap;

use std::os::unix::io::AsRawFd;
use std::sync::atomic::AtomicU32;
use std::fs::{OpenOptions, File};

use framebuffer;
use framebuffer::screeninfo::{FixScreeninfo,VarScreeninfo};
use framebuffer::common::{FBIOGET_FSCREENINFO,
                          FBIOGET_VSCREENINFO,
                          FBIOPUT_VSCREENINFO,
                          MXCFB_SET_AUTO_UPDATE_MODE,
                          MXCFB_SET_UPDATE_SCHEME,
                          MXCFB_ENABLE_EPDC_ACCESS,
                          MXCFB_DISABLE_EPDC_ACCESS};

use rusttype::{Font, FontCollection};

/// Framebuffer struct containing the state (latest update marker etc.)
/// along with the var/fix screeninfo structs.
pub struct Framebuffer<'a> {
    pub device: File,
    pub frame: MemoryMap,
    pub marker: ::std::sync::atomic::AtomicU32,
    pub default_font: Font<'a>,
    pub var_screen_info: VarScreeninfo,
    pub fix_screen_info: FixScreeninfo,
}

unsafe impl<'a> Send for Framebuffer<'a> {}
unsafe impl<'a> Sync for Framebuffer<'a> {}

impl<'a> framebuffer::FramebufferBase<'a> for Framebuffer<'a> {
    /// Creates a new instance of Framebuffer
    fn new(path_to_device: &str) -> Framebuffer {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path_to_device)
            .unwrap();

        let mut var_screen_info = Framebuffer::get_var_screeninfo(&device);
        let fix_screen_info = Framebuffer::get_fix_screeninfo(&device);

        let frame_length = (fix_screen_info.line_length * var_screen_info.yres) as usize;
        let mem_map = MemoryMap::new(
            frame_length,
            &[
                mmap::MapOption::MapReadable,
                mmap::MapOption::MapWritable,
                mmap::MapOption::MapFd(device.as_raw_fd()),
                mmap::MapOption::MapOffset(0),
                mmap::MapOption::MapNonStandardFlags(libc::MAP_SHARED),
            ],
        ).unwrap();

        // Load the font
        let font_data = include_bytes!("../../assets/DejaVuSans.ttf");
        let collection = FontCollection::from_bytes(font_data as &[u8]);

        var_screen_info.xres = 1872;
        var_screen_info.yres = 1404;
        var_screen_info.rotate = 1;
        var_screen_info.width = var_screen_info.xres;
        var_screen_info.height = var_screen_info.yres;
        var_screen_info.pixclock = 160000000;
        var_screen_info.left_margin = 32;
        var_screen_info.right_margin = 326;
        var_screen_info.upper_margin = 4;
        var_screen_info.lower_margin = 12;
        var_screen_info.hsync_len = 44;
        var_screen_info.vsync_len = 1;
        var_screen_info.sync = 0;
        var_screen_info.vmode = 0; // FB_VMODE_NONINTERLACED
        var_screen_info.accel_flags = 0;
        let mut fb = Framebuffer {
            marker: AtomicU32::new(1),
            device,
            frame: mem_map,
            default_font: collection.into_font().unwrap(),
            var_screen_info,
            fix_screen_info,
        };
        if !fb.put_var_screeninfo() {
            panic!("FBIOPUT_VSCREENINFO failed");
        }
        return fb;
    }

    /// Toggles the EPD Controller (see https://wiki.mobileread.com/wiki/EPD_controller)
    fn set_epdc_access(&mut self, state: bool) {
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                if state {
                    MXCFB_ENABLE_EPDC_ACCESS
                } else {
                    MXCFB_DISABLE_EPDC_ACCESS
                },
            );
        };
    }

    /// Toggles autoupdate mode
    fn set_autoupdate_mode(&mut self, mode: u32) {
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                MXCFB_SET_AUTO_UPDATE_MODE,
                &mut mode.clone(),
            );
        };
    }

    /// Toggles update scheme
    fn set_update_scheme(&mut self, scheme: u32) {
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                MXCFB_SET_UPDATE_SCHEME,
                &mut scheme.clone(),
            );
        };
    }

    /// Creates a FixScreeninfo struct and fills it using ioctl
    fn get_fix_screeninfo(device: &File) -> FixScreeninfo {
        let mut info: FixScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_FSCREENINFO, &mut info) };
        if result != 0 {
            panic!("FBIOGET_FSCREENINFO failed");
        }
        return info;
    }

    /// Creates a VarScreeninfo struct and fills it using ioctl
    fn get_var_screeninfo(device: &File) -> VarScreeninfo {
        let mut info: VarScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_VSCREENINFO, &mut info) };
        if result != 0 {
            panic!("FBIOGET_VSCREENINFO failed");
        }
        return info;
    }

    /// Makes the proper ioctl call to set the VarScreenInfo.
    /// You must first update the contents of self.var_screen_info
    /// and then call this function.
    fn put_var_screeninfo(&mut self) -> bool {
        let result = unsafe {
            ioctl(
                self.device.as_raw_fd(),
                FBIOPUT_VSCREENINFO,
                &mut self.var_screen_info,
            )
        };
        return result == 0;
    }
}
