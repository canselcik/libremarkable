use cgmath::{Point2, Vector2};

/// Describing the rotation of input devices.
pub enum InputDeviceRotation {
    /// When viewing the device in the standard portrait roation,
    /// the origin of this input device is on the top left
    Rot0,

    /// When viewing the device in the standard portrait roation,
    /// the origin of this input device is on the top right
    Rot90,

    /// When viewing the device in the standard portrait roation,
    /// the origin of this input device is on the bottom right
    Rot180,

    /// When viewing the device in the standard portrait roation,
    /// the origin of this input device is on the bottom left
    Rot270,
}

pub enum CoordinatePart {
    X(u16),
    Y(u16),
}

impl InputDeviceRotation {
    /// Takes a single coordinate part (`seg` == x or y coord) and returns the new coordinate
    /// depending on the rotation of the device.
    /// `size` must be the original size of the ev device that is used (the constants in framebuffer::common
    /// are already rotated to fit the typical portrait view of the framebuffer!).
    pub fn rotate_part(&self, seg: CoordinatePart, size: &Vector2<u16>) -> CoordinatePart {
        match seg {
            CoordinatePart::X(x) => match self {
                InputDeviceRotation::Rot0 => CoordinatePart::X(x),
                InputDeviceRotation::Rot90 => CoordinatePart::Y(x),
                InputDeviceRotation::Rot180 => CoordinatePart::X(size.x - x),
                InputDeviceRotation::Rot270 => CoordinatePart::Y(size.x - x),
            },
            CoordinatePart::Y(y) => match self {
                InputDeviceRotation::Rot0 => CoordinatePart::Y(y),
                InputDeviceRotation::Rot90 => CoordinatePart::X(size.y - y),
                InputDeviceRotation::Rot180 => CoordinatePart::Y(size.y - y),
                InputDeviceRotation::Rot270 => CoordinatePart::X(y),
            },
        }
    }

    /// Same as `rotate_part` but for a whole point.
    pub fn rotate_point(&self, point: &Point2<u16>, size: &Vector2<u16>) -> Point2<u16> {
        let rot_seg_x = self.rotate_part(CoordinatePart::X(point.x), size);
        let rot_seg_y = self.rotate_part(CoordinatePart::Y(point.y), size);

        if let CoordinatePart::X(x) = rot_seg_x {
            if let CoordinatePart::Y(y) = rot_seg_y {
                return Point2 { x, y };
            }
        }

        if let CoordinatePart::X(x) = rot_seg_y {
            if let CoordinatePart::Y(y) = rot_seg_x {
                return Point2 { x, y };
            }
        }

        unreachable!()
    }

    /// Whether based on the original rotation, width and height should be swapped.
    pub fn should_swap_size_axes(&self) -> bool {
        match self {
            InputDeviceRotation::Rot0 | InputDeviceRotation::Rot180 => false,
            InputDeviceRotation::Rot90 | InputDeviceRotation::Rot270 => true,
        }
    }

    /// Returns the same dimensions.
    /// These are swapped however when `should_swap_dimensions() == true`
    pub fn rotated_size(&self, src_size: &Vector2<u16>) -> Vector2<u16> {
        if self.should_swap_size_axes() {
            Vector2 {
                x: src_size.y,
                y: src_size.x,
            }
        } else {
            *src_size
        }
    }
}

#[cfg(test)]
mod test {
    use super::InputDeviceRotation::*;
    use cgmath::{Point2, Vector2};

    #[test]
    fn check_rotations() {
        let point = Point2 { x: 0, y: 0 };
        let size = Vector2 { x: 200, y: 100 };
        assert_eq!(Rot0.rotate_point(&point, &size), Point2 { x: 0, y: 0 });
        assert_eq!(Rot90.rotate_point(&point, &size), Point2 { x: 100, y: 0 });
        assert_eq!(
            Rot180.rotate_point(&point, &size),
            Point2 { x: 200, y: 100 }
        );
        assert_eq!(Rot270.rotate_point(&point, &size), Point2 { x: 0, y: 200 });
    }
}
