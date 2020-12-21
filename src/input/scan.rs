use super::ecodes;
use super::InputDevice;
use cgmath::Vector2;
use log::debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const INITIAL_DEVS_AVAILABLE_FOR: Duration = Duration::from_millis(1000);

lazy_static! {
    /// A singleton of the EvDevsScan object
    pub static ref SCANNED: EvDevs = EvDevs::new();
}

/// This struct contains the results of initially scaning all evdev devices,
/// which allows for device model independancy.
/// Some of its data is used by other constants.
///
/// EvDevsScan has some internal mutability to allow resuing the opened devices
/// for some time to increase performance.
/// TODO: Call this `EvDevsScanOutcome` or EvScanOutcome instead ??
pub struct EvDevs {
    pub wacom_path: PathBuf,
    pub multitouch_path: PathBuf,
    pub gpio_path: PathBuf,

    pub wacom_width: u16,
    pub wacom_height: u16,
    pub mt_width: u16,
    pub mt_height: u16,

    /// The resolution of the wacom no rotation applied
    pub wacom_orig_size: Vector2<u16>,
    pub multitouch_orig_size: Vector2<u16>,

    // Those will be preserved in case they are needed fairly fast
    // to prevent any additional delay of re-opening the fds.
    // They will get removed fairly quickly though.
    wacom_initial_dev: Arc<Mutex<Option<evdev::Device>>>,
    multitouch_initial_dev: Arc<Mutex<Option<evdev::Device>>>,
    gpio_initial_dev: Arc<Mutex<Option<evdev::Device>>>,
}

impl EvDevs {
    /// Scan all the evdev devices, figure out which is which
    /// and get some additional data for lazy constants.
    fn new() -> Self {
        // All of these have to be found
        let mut wacom_path: Option<PathBuf> = None;
        let mut wacom_dev: Option<evdev::Device> = None;
        let mut multitouch_path: Option<PathBuf> = None;
        let mut multitouch_dev: Option<evdev::Device> = None;
        let mut gpio_path: Option<PathBuf> = None;
        let mut gpio_dev: Option<evdev::Device> = None;

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
                    wacom_path = Some(evdev_path.clone());
                    wacom_dev = Some(dev);
                    continue;
                }

                if dev.keys_supported().contains(evdev::KEY_POWER as usize) {
                    // The device for buttons has the KEY_POWER button and support KEY event types
                    gpio_path = Some(evdev_path.clone());
                    gpio_dev = Some(dev);
                    continue;
                }
            }

            if dev.events_supported().contains(evdev::RELATIVE)
                && dev.absolute_axes_supported().contains(evdev::ABS_MT_SLOT)
            {
                // The touchscreen device has the ABS_MT_SLOT event and supports RELATIVE event types
                multitouch_path = Some(evdev_path.clone());
                multitouch_dev = Some(dev);
                continue;
            }
        }

        // Ensure that all devices were found
        if wacom_path.is_none() || wacom_dev.is_none() {
            panic!("Failed to find the wacom digitizer evdev!");
        }
        let wacom_path = wacom_path.unwrap();
        let wacom_dev = wacom_dev.unwrap();
        if multitouch_path.is_none() || multitouch_dev.is_none() {
            panic!("Failed to find the multitouch evdev!");
        }
        let multitouch_path = multitouch_path.unwrap();
        let multitouch_dev = multitouch_dev.unwrap();
        if gpio_path.is_none() || gpio_dev.is_none() {
            panic!("Failed to find the gpio evdev!");
        }
        let gpio_path = gpio_path.unwrap();
        let gpio_dev = gpio_dev.unwrap();

        // SIZES
        let wacom_state = wacom_dev.state();
        let wacom_orig_size = Vector2 {
            x: wacom_state.abs_vals[ecodes::ABS_X as usize].maximum as u16,
            y: wacom_state.abs_vals[ecodes::ABS_Y as usize].maximum as u16,
        };
        // X and Y are swapped for the wacom since rM1 and probably also rM2 have it rotated
        let (wacom_width, wacom_height) = crate::device::CURRENT_DEVICE
            .get_wacom_placement()
            .rotation
            .rotated_size(&wacom_orig_size)
            .into();

        let mt_state = multitouch_dev.state();
        let multitouch_orig_size = Vector2 {
            x: mt_state.abs_vals[ecodes::ABS_MT_POSITION_X as usize].maximum as u16,
            y: mt_state.abs_vals[ecodes::ABS_MT_POSITION_Y as usize].maximum as u16,
        };
        // Axes are swapped on the rM2 (see InputDeviceRotation for more)
        let (mt_width, mt_height) = crate::device::CURRENT_DEVICE
            .get_multitouch_placement()
            .rotation
            .rotated_size(&multitouch_orig_size)
            .into();

        // DEVICES
        let wacom_initial_dev = Arc::new(Mutex::new(Some(wacom_dev)));
        let multitouch_initial_dev = Arc::new(Mutex::new(Some(multitouch_dev)));
        let gpio_initial_dev = Arc::new(Mutex::new(Some(gpio_dev)));

        // Spawn a thread to remove close the initial devices after some time
        let wacom_initial_dev2 = wacom_initial_dev.clone();
        let multitouch_initial_dev2 = multitouch_initial_dev.clone();
        let gpio_initial_dev2 = gpio_initial_dev.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            // Remove devices (and thereby closing them)
            (*(*wacom_initial_dev2).lock().unwrap()) = None;
            (*(*multitouch_initial_dev2).lock().unwrap()) = None;
            (*(*gpio_initial_dev2).lock().unwrap()) = None;
            debug!("Closed initially opened evdev fds (if not used by now).");
        });

        Self {
            wacom_path,
            multitouch_path,
            gpio_path,

            wacom_width,
            wacom_height,

            mt_width,
            mt_height,

            multitouch_orig_size,
            wacom_orig_size,

            wacom_initial_dev,
            multitouch_initial_dev,
            gpio_initial_dev,
        }
    }

    /// Get the path to a InputDevice
    pub fn get_path(&self, device: InputDevice) -> &PathBuf {
        match device {
            InputDevice::Wacom => &self.wacom_path,
            InputDevice::Multitouch => &self.multitouch_path,
            InputDevice::GPIO => &self.gpio_path,
            InputDevice::Unknown => panic!("\"InputDevice::Unkown\" is no device!"),
        }
    }

    /// Get a ev device. If this is called early, it can get the device used for the initial scan.
    pub fn get_device(&self, device: InputDevice) -> Result<evdev::Device, impl std::error::Error> {
        let dev_arc = match device {
            InputDevice::Wacom => self.wacom_initial_dev.clone(),
            InputDevice::Multitouch => self.multitouch_initial_dev.clone(),
            InputDevice::GPIO => self.gpio_initial_dev.clone(),
            InputDevice::Unknown => panic!("\"InputDevice::Unkown\" is no device!"),
        };

        let mut resuable_device = dev_arc.lock().unwrap();
        if resuable_device.is_some() {
            let mut resuable_device = resuable_device.take().unwrap();
            resuable_device.events_no_sync()?; // Clear events until now
            Ok(resuable_device)
        } else {
            evdev::Device::open(self.get_path(device))
        }
    }
}
