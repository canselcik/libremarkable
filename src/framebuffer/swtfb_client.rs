//! This implements the IPC part of a rM2Framebuffer client to interact with this server:
//! https://github.com/ddvk/remarkable2-framebuffer
//!
//! The client is developed according to the spec here:
//! https://github.com/ddvk/remarkable2-framebuffer/issues/11

use super::mxcfb::mxcfb_update_data;
use crate::device;
use crate::framebuffer::screeninfo::{FixScreeninfo, VarScreeninfo};
use memmap2::{MmapOptions, MmapRaw};
use std::ffi::{c_void, CStr, CString};
use std::fs::OpenOptions;
use std::io::Error as IoError;
use std::os::unix::prelude::AsRawFd;
use std::path::{Path, PathBuf};
use std::{env, mem, ptr};

const SWTFB_MESSAGE_QUEUE_ID: i32 = 0x2257c;

pub const WIDTH: i32 = crate::framebuffer::common::DISPLAYWIDTH as i32;
pub const HEIGHT: i32 = crate::framebuffer::common::DISPLAYHEIGHT as i32;

pub const BUF_SIZE: i32 = WIDTH * HEIGHT * std::mem::size_of::<u16>() as i32; // hardcoded size of display mem for rM2
const SEM_WAIT_TIMEOUT_NS: libc::c_long = 200_000_000;

