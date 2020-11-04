use super::ecodes;
use super::InputDevice;
use std::path::{Path, PathBuf};

lazy_static! {
    /// A singleton of the EvDevsScan object
    pub static ref SCAN: EvDevsScan = EvDevsScan::new();
}

/// This struct contains the results of initially scaning all evdev devices,
/// which allows for device model independancy.
/// Some of its data is used by other constants.
///
/// EvDevsScan has some internal mutability to allow resuing the opened devices
/// for some time to increase performance.
/// TODO: Call this `EvDevsScanOutcome` or EvScanOutcome instead ??
pub struct EvDevsScan {
    pub wacom_path: PathBuf,
    pub multitouch_path: PathBuf,
    pub gpio_path: PathBuf,

    pub wacom_width: u16,
    pub wacom_height: u16,
    pub mt_width: u16,
    pub mt_height: u16,
}

impl EvDevsScan {
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

        // Figure out sizes
        let wacom_state = wacom_dev.state();
        // X and Y are swapped for the wacom since rM1 and rM2 have it rotated
        let wacom_width = wacom_state.abs_vals[ecodes::ABS_Y as usize].maximum as u16;
        let wacom_height = wacom_state.abs_vals[ecodes::ABS_X as usize].maximum as u16;

        let mt_state = multitouch_dev.state();
        let mt_width = mt_state.abs_vals[ecodes::ABS_MT_POSITION_X as usize].maximum as u16;
        let mt_height = mt_state.abs_vals[ecodes::ABS_MT_POSITION_Y as usize].maximum as u16;

        Self {
            wacom_path,
            multitouch_path,
            gpio_path,

            wacom_width,
            wacom_height,

            mt_width,
            mt_height,
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
}
