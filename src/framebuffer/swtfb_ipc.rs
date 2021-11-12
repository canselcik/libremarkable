//! This implements the IPC part of a rM2Framebuffer client to interact with this server:
//! https://github.com/ddvk/remarkable2-framebuffer
//!
//! The client is developed according to the spec here:
//! https://github.com/ddvk/remarkable2-framebuffer/issues/11

const SWTFB_MESSAGE_QUEUE_ID: i32 = 0x2257c;

use super::mxcfb::mxcfb_update_data;
use std::ffi::CString;

pub const WIDTH: i32 = 1404;
pub const HEIGHT: i32 = 1872;

#[allow(non_upper_case_globals)]
pub const maxWidth: i32 = 1404;
#[allow(non_upper_case_globals)]
pub const maxHeight: i32 = 1872;
pub const BUF_SIZE: i32 = maxWidth * maxHeight * std::mem::size_of::<u16>() as i32; // hardcoded size of display mem for rM2
const SEM_WAIT_TIMEOUT: i32 = 200000000; /* 200 * 1000 * 1000, e.g. 200ms */

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
#[allow(non_camel_case_types, dead_code)]
pub struct xochitl_data {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,

    waveform: i32,
    flags: i32,
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types, dead_code)]
pub struct wait_sem_data {
    /// C string
    sem_name: [u8; 512],
}

/// MSG_TYPE has to match swtfb_update_data !!!
#[derive(Clone, Copy)]
#[allow(non_camel_case_types, dead_code)]
pub struct swtfb_update {
    mtype: MSG_TYPE,
    data: swtfb_update_data,
    //ms: u64,
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types, dead_code)]
pub union swtfb_update_data {
    xochitl_update: xochitl_data,
    update: mxcfb_update_data,
    wait_update: wait_sem_data,
}

pub struct SwtfbIpcQueue {
    msqid: i32,
    do_wait_ioctl: bool,
}

impl SwtfbIpcQueue {
    pub fn new() -> Self {
        let msqid = unsafe {
            libc::msgget(
                SWTFB_MESSAGE_QUEUE_ID,
                libc::IPC_CREAT | libc::SHM_R | libc::SHM_W,
            )
        };
        if msqid < 0 {
            // TODO: Make proper error
            panic!("Got an error when initializing/creating ipc queue!");
        }

        // Not sure if actually needed
        std::env::set_var("RM2FB_SHIM", "0.1");

        // TODO: Nested not yet handled!
        if std::env::var("RM2FB_ACTIVE").is_ok() {
            std::env::set_var("RM2FB_NESTED", "1");
        } else {
            std::env::set_var("RM2FB_ACTIVE", "1");
        }

        Self {
            msqid,
            do_wait_ioctl: std::env::var("RM2FB_NO_WAIT_IOCTL").is_err(),
        }
    }

    pub fn send(&self, update: &swtfb_update) -> bool {
        unsafe {
            let ptr = std::ptr::addr_of!(update) as *const std::ffi::c_void;
            println!("Ptr: {:?}", ptr);
            println!("Size: {}", std::mem::size_of::<swtfb_update>());
            libc::msgsnd(self.msqid, ptr, std::mem::size_of::<swtfb_update>(), 0) == 0
        }
    }

    pub fn send_mxcfb_update(&self, update: &mxcfb_update_data) -> bool {
        println!("Sending update...");
        let ret = self.send(&swtfb_update {
            mtype: MSG_TYPE::UPDATE_t,
            data: swtfb_update_data { update: *update },
        });
        println!("Update sent: {}", ret);
        ret
    }

    pub fn send_xochitl_update(&self, data: &xochitl_data) -> bool {
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

        // UNTESTED!
        // https://github.com/ddvk/remarkable2-framebuffer/blob/1e288aa9/src/client/main.cpp#L149

        let sem_name_str = format!("/rm2fb.wait.{}", unsafe { libc::getpid() });
        let mut sem_name = [0u8; 512];
        for (i, byte) in sem_name_str.as_bytes().into_iter().enumerate() {
            sem_name[i] = *byte;
        }
        self.send_wait_update(&wait_sem_data { sem_name });
        let sem_name_c = CString::new(sem_name_str.as_str()).unwrap();
        let sem = unsafe { libc::sem_open(sem_name_c.as_ptr() as *const u8, libc::O_CREAT) };

        let mut timeout = libc::timespec {
            tv_nsec: 0,
            tv_sec: 0,
        };
        unsafe {
            libc::clock_gettime(libc::CLOCK_REALTIME, &mut timeout);
        }
        timeout.tv_nsec += SEM_WAIT_TIMEOUT;

        if timeout.tv_nsec >= 1_000_000_000 {
            timeout.tv_nsec -= 1_000_000_000;
            timeout.tv_sec += 1;
        }
        unsafe {
            libc::sem_timedwait(sem, &timeout);
            libc::sem_unlink(sem_name_c.as_ptr() as *const u8);
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
}

impl Drop for SwtfbIpcQueue {
    fn drop(&mut self) {
        if unsafe { libc::msgctl(self.msqid, libc::IPC_RMID, std::ptr::null_mut()) } != 0 {
            panic!("Got an error when closing ipc queue!")
        }
    }
}
