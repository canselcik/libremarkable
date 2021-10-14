use crate::input::rotate::InputDeviceRotation;
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Model {
    Gen1,
    Gen2,
    Unknown,
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Model::Gen1 => write!(f, "reMarkable 1"),
            Model::Gen2 => write!(f, "reMarkable 2"),
            Model::Unknown => write!(f, "Unknown reMarkable"),
        }
    }
}

impl Model {
    fn current_model() -> std::io::Result<Model> {
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
            Ok(Model::Unknown)
        }
    }
}

lazy_static! {
    pub static ref CURRENT_DEVICE: Device = Device::new();
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
            .unwrap_or_else(|e| panic!("Got IO Error when determining model: {}", e));
        assert!(model != Model::Unknown, "Failed to determine model!");

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
            Model::Unknown => unreachable!(),
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
            Model::Unknown => unreachable!(),
        }
    }
}
