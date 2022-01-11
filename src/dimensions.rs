use crate::input::scan::SCANNED;
use once_cell::sync::Lazy;

pub const DISPLAYWIDTH: u16 = 1404;
pub const DISPLAYHEIGHT: u16 = 1872;

/// Will be 767 rM1 and 1403 on the rM2
pub static MTWIDTH: Lazy<u16> = Lazy::new(|| SCANNED.mt_width);
/// Will be 1023 the rM1 and 1871 on the rM2
pub static MTHEIGHT: Lazy<u16> = Lazy::new(|| SCANNED.mt_height);

/// Will be 15725 on both the rM1 and rM2
pub static WACOMWIDTH: Lazy<u16> = Lazy::new(|| SCANNED.wacom_width);
/// Will be 20967 on the rM1 and 20966 on the rM2
pub static WACOMHEIGHT: Lazy<u16> = Lazy::new(|| SCANNED.wacom_height);
