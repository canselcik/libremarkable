use crate::input::rotate::InputDeviceRotation;
use once_cell::sync::Lazy;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Model {
    Gen1,
    Gen2,
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Model::Gen1 => write!(f, "reMarkable 1"),
            Model::Gen2 => write!(f, "reMarkable 2"),
        }
    }
}

impl Model {
    pub fn current_model() -> Result<Model, ErrorKind> {
        let content = std::fs::read_to_string("/sys/devices/soc0/machine")?;
        let machine_name = content.trim();
        // "reMarkable Prototype 1" was also seen for reMarkable 1 owners (and it didn't mean they preordered it).
        // See https://github.com/Eeems/oxide/issues/48#issuecomment-698414093
        if machine_name == "reMarkable 1.0" || machine_name == "reMarkable Prototype 1" {
            Ok(Model::Gen1)
        // https://github.com/Eeems/oxide/issues/48#issuecomment-698223552
        } else if machine_name == "reMarkable 2.0" {
            Ok(Model::Gen2)
        } else {
            Err(ErrorKind::UnknownVersion(machine_name.to_owned()))
        }
    }
}

pub static CURRENT_DEVICE: Lazy<Device> = Lazy::new(Device::new);

/// Differentiate between the reasons why the determination of the current device model can fail.
#[derive(Debug)]
pub enum ErrorKind {
    /// An IO error occured when reading /sys/devices/soc0/machine
    IOError(std::io::Error),
    /// The version string in /sys/devices/soc0/machine does not match any of the known versions.
    UnknownVersion(String),
}

impl From<std::io::Error> for ErrorKind {
    fn from(err: std::io::Error) -> Self {
        ErrorKind::IOError(err)
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::IOError(err) => err.fmt(f),
            ErrorKind::UnknownVersion(version) => {
                write!(f, "Unknown reMarkable version '{}'", version)
            }
        }
    }
}

/// Mainly information regarding both models
pub struct Device {
    pub model: Model,
}

/// The here specified roation and inversions should get the device into portrait
/// rotation where the origin (0, 0) is at the top left.
/// Scaling is not specified here, but Inputs will scale the axis to match the
/// size of the framebuffer.
pub struct InputDevicePlacement {
    /// What rotation is needed to get it into portrait rotation
    pub rotation: InputDeviceRotation,
    /// Whether to the x axis any axis AFTER a rotation was applied
    pub invert_x: bool,
    /// Whether to the y axis any axis AFTER a rotation was applied
    pub invert_y: bool,
}

impl Device {
    fn new() -> Self {
        let model = Model::current_model()
            .unwrap_or_else(|e| panic!("Got an error when determining model: {}", e));

        Self { model }
    }

    pub fn get_multitouch_placement(&self) -> InputDevicePlacement {
        match self.model {
            Model::Gen1 => InputDevicePlacement {
                rotation: InputDeviceRotation::Rot180,
                invert_x: false,
                invert_y: false,
            },
            Model::Gen2 => InputDevicePlacement {
                rotation: InputDeviceRotation::Rot180,
                invert_x: true,
                invert_y: false,
            },
        }
    }

    pub fn get_wacom_placement(&self) -> InputDevicePlacement {
        // The Wacom digitizer on Gen1 and Gen2 is placed the same
        InputDevicePlacement {
            rotation: InputDeviceRotation::Rot270,
            invert_x: false,
            invert_y: false,
        }
    }

    /// Name of the battery as found in /sys/class/power_supply
    pub fn get_internal_battery_name(&self) -> &str {
        match self.model {
            Model::Gen1 => "bq27441-0",
            Model::Gen2 => "max77818_battery",
        }
    }
}
