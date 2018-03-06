use libc;

use std::os::unix::io::AsRawFd;
use std::sync::atomic::Ordering;

use fb;
use mxc_types;
use mxc_types::{mxcfb_update_marker_data, mxcfb_update_data};

impl<'a> fb::Framebuffer<'a> {
    pub fn refresh(
        &mut self,
        mut update_region: mxc_types::mxcfb_rect,
        update_mode: mxc_types::update_mode,
        waveform_mode: mxc_types::waveform_mode,
        temperature: mxc_types::display_temp,
        dither_mode: mxc_types::dither_mode,
        quant_bit: i32,
        flags: u32,
    ) -> u32 {
        // No accounting for this, out of bounds, entirely ignored
        if update_region.left >= mxc_types::DISPLAYWIDTH as u32 ||
            update_region.top >= mxc_types::DISPLAYHEIGHT as u32
        {
            return 0;
        }

        // Dont try to refresh OOB horizontally
        let max_x = update_region.left + update_region.width;
        let x_overflow = max_x - mxc_types::DISPLAYWIDTH as u32;
        if x_overflow > 0 {
            update_region.width -= x_overflow;
        }

        // Dont try to refresh OOB vertically
        let max_y = update_region.top + update_region.height;
        let y_overflow = max_y - mxc_types::DISPLAYHEIGHT as u32;
        if y_overflow > 0 {
            update_region.height -= y_overflow;
        }

        let whole = mxcfb_update_data {
            update_mode: update_mode as u32,
            update_marker: *self.marker.get_mut() as u32,
            waveform_mode: waveform_mode as u32,
            temp: temperature as i32,
            flags: flags,
            quant_bit: quant_bit,
            dither_mode: dither_mode as i32,
            update_region: update_region,
            ..Default::default()
        };
        let pt: *const mxcfb_update_data = &whole;
        unsafe {
            libc::ioctl(self.device.as_raw_fd(), mxc_types::MXCFB_SEND_UPDATE, pt);
        }
        // TODO: Do proper compare and swap
        self.marker.swap(whole.update_marker + 1, Ordering::Relaxed);
        return whole.update_marker;
    }

    pub fn wait_refresh_complete(&mut self, marker: u32) {
        let mut markerdata = mxcfb_update_marker_data {
            update_marker: marker,
            collision_test: 0,
        };
        unsafe {
            libc::ioctl(
                self.device.as_raw_fd(),
                mxc_types::MXCFB_WAIT_FOR_UPDATE_COMPLETE,
                &mut markerdata,
            );
        };
        // TODO: Return collision test -- kernel updates it to the next marker's collision data
    }
}
