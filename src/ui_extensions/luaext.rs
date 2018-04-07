use std;
use hlua;

use framebuffer::common::*;
use framebuffer::core;

use framebuffer::FramebufferRefresh;
use framebuffer::FramebufferIO;
use framebuffer::FramebufferDraw;

/// We reluctantly resort to a static global here to associate the lua context
/// with the only active framebuffer we will have
pub static mut G_FB: *mut core::Framebuffer = std::ptr::null_mut();

/// A macro to utilize this static global only inside this file.
macro_rules! get_current_framebuffer {
    () => (
        unsafe {
            std::mem::transmute::<* mut core::Framebuffer, &mut core::Framebuffer>(G_FB)
        }
    )
}

pub fn lua_refresh(y: hlua::AnyLuaValue, x: hlua::AnyLuaValue, height: hlua::AnyLuaValue,
               width: hlua::AnyLuaValue, deep: hlua::AnyLuaValue, wait: hlua::AnyLuaValue) {
    let framebuffer = get_current_framebuffer!();
    match (y, x, height, width, deep, wait) {
        (hlua::AnyLuaValue::LuaNumber(ny),
         hlua::AnyLuaValue::LuaNumber(nx),
         hlua::AnyLuaValue::LuaNumber(nheight),
         hlua::AnyLuaValue::LuaNumber(nwidth),
         hlua::AnyLuaValue::LuaBoolean(bdeep),
         hlua::AnyLuaValue::LuaBoolean(bwait)) => {
            let rect = mxcfb_rect {
                top: ny as u32,
                left: nx as u32,
                height: nheight as u32,
                width: nwidth as u32,
            };
            let marker = match bdeep {
                false => framebuffer.refresh(
                    &rect,
                    update_mode::UPDATE_MODE_PARTIAL,
                    waveform_mode::WAVEFORM_MODE_DU,
                    display_temp::TEMP_USE_REMARKABLE_DRAW,
                    dither_mode::EPDC_FLAG_EXP1,
                    DRAWING_QUANT_BIT,
                    0,
                ),
                true => framebuffer.refresh(
                    &rect,
                    update_mode::UPDATE_MODE_PARTIAL,
                    waveform_mode::WAVEFORM_MODE_GC16_FAST,
                    display_temp::TEMP_USE_PAPYRUS,
                    dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                    0,
                    0,
                ),
            };
            if bwait {
                framebuffer.wait_refresh_complete(marker);
            }
        },
        _ => {},
    };
}

pub fn lua_draw_text(y: hlua::AnyLuaValue, x: hlua::AnyLuaValue, text: hlua::AnyLuaValue, size: hlua::AnyLuaValue, color: hlua::AnyLuaValue) {
    let framebuffer = get_current_framebuffer!();
    match (y, x, text, size, color) {
        (hlua::AnyLuaValue::LuaNumber(ny),
         hlua::AnyLuaValue::LuaNumber(nx),
         hlua::AnyLuaValue::LuaString(stext),
         hlua::AnyLuaValue::LuaNumber(nsize),
         hlua::AnyLuaValue::LuaNumber(ncolor)) => {
            // TODO: Expose the drawn region to Lua so that it can be updated
            let _rect = framebuffer.draw_text(ny as usize,
                                              nx as usize,
                                              stext,
                                              nsize as usize,
                                              ncolor as u8);
        },
        _ => {},
    };
}

pub fn lua_set_pixel(y: hlua::AnyLuaValue, x: hlua::AnyLuaValue, color: hlua::AnyLuaValue) {
    let framebuffer = get_current_framebuffer!();
    match (y, x, color) {
        (hlua::AnyLuaValue::LuaNumber(ny),
         hlua::AnyLuaValue::LuaNumber(nx),
         hlua::AnyLuaValue::LuaNumber(ncolor)) => framebuffer.write_pixel(ny as usize, nx as usize, ncolor as u8),
        _ => {},
    };
}

pub fn lua_clear() {
    let framebuffer = get_current_framebuffer!();
    let (yres, xres) = (
        framebuffer.var_screen_info.yres,
        framebuffer.var_screen_info.xres,
    );

    framebuffer.clear();

    let marker = framebuffer.refresh(
        &mxcfb_rect {
            top: 0,
            left: 0,
            height: yres,
            width: xres,
        },
        update_mode::UPDATE_MODE_FULL,
        waveform_mode::WAVEFORM_MODE_INIT,
        display_temp::TEMP_USE_AMBIENT,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0, 0,
    );
    framebuffer.wait_refresh_complete(marker);
}