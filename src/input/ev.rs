use evdev;
use epoll;
use std;
use input;

/// Non-blocking function that will open the provided path and wait for more data with epoll.
/// `handler` must implement the `EvDevHandler` trait so that it can get callbacks `on_init` and `on_event`.
pub fn start_evdev<H: input::EvdevHandler + Send>(path: String, handler: &'static mut H) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut dev = evdev::Device::open(&path).unwrap();
        let devn = unsafe {
            let mut ptr = std::mem::transmute(dev.name().as_ptr());
            std::ffi::CString::from_raw(ptr).into_string().unwrap()
        };

        let mut v = vec![
            epoll::Event {
                events: (epoll::Events::EPOLLET | epoll::Events::EPOLLIN | epoll::Events::EPOLLPRI).bits(),
                data: 0,
            },
        ];

        let epfd = epoll::create(false).unwrap();
        epoll::ctl(epfd, epoll::ControlOptions::EPOLL_CTL_ADD, dev.fd(), v[0]).unwrap();

        // init callback
        handler.on_init(devn.clone(), &mut dev);

        loop {
            // -1 indefinite wait but it is okay because our EPOLL FD is watching on ALL input devices at once
            let res = epoll::wait(epfd, -1, &mut v[0..1]).unwrap();
            if res != 1 {
                warn!("epoll_wait returned {0}", res);
            }

            for ev in dev.events_no_sync().unwrap() {
                // event callback
                handler.on_event(&devn, ev);
            }
        }
    })
}
