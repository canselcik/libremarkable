use crate::input::scan::SCANNED;

pub const DISPLAYWIDTH: u16 = 1404;
pub const DISPLAYHEIGHT: u16 = 1872;

lazy_static! {
    /// Will be 767 rM1 and 1403 on the rM2
    pub static ref MTWIDTH: u16 = SCANNED.mt_width;
    /// Will be 1023 the rM1 and 1871 on the rM2
    pub static ref MTHEIGHT: u16 = SCANNED.mt_height;

    /// Will be 15725 on both the rM1 and rM2
    pub static ref WACOMWIDTH: u16 = SCANNED.wacom_width;
    /// Will be 20967 on the rM1 and 20966 on the rM2
    pub static ref WACOMHEIGHT: u16 = SCANNED.wacom_height;
}
