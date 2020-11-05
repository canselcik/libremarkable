use crate::input;

use input::scan::SCANNED;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct EvDevContext {
    device: input::InputDevice,
    pub state: input::InputDeviceState,
    pub tx: std::sync::mpsc::Sender<input::InputEvent>,
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

    /// After exit is requested, there will be one more event read from the device before
    /// it is closed.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::Relaxed)
    }

    pub fn stop(&mut self) {
        self.exit_requested.store(true, Ordering::Relaxed);
    }

    pub fn new(
        device: input::InputDevice,
        tx: std::sync::mpsc::Sender<input::InputEvent>,
    ) -> EvDevContext {
        EvDevContext {
            device,
            tx,
            state: input::InputDeviceState::new(device),
            started: Arc::new(AtomicBool::new(false)),
            exit_requested: Arc::new(AtomicBool::new(false)),
            exited: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Non-blocking function that will open the provided path and wait for more data with epoll
    pub fn start(&mut self) {
        let path = SCANNED.get_path(self.device);

        self.started.store(true, Ordering::Relaxed);
        self.exited.store(false, Ordering::Relaxed);
        self.exit_requested.store(false, Ordering::Relaxed);

        match SCANNED.get_device(self.device) {
            Err(e) => error!("Error while reading events from epoll fd: {0}", e),
            Ok(mut dev) => {
                let mut v = vec![epoll::Event {
                    events: (epoll::Events::EPOLLET
                        | epoll::Events::EPOLLIN
                        | epoll::Events::EPOLLPRI)
                        .bits(),
                    data: 0,
                }];
                let epfd = epoll::create(false).unwrap();
                epoll::ctl(epfd, epoll::ControlOptions::EPOLL_CTL_ADD, dev.fd(), v[0]).unwrap();

                // init callback
                info!("Init complete for {:?}", path);

                let exit_req = Arc::clone(&self.exit_requested);
                let exited = Arc::clone(&self.exited);
                let device_type = self.device;
                let state = self.state.clone();
                let tx = self.tx.clone();
                let _ = std::thread::spawn(move || {
                    while !exit_req.load(Ordering::Relaxed) {
                        // -1 indefinite wait but it is okay because our EPOLL FD
                        // is watching on ALL input devices at once.
                        let res = match epoll::wait(epfd, -1, &mut v[0..1]) {
                            Ok(res) => res,
                            Err(err) => {
                                warn!("epoll_wait failed: {}", err);
                                continue;
                            }
                        };
                        if res != 1 {
                            warn!("epoll_wait returned {0}", res);
                        }

                        for ev in dev.events_no_sync().unwrap() {
                            // event callback
                            match device_type {
                                input::InputDevice::Multitouch => {
                                    for event in input::multitouch::decode(&ev, &state) {
                                        if let Err(e) = tx.send(event) {
                                            error!(
                                                "Failed to write InputEvent into the channel: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                input::InputDevice::Wacom => {
                                    if let Some(event) = input::wacom::decode(&ev, &state) {
                                        if let Err(e) = tx.send(event) {
                                            error!(
                                                "Failed to write InputEvent into the channel: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                input::InputDevice::GPIO => {
                                    if let Some(event) = input::gpio::decode(&ev, &state) {
                                        if let Err(e) = tx.send(event) {
                                            error!(
                                                "Failed to write InputEvent into the channel: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                _ => unreachable!(),
                            };
                        }
                    }
                    exited.store(true, Ordering::Relaxed);
                });
            }
        }
    }
}
