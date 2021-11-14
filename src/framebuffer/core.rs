use libc::ioctl;
use memmap2::{MmapOptions, MmapRaw};
use rusttype::Font;

use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::AtomicU32;

use crate::framebuffer;
use crate::framebuffer::common::{
    FBIOGET_FSCREENINFO, FBIOGET_VSCREENINFO, FBIOPUT_VSCREENINFO, MXCFB_DISABLE_EPDC_ACCESS,
    MXCFB_ENABLE_EPDC_ACCESS, MXCFB_SET_AUTO_UPDATE_MODE, MXCFB_SET_UPDATE_SCHEME,
};
use crate::framebuffer::screeninfo::{FixScreeninfo, VarScreeninfo};
use crate::framebuffer::swtfb_client::SwtfbClient;

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
    pub swtfb_client: Option<super::swtfb_client::SwtfbClient>,
}

unsafe impl<'a> Send for Framebuffer<'a> {}
unsafe impl<'a> Sync for Framebuffer<'a> {}

impl<'a> framebuffer::FramebufferBase<'a> for Framebuffer<'a> {
    fn from_path(path_to_device: &str) -> Framebuffer<'_> {
        let swtfb_client = if path_to_device == crate::device::Model::Gen2.framebuffer_path() {
            Some(SwtfbClient::default())
        } else {
            None
        };

        let (device, mem_map) = if let Some(ref swtfb_client) = swtfb_client {
            let (device, mem_map) = swtfb_client
                .open_buffer()
                .expect("Failed to open swtfb shared buffer");
            (device, Some(mem_map))
        } else {
            let device = OpenOptions::new()
                .read(true)
                .write(true)
                .open(path_to_device)
                .unwrap();
            (device, None)
        };

        let mut var_screen_info = Framebuffer::get_var_screeninfo(&device, swtfb_client.as_ref());
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

        Framebuffer::put_var_screeninfo(&device, swtfb_client.as_ref(), &mut var_screen_info);

        let fix_screen_info = Framebuffer::get_fix_screeninfo(&device, swtfb_client.as_ref());
        let frame_length = (fix_screen_info.line_length * var_screen_info.yres) as usize;

        let mem_map = if let Some(mem_map) = mem_map {
            mem_map
        } else {
            MmapOptions::new()
                .len(frame_length)
                .map_raw(&device)
                .expect("Unable to map provided path")
        };

        // Load the font
        let font_data = include_bytes!("../../assets/Roboto-Regular.ttf");
        let default_font = Font::try_from_bytes(font_data as &[u8]).expect("corrupted font data");
        Framebuffer {
            marker: AtomicU32::new(1),
            device,
            frame: mem_map,
            default_font,
            var_screen_info,
            fix_screen_info,
            swtfb_client,
        }
    }

    fn set_epdc_access(&mut self, state: bool) {
        if self.swtfb_client.is_some() {
            // Not catched in rm2fb => noop
            return;
        }

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

    fn set_autoupdate_mode(&mut self, mode: u32) {
        if self.swtfb_client.is_some() {
            // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L137
            // Is a noop in rm2fb
            return;
        }

        let m = mode.to_owned();
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                MXCFB_SET_AUTO_UPDATE_MODE,
                &m as *const u32,
            );
        };
    }

    fn set_update_scheme(&mut self, scheme: u32) {
        if self.swtfb_client.is_some() {
            // Not catched in rm2fb => noop
            return;
        }

        let s = scheme.to_owned();
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                MXCFB_SET_UPDATE_SCHEME,
                &s as *const u32,
            );
        };
    }

    fn get_fix_screeninfo(device: &File, swtfb_client: Option<&SwtfbClient>) -> FixScreeninfo {
        if let Some(swtfb_client) = swtfb_client {
            return swtfb_client.get_fix_screeninfo();
        }

        let mut info: FixScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_FSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_FSCREENINFO failed");
        info
    }

    fn get_var_screeninfo(device: &File, swtfb_client: Option<&SwtfbClient>) -> VarScreeninfo {
        if let Some(swtfb_client) = swtfb_client {
            return swtfb_client.get_var_screeninfo();
        }

        let mut info: VarScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_VSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_VSCREENINFO failed");
        info
    }

    fn put_var_screeninfo(
        device: &std::fs::File,
        swtfb_client: Option<&SwtfbClient>,
        var_screen_info: &mut VarScreeninfo,
    ) -> bool {
        if swtfb_client.is_some() {
            // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L214
            // Is a noop in rm2fb
            return true;
        }

        let result = unsafe { ioctl(device.as_raw_fd(), FBIOPUT_VSCREENINFO, var_screen_info) };
        result == 0
    }

    fn update_var_screeninfo(&mut self) -> bool {
        Self::put_var_screeninfo(
            &self.device,
            self.swtfb_client.as_ref(),
            &mut self.var_screen_info,
        )
    }
}
