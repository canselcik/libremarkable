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

/// The minimum height/width that we will enforce before each call to MXCFB_SEND_UPDATE
/// The higher it is, the more likely we are to have collisions between updates.
/// The smaller it is, the more likely we are to have display artifacts.
const MIN_SEND_UPDATE_DIMENSION_PX: u32 = 8;

impl<'a> framebuffer::FramebufferRefresh for core::Framebuffer<'a> {
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

        update_region.width = max!(update_region.width, MIN_SEND_UPDATE_DIMENSION_PX);
        update_region.height = max!(update_region.height, MIN_SEND_UPDATE_DIMENSION_PX);

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

    fn wait_refresh_complete(&mut self, marker: u32) -> u32 {
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
        return markerdata.collision_test;
    }
}
