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

impl Device {
    fn new() -> Self {
        let model = Model::current_model()
            .unwrap_or_else(|e| panic!("Got IO Error when determining model: {}", e));
        if model == Model::Unknown {
            panic!("Failed to determine model!");
        }

        Self { model }
    }

    pub fn get_multitouch_rotation(&self) -> InputDeviceRotation {
        match self.model {
            Model::Gen1 => InputDeviceRotation::Rot180,
            Model::Gen2 => InputDeviceRotation::Rot270,
            Model::Unknown => unreachable!(),
        }
    }

    pub fn get_wacom_rotation(&self) -> InputDeviceRotation {
        // Not sure if the rotation of Gen2 differs
        InputDeviceRotation::Rot270
    }
}