/// long on 32 bit is 4 bytes as well!!
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum MSG_TYPE {
    INIT_t = 1,
    UPDATE_t = 2,
    XO_t = 3,
    WAIT_t = 4,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
pub struct xochitl_data {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,

    pub waveform: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
pub struct wait_sem_data {
    /// C string
    pub sem_name: [u8; 512],
}

/// MSG_TYPE has to match swtfb_update_data !!!
#[derive(Clone, Copy)]
#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
pub struct swtfb_update {
    pub mtype: MSG_TYPE,
    pub data: swtfb_update_data,
    //ms: u64,
}

#[derive(Clone, Copy)]
#[repr(C)]
#[allow(non_camel_case_types, dead_code)]
pub union swtfb_update_data {
    pub xochitl_update: xochitl_data,
    pub update: mxcfb_update_data,
    pub wait_update: wait_sem_data,
}

pub struct SwtfbClient {
    msqid: i32,
    path: PathBuf,
    do_wait_ioctl: bool,
}

impl Default for SwtfbClient {
    fn default() -> Self {
        Self::new(device::Model::Gen2.framebuffer_path())
    }
}

impl SwtfbClient {
    pub fn new(path: impl AsRef<Path>) -> SwtfbClient {
        assert!(
            device::CURRENT_DEVICE.model == device::Model::Gen2,
            "SWTFB is not supported on devices other than rM 2"
        );

        let msqid = unsafe {
            libc::msgget(
                SWTFB_MESSAGE_QUEUE_ID,
                libc::IPC_CREAT | libc::SHM_R | libc::SHM_W,
            )
        };
        assert!(msqid >= 0);

        Self {
            msqid,
            path: PathBuf::from(path.as_ref()),
            do_wait_ioctl: env::var("RM2FB_NO_WAIT_IOCTL").is_err(),
        }
    }

    pub fn open_buffer(&self) -> Result<MmapRaw, IoError> {
        let device = OpenOptions::new().read(true).write(true).open(&self.path)?;
        let ret = unsafe { libc::ftruncate(device.as_raw_fd(), BUF_SIZE as libc::off_t) };
        if ret < 0 {
            return Err(IoError::last_os_error());
        }
        let mem_map = MmapOptions::new().len(BUF_SIZE as usize).map_raw(&device)?;
        Ok(mem_map)
    }

    pub fn send(&self, update: &swtfb_update) -> bool {
        unsafe {
            let ptr = ptr::addr_of!(*update) as *const c_void;
            libc::msgsnd(self.msqid, ptr, mem::size_of::<swtfb_update>(), 0) == 0
        }
    }

    pub fn send_mxcfb_update(&self, update: &mxcfb_update_data) -> bool {
        self.send(&swtfb_update {
            mtype: MSG_TYPE::UPDATE_t,
            data: swtfb_update_data { update: *update },
        })
    }

    /// This function seems to be meant for internal use only.
    #[allow(dead_code)]
    fn send_xochitl_update(&self, data: &xochitl_data) -> bool {
        self.send(&swtfb_update {
            mtype: MSG_TYPE::XO_t,
            data: swtfb_update_data {
                xochitl_update: *data,
            },
        })
    }

    pub fn wait_for_update_complete(&self) {
        if !self.do_wait_ioctl {
            return;
        }

        // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L149

        let sem_name_str = format!("/rm2fb.wait.{}", unsafe { libc::getpid() });
        let mut sem_name = [0u8; 512];
        for (i, byte) in sem_name_str.as_bytes().iter().enumerate() {
            sem_name[i] = *byte;
        }
        self.send_wait_update(&wait_sem_data { sem_name });
        let sem_name_c = CString::new(sem_name_str.as_str()).unwrap();
        let sem = unsafe { libc::sem_open(sem_name_c.as_ptr(), libc::O_CREAT, 0x644, 0) };
        if sem == libc::SEM_FAILED {
            panic!(
                "Opening semaphore to wait for swtfb update failed: {:?}",
                unsafe { CStr::from_ptr(libc::strerror(*libc::__errno_location())) }
            );
        }

        let mut timeout = libc::timespec {
            tv_nsec: 0,
            tv_sec: 0,
        };
        unsafe {
            libc::clock_gettime(libc::CLOCK_REALTIME, &mut timeout);
        }
        timeout.tv_nsec += SEM_WAIT_TIMEOUT_NS;
        // Move overflow ns to secs
        timeout.tv_sec += timeout.tv_nsec / 1_000_000_000;
        timeout.tv_nsec %= 1_000_000_000;

        unsafe {
            libc::sem_timedwait(sem, &timeout);
            libc::sem_unlink(sem_name_c.as_ptr() as *const libc::c_char);
        }
    }

    pub fn send_wait_update(&self, wait_update: &wait_sem_data) -> bool {
        self.send(&swtfb_update {
            mtype: MSG_TYPE::WAIT_t,
            data: swtfb_update_data {
                wait_update: *wait_update,
            },
        })
    }

    pub fn get_fix_screeninfo(&self) -> FixScreeninfo {
        // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L217
        let mut screeninfo: FixScreeninfo = unsafe { std::mem::zeroed() };
        //screeninfo.smem_start = mem_map.as_ptr() as u32; // Not used anyway. TODO: Consider adding properly
        screeninfo.smem_len = super::swtfb_client::BUF_SIZE as u32;
        screeninfo.line_length =
            super::swtfb_client::WIDTH as u32 * std::mem::size_of::<u16>() as u32;
        screeninfo
    }

    pub fn get_var_screeninfo(&self) -> VarScreeninfo {
        // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L194
        let mut screeninfo: VarScreeninfo = unsafe { std::mem::zeroed() };
        screeninfo.xres = super::swtfb_client::WIDTH as u32;
        screeninfo.yres = super::swtfb_client::HEIGHT as u32;
        screeninfo.grayscale = 0;
        screeninfo.bits_per_pixel = 8 * std::mem::size_of::<u16>() as u32;
        screeninfo.xres_virtual = super::swtfb_client::WIDTH as u32;
        screeninfo.yres_virtual = super::swtfb_client::HEIGHT as u32;

        //set to RGB565
        screeninfo.red.offset = 11;
        screeninfo.red.length = 5;
        screeninfo.green.offset = 5;
        screeninfo.green.length = 6;
        screeninfo.blue.offset = 0;
        screeninfo.blue.length = 5;
        screeninfo
    }
}
