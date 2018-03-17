use hlua;
use fb;
use std;
use mxc_types;
use uix;

pub fn init(ctx: &mut uix::ApplicationContext) {
    let rawfb = ctx.get_framebuffer_ptr();
    let lua = ctx.get_lua_context();
    let mut nms = lua.empty_array("fb");
    {
        nms.set("print_vinfo", hlua::function0(move || {
            let framebuffer = unsafe {
                std::mem::transmute_copy::<_, &mut fb::Framebuffer>(&rawfb)
            };
            println!("Printing vinfo from lua context: {0:#?}", framebuffer.var_screen_info);
        }));

        nms.set("clear", hlua::function0(move || {
            let framebuffer = unsafe {
                std::mem::transmute_copy::<_, &mut fb::Framebuffer>(&rawfb)
            };
            let (yres, xres) = (
                framebuffer.var_screen_info.yres,
                framebuffer.var_screen_info.xres,
            );
            framebuffer.clear();

            let marker = framebuffer.refresh(
                mxc_types::mxcfb_rect {
                    top: 0,
                    left: 0,
                    height: yres,
                    width: xres,
                },
                mxc_types::update_mode::UPDATE_MODE_FULL,
                mxc_types::waveform_mode::WAVEFORM_MODE_INIT,
                mxc_types::display_temp::TEMP_USE_AMBIENT,
                mxc_types::dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0, 0,
            );
            framebuffer.wait_refresh_complete(marker);
        }));
    }
}