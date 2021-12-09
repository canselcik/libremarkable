pub mod common;
pub mod mxcfb;
pub mod screeninfo;

pub mod storage;

pub mod io;

pub mod swtfb_client;

pub use cgmath;

pub trait FramebufferIO {
    /// Writes an arbitrary length frame into the framebuffer
    fn write_frame(&mut self, frame: &[u8]);
    /// Writes a single pixel at `pos` with value `v`
    fn write_pixel(&mut self, pos: cgmath::Point2<i32>, v: common::color);
    /// Reads the value of the pixel at `pos`
    fn read_pixel(&self, pos: cgmath::Point2<u32>) -> common::color;
    /// Reads the value at offset `ofst` from the mmapp'ed framebuffer region
    fn read_offset(&self, ofst: isize) -> u8;
    /// Dumps the contents of the specified rectangle into a `Vec<u8>` from which
    /// you can later create a CompressedCanvasState or pass to restore_region().
    /// The pixel format is rgb565_le.
    fn dump_region(&self, rect: common::mxcfb_rect) -> Result<Vec<u8>, &'static str>;
    /// Restores into the framebuffer the contents of the specified rectangle from a u8 slice
    fn restore_region(
        &mut self,
        rect: common::mxcfb_rect,
        data: &[u8],
    ) -> Result<u32, &'static str>;
}

mod graphics;

pub mod draw;
pub trait FramebufferDraw {
    /// Draws `img` at `pos` with 1:1 scaling
    fn draw_image(&mut self, img: &image::RgbImage, pos: cgmath::Point2<i32>)
        -> common::mxcfb_rect;
    /// Draws a straight line
    fn draw_line(
        &mut self,
        start: cgmath::Point2<i32>,
        end: cgmath::Point2<i32>,
        width: u32,
        v: common::color,
    ) -> common::mxcfb_rect;
    /// Draws a circle using Bresenham circle algorithm
    fn draw_circle(
        &mut self,
        pos: cgmath::Point2<i32>,
        rad: u32,
        c: common::color,
    ) -> common::mxcfb_rect;
    /// Fills a circle
    fn fill_circle(
        &mut self,
        pos: cgmath::Point2<i32>,
        rad: u32,
        c: common::color,
    ) -> common::mxcfb_rect;
    /// Draws a polygon
    fn draw_polygon(
        &mut self,
        _: &[cgmath::Point2<i32>],
        fill: bool,
        c: common::color,
    ) -> common::mxcfb_rect;
    /// Draws a bezier curve begining at `startpt`, with control point `ctrlpt`, ending at `endpt` with `color`
    fn draw_bezier(
        &mut self,
        startpt: cgmath::Point2<f32>,
        ctrlpt: cgmath::Point2<f32>,
        endpt: cgmath::Point2<f32>,
        width: f32,
        samples: i32,
        v: common::color,
    ) -> common::mxcfb_rect;
    /// Draws a bezier curve begining at `startpt`, with control point `ctrlpt`, ending at `endpt`
    /// with a width at each point and color `color`
    fn draw_dynamic_bezier(
        &mut self,
        startpt: (cgmath::Point2<f32>, f32),
        ctrlpt: (cgmath::Point2<f32>, f32),
        endpt: (cgmath::Point2<f32>, f32),
        samples: i32,
        v: common::color,
    ) -> common::mxcfb_rect;
    /// Draws `text` at `pos` with `color` using scale `size`
    fn draw_text(
        &mut self,
        pos: cgmath::Point2<f32>,
        text: &str,
        size: f32,
        col: common::color,
        dryrun: bool,
    ) -> common::mxcfb_rect;
    /// Draws a 1px border rectangle of size `size` at `pos` with `border_px` border thickness
    fn draw_rect(
        &mut self,
        pos: cgmath::Point2<i32>,
        size: cgmath::Vector2<u32>,
        border_px: u32,
        c: common::color,
    );
    /// Fills rectangle of size `size` at `pos`
    fn fill_rect(&mut self, pos: cgmath::Point2<i32>, size: cgmath::Vector2<u32>, c: common::color);
    /// Clears the framebuffer however does not perform a refresh
    fn clear(&mut self);
}

pub mod core;
pub trait FramebufferBase<'a> {
    /// Creates a new instance of Framebuffer
    fn from_path(path_to_device: &str) -> core::Framebuffer<'_>;
    /// Toggles the EPD Controller (see https://wiki.mobileread.com/wiki/EPD_controller)
    fn set_epdc_access(&mut self, state: bool);
    /// Toggles autoupdate mode
    fn set_autoupdate_mode(&mut self, mode: u32);
    /// Toggles update scheme
    fn set_update_scheme(&mut self, scheme: u32);
    /// Creates a FixScreeninfo struct and fills it using ioctl
    fn get_fix_screeninfo(
        device: &std::fs::File,
        swtfb_client: Option<&swtfb_client::SwtfbClient>,
    ) -> screeninfo::FixScreeninfo;
    /// Creates a VarScreeninfo struct and fills it using ioctl
    fn get_var_screeninfo(
        device: &std::fs::File,
        swtfb_client: Option<&swtfb_client::SwtfbClient>,
    ) -> screeninfo::VarScreeninfo;
    /// Makes the proper ioctl call to set the VarScreenInfo.
    /// You must first update the contents of self.var_screen_info
    /// and then call this function.
    fn put_var_screeninfo(
        device: &std::fs::File,
        swtfb_client: Option<&swtfb_client::SwtfbClient>,
        var_screen_info: &mut screeninfo::VarScreeninfo,
    ) -> bool;

    fn update_var_screeninfo(&mut self) -> bool;
}

pub mod refresh;
pub trait FramebufferRefresh {
    /// Refreshes the entire screen with the provided parameters. If `wait_completion` is
    /// set to true, doesn't return before the refresh has been completed. Returns the marker.
    fn full_refresh(
        &self,
        waveform_mode: common::waveform_mode,
        temperature: common::display_temp,
        dither_mode: common::dither_mode,
        quant_bit: i32,
        wait_completion: bool,
    ) -> u32;

    /// Refreshes the given `region` with the provided parameters. If `mode` is `DryRun` or
    /// `Wait`, this function won't return before the `DryRun`'s collision_test or
    /// refresh has been completed. In `Async` mode, this function will return immediately
    /// and return a `marker` which can then later be fed to `wait_refresh_complete` to wait
    /// for its completion. In `DryRun`, it will return the `collision_test` result.
    ///
    /// `force_full_refresh` allows rare cases where you may want to do a full refresh on a
    /// partial region. 99.9% of of the time, you want this set to `false`.
    ///
    /// Some additional points to note:
    ///
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
    #[allow(clippy::too_many_arguments)]
    fn partial_refresh(
        &self,
        region: &common::mxcfb_rect,
        mode: refresh::PartialRefreshMode,
        waveform_mode: common::waveform_mode,
        temperature: common::display_temp,
        dither_mode: common::dither_mode,
        quant_bit: i32,
        force_full_refresh: bool,
    ) -> u32;

    /// Takes a marker returned by `partial_refresh` and blocks until that
    /// refresh has been reflected on the display.
    /// Returns the collusion_test result which is supposed to be
    /// related to the collusion information.
    fn wait_refresh_complete(&self, marker: u32) -> u32;
}
