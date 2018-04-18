use evdev;
use epoll;
use std;
use input;
use input::EvdevHandler;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct EvDevContext {
    handle: Arc<std::thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl EvDevContext {
    /// Returns true if we were able to wait until the termination
    /// of the epoll thread. It will always return true since
    /// we have only one strong reference to the `JoinHandle<T>`.
    ///
    pub fn join(self) -> bool {
        match Arc::try_unwrap(self.handle) {
            Ok(handle) => {
                handle.join().unwrap();
                true
            }
            Err(_arc) => false,
        }
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Non-blocking function that will open the provided path and wait for more data with epoll
///
///    `handler` must implement the `EvDevHandler` trait so that it
///    can get callbacks `on_init` and `on_event`.
///
#[allow(mutable_transmutes)]
pub fn start_evdev<H: input::EvdevHandler + Send + Sync>(
    path: String,
    handler: &H,
) -> EvDevContext {
    let running = Arc::new(AtomicBool::new(true));
    let shared_running = Arc::clone(&running);
    let handler_ref = unsafe {
        std::mem::transmute::<&H, &'static mut input::UnifiedInputHandler>(&handler)
    };

    let handle = Arc::new(std::thread::spawn(move || {
        let mut dev = evdev::Device::open(&path).unwrap();
        let devn = unsafe {
            let mut ptr = std::mem::transmute(dev.name().as_ptr());
            std::ffi::CString::from_raw(ptr).into_string().unwrap()
        };

        let mut v = vec![
            epoll::Event {
                events: (epoll::Events::EPOLLET | epoll::Events::EPOLLIN | epoll::Events::EPOLLPRI)
                    .bits(),
                data: 0,
            },
        ];

        let epfd = epoll::create(false).unwrap();
        epoll::ctl(epfd, epoll::ControlOptions::EPOLL_CTL_ADD, dev.fd(), v[0]).unwrap();

        // init callback
        handler_ref.on_init(devn.clone(), &mut dev);

        while shared_running.load(Ordering::Relaxed) {
            // -1 indefinite wait but it is okay because our EPOLL FD is watching on ALL input devices at once
            let res = epoll::wait(epfd, -1, &mut v[0..1]).unwrap();
            if res != 1 {
                warn!("epoll_wait returned {0}", res);
            }

            for ev in dev.events_no_sync().unwrap() {
                // event callback
                handler_ref.on_event(&devn, ev);
            }
        }
    }));

    return EvDevContext { handle, running };
}
