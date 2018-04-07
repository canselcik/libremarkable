use libc;

use std::os::unix::io::AsRawFd;
use std::sync::atomic::Ordering;

use framebuffer;
use framebuffer::common;
use framebuffer::core;
use framebuffer::mxcfb::*;

macro_rules! max {
        ($x: expr) => ($x);
        ($x: expr, $($z: expr),+) => (::std::cmp::max($x, max!($($z),*)));
}

impl<'a> framebuffer::FramebufferRefresh for core::Framebuffer<'a> {
    ///    1) PxP must process 8x8 pixel blocks, and all pixels in each block
    ///    are considered for auto-waveform mode selection. If the
    ///    update region is not 8x8 aligned, additional unwanted pixels
    ///    will be considered in auto-waveform mode selection.
    ///
    ///    2) PxP input must be 32-bit aligned, so any update
    ///    address not 32-bit aligned must be shifted to meet the
    ///    32-bit alignment.  The PxP will thus end up processing pixels
    ///    outside of the update region to satisfy this alignment restriction,
    ///    which can affect auto-waveform mode selection.
    ///
    ///    3) If input fails 32-bit alignment, and the resulting expansion
    ///    of the processed region would add at least 8 pixels more per
    ///    line than the original update line width, the EPDC would
    ///    cause screen artifacts by incorrectly handling the 8+ pixels
    ///    at the end of each line.
    fn refresh(
        &mut self,
        region: &common::mxcfb_rect,
        update_mode: common::update_mode,
        waveform_mode: common::waveform_mode,
        temperature: common::display_temp,
        dither_mode: common::dither_mode,
        quant_bit: i32,
        flags: u32,
    ) -> u32 {
        let mut update_region = region.clone();

        // No accounting for this, out of bounds, entirely ignored
        if update_region.left >= common::DISPLAYWIDTH as u32 ||
           update_region.top >= common::DISPLAYHEIGHT as u32 {
            return 0;
        }

        update_region.width = max!(update_region.width, 8);
        update_region.height = max!(update_region.height, 8);

        // Dont try to refresh OOB horizontally
        let max_x = update_region.left + update_region.width;
        if max_x > common::DISPLAYWIDTH as u32 {
            update_region.width -= max_x - (common::DISPLAYWIDTH as u32);
        }

        // Dont try to refresh OOB vertically
        let max_y = update_region.top + update_region.height;
        if max_y > common::DISPLAYHEIGHT as u32 {
            update_region.height -= max_y - (common::DISPLAYHEIGHT as u32);
        }

        let whole = mxcfb_update_data {
            update_mode: update_mode as u32,
            update_marker: *self.marker.get_mut() as u32,
            waveform_mode: waveform_mode as u32,
            temp: temperature as i32,
            flags,
            quant_bit,
            dither_mode: dither_mode as i32,
            update_region,
            ..Default::default()
        };
        let pt: *const mxcfb_update_data = &whole;
        unsafe {
            libc::ioctl(self.device.as_raw_fd(), common::MXCFB_SEND_UPDATE, pt);
        }
        // TODO: Do proper compare and swap
        self.marker.swap(whole.update_marker + 1, Ordering::Relaxed);
        return whole.update_marker;
    }

    fn wait_refresh_complete(&mut self, marker: u32) {
        let mut markerdata = mxcfb_update_marker_data {
            update_marker: marker,
            collision_test: 0,
        };
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                common::MXCFB_WAIT_FOR_UPDATE_COMPLETE,
                &mut markerdata,
            );
        };
        // TODO: Return collision test -- kernel updates it to the next marker's collision data
    }
}
