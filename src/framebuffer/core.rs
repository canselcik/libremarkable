use libc::ioctl;
use memmap2::{MmapOptions, MmapRaw};

use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::AtomicU32;

use crate::device;
use crate::device::Model;
use crate::framebuffer;
use crate::framebuffer::common::{
    FBIOGET_FSCREENINFO, FBIOGET_VSCREENINFO, FBIOPUT_VSCREENINFO, MXCFB_DISABLE_EPDC_ACCESS,
    MXCFB_ENABLE_EPDC_ACCESS, MXCFB_SET_AUTO_UPDATE_MODE, MXCFB_SET_UPDATE_SCHEME,
};
use crate::framebuffer::screeninfo::{FixScreeninfo, VarScreeninfo};
use crate::framebuffer::swtfb_client::SwtfbClient;
use crate::framebuffer::FramebufferBase;

pub enum FramebufferUpdate {
    Ioctl(File),
    Swtfb(SwtfbClient),
}

/// Framebuffer struct containing the state (latest update marker etc.)
/// along with the var/fix screeninfo structs.
pub struct Framebuffer {
    pub frame: MmapRaw,
    pub marker: AtomicU32,
    /// Not updated as a result of calling `Framebuffer::put_var_screeninfo(..)`.
    /// It is your responsibility to update this when you call into that function
    /// like it has been done in `Framebuffer::new(..)`.
    pub var_screen_info: VarScreeninfo,
    pub fix_screen_info: FixScreeninfo,
    pub framebuffer_update: FramebufferUpdate,
}

unsafe impl Send for Framebuffer {}
unsafe impl Sync for Framebuffer {}

impl Default for Framebuffer {
    fn default() -> Self {
        Framebuffer::new()
    }
}

impl Framebuffer {
    /// Create a new framebuffer instance, autodetecting the correct update method.
    pub fn new() -> Framebuffer {
        let device = &*device::CURRENT_DEVICE;
        match device.model {
            Model::Gen1 => Framebuffer::device(device.get_framebuffer_path()),
            Model::Gen2 => {
                // Auto-select old method still if env LIBREMARKABLE_FB_DISFAVOR_INTERNAL_RM2FB is set affirmatively
                if let Ok(env_answer) = std::env::var("LIBREMARKABLE_FB_DISFAVOR_INTERNAL_RM2FB") {
                    let env_answer = env_answer.to_lowercase();
                    if env_answer == "1" || env_answer == "true" || env_answer == "yes" {
                        Framebuffer::device(device.get_framebuffer_path())
                    }
                }
                Framebuffer::rm2fb(device.get_framebuffer_path())
            }
        }
    }

    /// Use the [ioctl-based device interface](https://www.kernel.org/doc/html/latest/fb/internals.html)
    /// for framebuffer metadata.
    ///
    /// This matches the pre-0.6.0 behaviour, and relies on the rm2fb client
    /// shim on RM2. `new` is generally preferred, though existing apps may
    /// wish to use this method to avoid some risk of changing behaviour.
    pub fn device(path: impl AsRef<Path>) -> Framebuffer {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();
        Framebuffer::build(FramebufferUpdate::Ioctl(device))
    }

    /// Uses the [rm2fb interface](https://github.com/ddvk/remarkable2-framebuffer)
    /// ufor framebuffer metadata.
    ///
    /// This will not work at all on RM1; consider using `new` to autodetect
    /// the right interface for the current hardware.
    pub fn rm2fb(path: impl AsRef<Path>) -> Framebuffer {
        Framebuffer::build(FramebufferUpdate::Swtfb(SwtfbClient::new(path)))
    }

    #[deprecated = "Use `new` to autodetect the right update method based on your device version, or `device` or `rm2fb` to choose one explicitly."]
    pub fn from_path(path_to_device: &str) -> Framebuffer {
        if path_to_device == crate::device::Model::Gen2.framebuffer_path() {
            Framebuffer::device(path_to_device)
        } else {
            Framebuffer::rm2fb(path_to_device)
        }
    }

    fn build(framebuffer_update: FramebufferUpdate) -> Framebuffer {
        let mut var_screen_info = match &framebuffer_update {
            FramebufferUpdate::Ioctl(device) => Framebuffer::get_var_screeninfo(device),
            FramebufferUpdate::Swtfb(c) => c.get_var_screeninfo(),
        };
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

        let fix_screen_info = match &framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                Framebuffer::put_var_screeninfo(device, &mut var_screen_info);
                Framebuffer::get_fix_screeninfo(device)
            }
            FramebufferUpdate::Swtfb(c) => c.get_fix_screeninfo(),
        };

        let frame_length = (fix_screen_info.line_length * var_screen_info.yres) as usize;

        let mem_map = match &framebuffer_update {
            FramebufferUpdate::Ioctl(device) => MmapOptions::new()
                .len(frame_length)
                .map_raw(device)
                .expect("Unable to map provided path"),
            FramebufferUpdate::Swtfb(swtfb_client) => swtfb_client
                .open_buffer()
                .expect("Failed to open swtfb shared buffer"),
        };

        Framebuffer {
            marker: AtomicU32::new(1),
            frame: mem_map,
            var_screen_info,
            fix_screen_info,
            framebuffer_update,
        }
    }
}

impl framebuffer::FramebufferBase for Framebuffer {
    fn set_epdc_access(&mut self, state: bool) {
        match &self.framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                let request = if state {
                    MXCFB_ENABLE_EPDC_ACCESS
                } else {
                    MXCFB_DISABLE_EPDC_ACCESS
                };
                unsafe {
                    libc::ioctl(device.as_raw_fd(), request);
                };
            }
            FramebufferUpdate::Swtfb(_) => {}
        }
    }

    fn set_autoupdate_mode(&mut self, mode: u32) {
        match &self.framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                let m = mode.to_owned();
                unsafe {
                    libc::ioctl(
                        device.as_raw_fd(),
                        MXCFB_SET_AUTO_UPDATE_MODE,
                        &m as *const u32,
                    );
                };
            }
            FramebufferUpdate::Swtfb(_) => {}
        }
    }

    fn set_update_scheme(&mut self, scheme: u32) {
        match &self.framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                let s = scheme.to_owned();
                unsafe {
                    libc::ioctl(
                        device.as_raw_fd(),
                        MXCFB_SET_UPDATE_SCHEME,
                        &s as *const u32,
                    );
                };
            }
            FramebufferUpdate::Swtfb(_) => {}
        }
    }

    fn get_fix_screeninfo(device: &File) -> FixScreeninfo {
        let mut info: FixScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_FSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_FSCREENINFO failed");
        info
    }

    fn get_var_screeninfo(device: &File) -> VarScreeninfo {
        let mut info: VarScreeninfo = Default::default();
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOGET_VSCREENINFO, &mut info) };
        assert!(result == 0, "FBIOGET_VSCREENINFO failed");
        info
    }

    fn put_var_screeninfo(device: &std::fs::File, var_screen_info: &mut VarScreeninfo) -> bool {
        let result = unsafe { ioctl(device.as_raw_fd(), FBIOPUT_VSCREENINFO, var_screen_info) };
        result == 0
    }

    fn update_var_screeninfo(&mut self) -> bool {
        match &self.framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                Self::put_var_screeninfo(device, &mut self.var_screen_info)
            }
            FramebufferUpdate::Swtfb(_) => true,
        }
    }
}
