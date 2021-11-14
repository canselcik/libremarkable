use libc::{c_int, c_void, key_t, msqid_ds, size_t};
use redhook::{hook, real};

use libremarkable::framebuffer::swtfb_client;

hook! {
  unsafe fn msgget(key: key_t, msgflg: c_int) -> c_int => msgget_spy {
        let res = real!(msgget)(key, msgflg);
        eprintln!("Spy: msgget({:#x}, {:#x}) => {:#x}", key, msgflg, res);
        res
  }
}

hook! {
      unsafe fn msgctl(msqid: c_int, cmd: c_int, buf: *mut msqid_ds) -> c_int => msgctl_spy {
        let res = real!(msgctl)(msqid, cmd, buf);
        eprintln!("Spy: msgctl({:#x}, {:#x}, {:?}) => {}", msqid, cmd, buf, res);
        res
  }
}

hook! {
    unsafe fn msgsnd(msqid: c_int, msgp: *const c_void, msgsz: size_t, msgflg: c_int) -> c_int => msgsnd_spy {
        let res = real!(msgsnd)(msqid, msgp, msgsz, msgflg);
        eprintln!("Spy: msgsnd({:#x}, {:?}, {}, {:#x}) => {}", msqid, msgp, msgsz, msgflg, res);
        if msgsz == std::mem::size_of::<swtfb_client::swtfb_update>() {
            let msg = &*(msgp as *const swtfb_client::swtfb_update);
                eprintln!("Spy: msgsnd: Message: swt_update.mtype: {:?}, data: ... }}", msg.mtype);
                let data_str_formatted = match msg.mtype {
                    swtfb_client::MSG_TYPE::INIT_t => {
                        format!("...")
                    },
                    swtfb_client::MSG_TYPE::UPDATE_t => {
                        format!("{:?}", msg.data.update)
                    },
                    swtfb_client::MSG_TYPE::XO_t => {
                        format!("{:?}", msg.data.xochitl_update)
                    },
                    swtfb_client::MSG_TYPE::WAIT_t => {
                        format!("{:?}", msg.data.wait_update)
                    }
                };
                eprintln!("Spy: msgsnd: Message: swt_update.data: {}", data_str_formatted);
        }else {
            eprintln!("Spy: msgsnd: Error: Message is not sizeof(swtfb_update) (expected {}, got {})", std::mem::size_of::<swtfb_client::swtfb_update>(), msgsz)
        }
        res
  }
}
