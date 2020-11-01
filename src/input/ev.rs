use crate::input;

use log::{error, info, warn};

use fxhash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

lazy_static! {
    /// Map of what InputDevice has what event file path on the system.
    /// The paths are different depending on the reMarkable model.
    pub static ref INPUT_DEVICE_PATHS: FxHashMap<input::InputDevice, PathBuf> =
        event_file_paths();
}

/// Returns a HashMap that contains the appropriate file paths for the InputDevices
/// This way it'll always find the correct device no matter the device generation.
/// Based on code by [@raisjn](https://github.com/raisjn): https://gist.github.com/raisjn/01b16286dcc4461a6643f40f8f553966
fn event_file_paths() -> FxHashMap<input::InputDevice, PathBuf> {
    let mut input_device_paths: FxHashMap<input::InputDevice, PathBuf> = FxHashMap::default();

    // Get all /dev/input/event* file paths
    let mut event_file_paths: Vec<PathBuf> = Vec::new();
    let input_dir = Path::new("/dev/input");
    for entry in input_dir
        .read_dir()
        .unwrap_or_else(|_| panic!("Failed to list {:?}", input_dir))
    {
        let entry = entry.unwrap();
        let file_name = entry.file_name().as_os_str().to_str().unwrap().to_owned();
        if !file_name.starts_with("event") {
            continue;
        }

        let evdev_path = input_dir.join(&file_name);
        event_file_paths.push(evdev_path);
    }

    // Open and check capabilities of each event device
    for evdev_path in event_file_paths {
        let dev = evdev::Device::open(&evdev_path)
            .unwrap_or_else(|_| panic!("Failed to scan {:?}", &evdev_path));
        if dev.events_supported().contains(evdev::KEY) {
            if dev.keys_supported().contains(evdev::BTN_STYLUS as usize)
                && dev.events_supported().contains(evdev::ABSOLUTE)
            {
                // The device with the wacom digitizer has the BTN_STYLUS event
                // and support KEY as well as ABSOLUTE event types
                input_device_paths.insert(input::InputDevice::Wacom, evdev_path.clone());
            }

            if dev.keys_supported().contains(evdev::KEY_POWER as usize) {
                // The device for buttons has the KEY_POWER button and support KEY event types
                input_device_paths.insert(input::InputDevice::GPIO, evdev_path.clone());
            }
        }

        if dev.events_supported().contains(evdev::RELATIVE)
            && dev.absolute_axes_supported().contains(evdev::ABS_MT_SLOT)
        {
            // The touchscreen device has the ABS_MT_SLOT event and supports RELATIVE event types
            input_device_paths.insert(input::InputDevice::Multitouch, evdev_path.clone());
        }
    }

    input_device_paths
}

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
        if self.device == input::InputDevice::Unknown {
            panic!("Invalid device (\"Unknown\")!");
        }
        let path = INPUT_DEVICE_PATHS[&self.device].clone();

        self.started.store(true, Ordering::Relaxed);
        self.exited.store(false, Ordering::Relaxed);
        self.exit_requested.store(false, Ordering::Relaxed);

        match evdev::Device::open(&path) {
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
