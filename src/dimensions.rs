#[cfg(feature = "input")]
use crate::input::scan::SCANNED;
#[cfg(feature = "input")]
use std::sync::LazyLock;

pub const DISPLAYWIDTH: u16 = 1404;
pub const DISPLAYHEIGHT: u16 = 1872;

/// Will be 767 rM1 and 1403 on the rM2
#[cfg(feature = "input")]
pub static MTWIDTH: LazyLock<u16> = LazyLock::new(|| SCANNED.mt_width);
/// Will be 1023 the rM1 and 1871 on the rM2
#[cfg(feature = "input")]
pub static MTHEIGHT: LazyLock<u16> = LazyLock::new(|| SCANNED.mt_height);

/// Will be 15725 on both the rM1 and rM2
#[cfg(feature = "input")]
pub static WACOMWIDTH: LazyLock<u16> = LazyLock::new(|| SCANNED.wacom_width);
/// Will be 20967 on the rM1 and 20966 on the rM2
#[cfg(feature = "input")]
pub static WACOMHEIGHT: LazyLock<u16> = LazyLock::new(|| SCANNED.wacom_height);
