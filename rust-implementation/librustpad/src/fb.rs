#![allow(dead_code)]

use libc;
use libc::ioctl;
use mmap;
use mmap::MemoryMap;

use std::os::unix::io::AsRawFd;
use std::sync::atomic::AtomicU32;
use std::fs::{OpenOptions, File};

use mxc_types;
use mxc_types::{VarScreeninfo, FixScreeninfo, FBIOGET_FSCREENINFO, FBIOGET_VSCREENINFO,
                FBIOPUT_VSCREENINFO};

use rusttype::{Font, FontCollection};

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

impl<'a> Framebuffer<'a> {
    pub fn new(path_to_device: &str) -> Framebuffer {
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
        // TODO: Make this more portable (right now the build box needs to have it in the location here. Any font really.)
        let font_data = include_bytes!("/usr/share/fonts/TTF/DejaVuSans.ttf");
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


    pub fn set_epdc_access(&mut self, state: bool) {
        const MXCFB_DISABLE_EPDC_ACCESS: u32 = io!(b'F', 0x35);
        const MXCFB_ENABLE_EPDC_ACCESS: u32 = io!(b'F', 0x36);
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

    pub fn set_autoupdate_mode(&mut self, mut mode: u32) {
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                mxc_types::MXCFB_SET_AUTO_UPDATE_MODE,
                &mut mode,
            );
        };
    }

    pub fn set_update_scheme(&mut self, mut scheme: u32) {
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                mxc_types::MXCFB_SET_UPDATE_SCHEME,
                &mut scheme,
            );
        };
    }

    ///Creates a FixScreeninfo struct and fills it using ioctl.
    pub fn get_fix_screeninfo(device: &File) -> FixScreeninfo {
        let mut info: FixScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_FSCREENINFO, &mut info) };
        if result != 0 {
            panic!("FBIOGET_FSCREENINFO failed");
        }
        return info;
    }

    ///Creates a VarScreeninfo struct and fills it using ioctl.
    pub fn get_var_screeninfo(device: &File) -> VarScreeninfo {
        let mut info: VarScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_VSCREENINFO, &mut info) };
        if result != 0 {
            panic!("FBIOGET_VSCREENINFO failed");
        }
        return info;
    }

    pub fn put_var_screeninfo(&mut self) -> bool {
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
