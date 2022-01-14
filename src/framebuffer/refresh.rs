use std::os::unix::io::AsRawFd;
use std::sync::atomic::Ordering;

use log::warn;

use crate::framebuffer;
use crate::framebuffer::common;
use crate::framebuffer::core;
use crate::framebuffer::core::FramebufferUpdate;
use crate::framebuffer::mxcfb::*;

pub enum PartialRefreshMode {
    DryRun,
    Async,
    Wait,
}

impl framebuffer::FramebufferRefresh for core::Framebuffer {
    fn full_refresh(
        &self,
        waveform_mode: common::waveform_mode,
        temperature: common::display_temp,
        dither_mode: common::dither_mode,
        quant_bit: i32,
        wait_completion: bool,
    ) -> u32 {
        let screen = common::mxcfb_rect {
            top: 0,
            left: 0,
            height: self.var_screen_info.yres,
            width: self.var_screen_info.xres,
        };
        let marker = self.marker.fetch_add(1, Ordering::Relaxed);
        let whole = mxcfb_update_data {
            update_mode: common::update_mode::UPDATE_MODE_FULL as u32,
            update_marker: marker as u32,
            waveform_mode: waveform_mode as u32,
            temp: temperature as i32,
            flags: 0,
            quant_bit,
            dither_mode: dither_mode as i32,
            update_region: screen,
            ..Default::default()
        };

        let update_succeeded = match &self.framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                let pt: *const mxcfb_update_data = &whole;
                (unsafe { libc::ioctl(device.as_raw_fd(), common::MXCFB_SEND_UPDATE, pt) }) >= 0
            }
            FramebufferUpdate::Swtfb(swtfb_client) => swtfb_client.send_mxcfb_update(&whole),
        };

        if !update_succeeded {
            warn!("Sending full_refresh update failed!")
        }

        if wait_completion {
            self.wait_refresh_complete(whole.update_marker)
        } else {
            whole.update_marker
        }
    }

    fn partial_refresh(
        &self,
        region: &common::mxcfb_rect,
        mode: PartialRefreshMode,
        waveform_mode: common::waveform_mode,
        temperature: common::display_temp,
        dither_mode: common::dither_mode,
        quant_bit: i32,
        force_full_refresh: bool,
    ) -> u32 {
        let mut update_region = region.to_owned();

        // No accounting for this, out of bounds, entirely ignored
        if update_region.left >= self.var_screen_info.xres
            || update_region.top >= self.var_screen_info.yres
        {
            return 0;
        }

        if update_region.width < 1 {
            update_region.width = 1
        }
        if update_region.height < 1 {
            update_region.height = 1
        }

        // Dont try to refresh OOB horizontally
        let max_x = update_region.left + update_region.width;
        if max_x > self.var_screen_info.xres {
            update_region.width -= max_x - self.var_screen_info.xres;
        }

        // Dont try to refresh OOB vertically
        let max_y = update_region.top + update_region.height;
        if max_y > self.var_screen_info.yres {
            update_region.height -= max_y - self.var_screen_info.yres;
        }

        let update_mode = if force_full_refresh {
            common::update_mode::UPDATE_MODE_FULL as u32
        } else {
            common::update_mode::UPDATE_MODE_PARTIAL as u32
        };

        let marker = self.marker.fetch_add(1, Ordering::Relaxed);
        let whole = mxcfb_update_data {
            update_mode,
            update_marker: marker as u32,
            waveform_mode: waveform_mode as u32,
            temp: temperature as i32,
            flags: match mode {
                PartialRefreshMode::DryRun => common::EPDC_FLAG_TEST_COLLISION as u32,
                _ => 0,
            },
            quant_bit,
            dither_mode: dither_mode as i32,
            update_region,
            ..Default::default()
        };

        let update_succeeded = match &self.framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                let pt: *const mxcfb_update_data = &whole;
                (unsafe { libc::ioctl(device.as_raw_fd(), common::MXCFB_SEND_UPDATE, pt) }) >= 0
            }
            FramebufferUpdate::Swtfb(swtfb_client) => swtfb_client.send_mxcfb_update(&whole),
        };

        if !update_succeeded {
            warn!("Sending partial_refresh update failed!")
        }

        match mode {
            PartialRefreshMode::Wait | PartialRefreshMode::DryRun => {
                self.wait_refresh_complete(whole.update_marker)
            }
            PartialRefreshMode::Async => whole.update_marker,
        }
    }

    fn wait_refresh_complete(&self, update_marker: u32) -> u32 {
        match &self.framebuffer_update {
            FramebufferUpdate::Ioctl(device) => {
                let mut markerdata = mxcfb_update_marker_data {
                    update_marker,
                    collision_test: 0,
                };
                if (unsafe {
                    libc::ioctl(
                        device.as_raw_fd(),
                        common::MXCFB_WAIT_FOR_UPDATE_COMPLETE,
                        &mut markerdata,
                    )
                }) < 0
                {
                    warn!("WAIT_FOR_UPDATE_COMPLETE failed");
                }
                markerdata.collision_test
            }
            FramebufferUpdate::Swtfb(swtfb_client) => {
                swtfb_client.wait_for_update_complete();
                // Assume success
                0
            }
        }
    }
}
