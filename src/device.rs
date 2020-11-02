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
        if content.contains('1') && !content.contains(".1") {
            return Ok(Model::Gen1);
        } else if content.contains('2') && !content.contains(".2") {
            return Ok(Model::Gen2);
        } else {
            return Ok(Model::Unknown);
        }
    }
}

lazy_static! {
    pub static ref DEVICE: Device = Device::new();
}

/// Mainly information regarding both models
pub struct Device {
    model: Model,
}

impl Device {
    fn new() -> Self {
        let model = Model::current_model()
            .unwrap_or_else(|e| panic!("Got IO Error when determining model: {}", e));
        if model == Model::Unknown {
            panic!("Failed to determine model!");
        }

        Self { model: model }
    }

    pub fn get_model(&self) -> Model {
        self.model
    }
}
