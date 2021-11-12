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
use crate::framebuffer::swtfb_ipc::SwtfbIpcQueue;

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
    pub swtfb_ipc_queue: Option<super::swtfb_ipc::SwtfbIpcQueue>,
}

unsafe impl<'a> Send for Framebuffer<'a> {}
unsafe impl<'a> Sync for Framebuffer<'a> {}

impl<'a> framebuffer::FramebufferBase<'a> for Framebuffer<'a> {
    fn from_path(path_to_device: &str) -> Framebuffer<'_> {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(if path_to_device == "/dev/shm/swtfb.01" {
                "/dev/fb0"
            } else {
                path_to_device
            })
            .unwrap();

        let swtfb_ipc_queue = if path_to_device == "/dev/shm/swtfb.01" {
            Some(SwtfbIpcQueue::new())
        } else {
            None
        };

        println!("Queue: {:?}", swtfb_ipc_queue.is_some());

        let mut var_screen_info =
            Framebuffer::get_var_screeninfo(&device, swtfb_ipc_queue.as_ref());
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

        println!("FB from_path: 1");

        Framebuffer::put_var_screeninfo(&device, swtfb_ipc_queue.as_ref(), &mut var_screen_info);

        println!("FB from_path: 2");

        let fix_screen_info = Framebuffer::get_fix_screeninfo(&device, swtfb_ipc_queue.as_ref());
        let frame_length = (fix_screen_info.line_length * var_screen_info.yres) as usize;

        println!("FB from_path: 3");

        let mem_map = MmapOptions::new()
            .len(frame_length)
            .map_raw(&device)
            .expect("Unable to map provided path");

        println!("FB from_path: 4");

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
            swtfb_ipc_queue,
        }
    }

    fn set_epdc_access(&mut self, state: bool) {
        println!("set_epdc_access");

        if self.swtfb_ipc_queue.is_none() {
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
    }

    fn set_autoupdate_mode(&mut self, mode: u32) {
        println!("set_autoupdate_mode");

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
        println!("set_update_scheme");

        let s = scheme.to_owned();
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                MXCFB_SET_UPDATE_SCHEME,
                &s as *const u32,
            );
        };
    }

    fn get_fix_screeninfo(device: &File, swtfb_ipc_queue: Option<&SwtfbIpcQueue>) -> FixScreeninfo {
        if swtfb_ipc_queue.is_some() {
            // https://github.com/ddvk/remarkable2-framebuffer/blob/e594fc44/src/shared/ipc.cpp#L96
            /*unsafe {
                libc::ftruncate(
                    device.as_raw_fd(),
                    super::swtfb_ipc::BUF_SIZE as libc::off_t,
                );
            }*/
            let mem_map = MmapOptions::new()
                .len(super::swtfb_ipc::BUF_SIZE as usize)
                .map_raw(device)
                .expect("Unable to map provided path");
            // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L217
            let mut screeninfo: FixScreeninfo = unsafe { std::mem::zeroed() };
            screeninfo.smem_start = mem_map.as_ptr() as u32;
            screeninfo.smem_len = super::swtfb_ipc::BUF_SIZE as u32;
            screeninfo.line_length =
                super::swtfb_ipc::WIDTH as u32 * std::mem::size_of::<u16>() as u32;
            return screeninfo;
        }
        let mut info: FixScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_FSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_FSCREENINFO failed");
        info
    }

    fn get_var_screeninfo(device: &File, swtfb_ipc_queue: Option<&SwtfbIpcQueue>) -> VarScreeninfo {
        if swtfb_ipc_queue.is_some() {
            // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L194
            let mut screeninfo: VarScreeninfo = unsafe { std::mem::zeroed() };
            screeninfo.xres = super::swtfb_ipc::WIDTH as u32;
            screeninfo.yres = super::swtfb_ipc::HEIGHT as u32;
            screeninfo.grayscale = 0;
            screeninfo.bits_per_pixel = 8 * std::mem::size_of::<u16>() as u32;
            screeninfo.xres_virtual = super::swtfb_ipc::WIDTH as u32;
            screeninfo.yres_virtual = super::swtfb_ipc::HEIGHT as u32;

            //set to RGB565
            screeninfo.red.offset = 11;
            screeninfo.red.length = 5;
            screeninfo.green.offset = 5;
            screeninfo.green.length = 6;
            screeninfo.blue.offset = 0;
            screeninfo.blue.length = 5;
            return screeninfo;
        }
        let mut info: VarScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_VSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_VSCREENINFO failed");
        info
    }

    fn put_var_screeninfo(
        device: &std::fs::File,
        swtfb_ipc_queue: Option<&SwtfbIpcQueue>,
        var_screen_info: &mut VarScreeninfo,
    ) -> bool {
        if swtfb_ipc_queue.is_some() {
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
            self.swtfb_ipc_queue.as_ref(),
            &mut self.var_screen_info,
        )
    }
}
