use evdev;
use epoll;
use std;
use input;
use input::EvdevHandler;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct EvDevContext {
    device: input::InputDevice,
    exit_requested: Arc<AtomicBool>,
    exited: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
}

impl EvDevContext {
    pub fn started(&self) -> bool {
        self.started.load(Ordering::Relaxed)
    }

    pub fn exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::Relaxed)
    }

    pub fn stop(&mut self) {
        self.exit_requested.store(true, Ordering::Relaxed);
    }

    pub fn new(device: input::InputDevice) -> EvDevContext {
        EvDevContext {
            device,
            started: Arc::new(AtomicBool::new(false)),
            exit_requested: Arc::new(AtomicBool::new(false)),
            exited: Arc::new(AtomicBool::new(false)),
        }
    }

    fn run(&self, handler: &'static mut input::UnifiedInputHandler) {
        let path = match self.device {
            input::InputDevice::Wacom => "/dev/input/event0",
            input::InputDevice::Multitouch => "/dev/input/event1",
            input::InputDevice::GPIO => "/dev/input/event2",
            _ => return,
        };
        let res = evdev::Device::open(&path);
        match res {
            Ok(mut dev) => {
                let mut v = vec![
                    epoll::Event {
                        events: (epoll::Events::EPOLLET | epoll::Events::EPOLLIN
                            | epoll::Events::EPOLLPRI)
                            .bits(),
                        data: 0,
                    },
                ];
                let epfd = epoll::create(false).unwrap();
                epoll::ctl(epfd, epoll::ControlOptions::EPOLL_CTL_ADD, dev.fd(), v[0]).unwrap();

                // init callback
                handler.on_init(path.to_string());
                while !self.exit_requested.load(Ordering::Relaxed) {
                    // -1 indefinite wait but it is okay because our EPOLL FD
                    // is watching on ALL input devices at once.
                    let res = epoll::wait(epfd, -1, &mut v[0..1]).unwrap();
                    if res != 1 {
                        warn!("epoll_wait returned {0}", res);
                    }

                    for ev in dev.events_no_sync().unwrap() {
                        // event callback
                        handler.on_event(self.device.clone(), ev);
                    }
                }
            }
            Err(err) => {
                println!("ERR: {0}", err);
            }
        };
        self.exited.store(true, Ordering::Relaxed);
    }

    /// Non-blocking function that will open the provided path and wait for more data with epoll
    pub fn start(&self, handler: &'static mut input::UnifiedInputHandler) {
        self.started.store(true, Ordering::Relaxed);
        self.exited.store(false, Ordering::Relaxed);
        self.exit_requested.store(false, Ordering::Relaxed);

        let arc = Arc::new(self.clone());
        let _ = std::thread::spawn(move || {
            arc.run(handler);
        });
    }
}
