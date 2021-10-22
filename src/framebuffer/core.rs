use memmap2::{MmapOptions, MmapRaw};

use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::FromRawFd;
use std::sync::atomic::AtomicU32;

use super::LIBRM2FB_CLIENT;
use crate::auto_ioctl;
use crate::framebuffer;
use crate::framebuffer::common::{
    FBIOGET_FSCREENINFO, FBIOGET_VSCREENINFO, FBIOPUT_VSCREENINFO, MXCFB_DISABLE_EPDC_ACCESS,
    MXCFB_ENABLE_EPDC_ACCESS, MXCFB_SET_AUTO_UPDATE_MODE, MXCFB_SET_UPDATE_SCHEME,
};
use crate::framebuffer::screeninfo::{FixScreeninfo, VarScreeninfo};

use rusttype::{Font, FontCollection};

/// Framebuffer struct containing the state (latest update marker etc.)
/// along with the var/fix screeninfo structs.
pub struct Framebuffer<'a> {
    pub device: File,
    pub frame: MmapRaw,
    pub marker: AtomicU32,
    pub default_font: Font<'a>,
    /// Not updated as a result of calling `Framebuffer::put_var_screeninfo(..)`.
    /// It is your responsibility to update this when you call into that function
    /// like it has been done in `Framebuffer::new(..)`.
    pub var_screen_info: VarScreeninfo,
    pub fix_screen_info: FixScreeninfo,
}

unsafe impl<'a> Send for Framebuffer<'a> {}
unsafe impl<'a> Sync for Framebuffer<'a> {}

impl<'a> framebuffer::FramebufferBase<'a> for Framebuffer<'a> {
    fn from_path(path_to_device: &str) -> Framebuffer<'_> {
        let device = if true {
            if let Some(lib) = &*LIBRM2FB_CLIENT {
                unsafe {
                    let c_str = std::ffi::CString::new(path_to_device).unwrap();
                    let c_world: *const libc::c_char = c_str.as_ptr() as *const libc::c_char;
                    let open_func: libloading::Symbol<
                        unsafe extern "C" fn(
                            fd: *const libc::c_char,
                            flags: libc::c_int,
                            mode: libc::mode_t,
                        ) -> libc::c_int,
                    > = lib.get(b"open").unwrap();
                    let fd = open_func(c_world, libc::O_RDWR, 0 as libc::mode_t);
                    println!("FD is {}", fd);
                    std::fs::File::from_raw_fd(fd)
                }
            } else {
                panic!("Failed to open file with shim lib!");
            }
        } else {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path_to_device)
                .unwrap()
        };

        let mut var_screen_info = Framebuffer::get_var_screeninfo(&device);
        var_screen_info.xres = 1404;
        var_screen_info.yres = 1872;
        var_screen_info.rotate = 1;
        var_screen_info.width = 0xffff_ffff;
        var_screen_info.height = 0xffff_ffff;
        var_screen_info.pixclock = 6250;
        var_screen_info.left_margin = 32;
        var_screen_info.right_margin = 326;
        var_screen_info.upper_margin = 4;
        var_screen_info.lower_margin = 12;
        var_screen_info.hsync_len = 44;
        var_screen_info.vsync_len = 1;
        var_screen_info.sync = 0;
        var_screen_info.vmode = 0; // FB_VMODE_NONINTERLACED
        var_screen_info.accel_flags = 0;

        Framebuffer::put_var_screeninfo(&device, &mut var_screen_info);

        let fix_screen_info = Framebuffer::get_fix_screeninfo(&device);
        let frame_length = (fix_screen_info.line_length * var_screen_info.yres) as usize;
        println!("FIX: {:#?}\nVAR: {:#?}", fix_screen_info, var_screen_info);
        let mem_map = MmapOptions::new()
            .len(frame_length)
            .map_raw(&device)
            .expect("Unable to map provided path");

        // Load the font
        let font_data = include_bytes!("../../assets/Roboto-Regular.ttf");
        let collection = FontCollection::from_bytes(font_data as &[u8]);
        Framebuffer {
            marker: AtomicU32::new(1),
            device,
            frame: mem_map,
            default_font: collection
                .and_then(|ft| ft.into_fonts().next().unwrap())
                .unwrap(),
            var_screen_info,
            fix_screen_info,
        }
    }

    fn set_epdc_access(&mut self, state: bool) {
        unsafe {
            auto_ioctl!(
                self.device.as_raw_fd(),
                if state {
                    MXCFB_ENABLE_EPDC_ACCESS
                } else {
                    MXCFB_DISABLE_EPDC_ACCESS
                }
            );
        };
    }

    fn set_autoupdate_mode(&mut self, mode: u32) {
        let m = mode.to_owned();
        unsafe {
            auto_ioctl!(
                self.device.as_raw_fd(),
                MXCFB_SET_AUTO_UPDATE_MODE,
                &m as *const u32
            );
        };
    }

    fn set_update_scheme(&mut self, scheme: u32) {
        let s = scheme.to_owned();
        unsafe {
            auto_ioctl!(
                self.device.as_raw_fd(),
                MXCFB_SET_UPDATE_SCHEME,
                &s as *const u32
            );
        };
    }

    fn get_fix_screeninfo(device: &File) -> FixScreeninfo {
        let mut info: FixScreeninfo = Default::default();
        let result = unsafe { auto_ioctl!(device.as_raw_fd(), FBIOGET_FSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_FSCREENINFO failed");
        info
    }

    fn get_var_screeninfo(device: &File) -> VarScreeninfo {
        let mut info: VarScreeninfo = Default::default();
        let result = unsafe { auto_ioctl!(device.as_raw_fd(), FBIOGET_VSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_VSCREENINFO failed");
        info
    }

    fn put_var_screeninfo(device: &File, var_screen_info: &mut VarScreeninfo) -> bool {
        let result =
            unsafe { auto_ioctl!(device.as_raw_fd(), FBIOPUT_VSCREENINFO, var_screen_info) };
        result == 0
    }

    fn update_var_screeninfo(&mut self) -> bool {
        Self::put_var_screeninfo(&self.device, &mut self.var_screen_info)
    }
}
