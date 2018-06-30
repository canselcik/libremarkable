use hlua;
use std;

use framebuffer::common::*;
use framebuffer::core;

use framebuffer::refresh::PartialRefreshMode;

use framebuffer::FramebufferDraw;
use framebuffer::FramebufferIO;
use framebuffer::FramebufferRefresh;

/// We reluctantly resort to a static global here to associate the lua context
/// with the only active framebuffer we will have
pub static mut G_FB: *mut core::Framebuffer = std::ptr::null_mut();

/// A macro to utilize this static global only inside this file.
macro_rules! get_current_framebuffer {
    () => {
        unsafe { &mut *(G_FB as *mut core::Framebuffer) }
    };
}

pub fn lua_refresh(
    y: hlua::AnyLuaValue,
    x: hlua::AnyLuaValue,
    height: hlua::AnyLuaValue,
    width: hlua::AnyLuaValue,
    deep: hlua::AnyLuaValue,
    wait: hlua::AnyLuaValue,
) {
    if let (
        hlua::AnyLuaValue::LuaNumber(ny),
        hlua::AnyLuaValue::LuaNumber(nx),
        hlua::AnyLuaValue::LuaNumber(nheight),
        hlua::AnyLuaValue::LuaNumber(nwidth),
        hlua::AnyLuaValue::LuaBoolean(bdeep),
        hlua::AnyLuaValue::LuaBoolean(bwait),
    ) = (y, x, height, width, deep, wait)
    {
        let framebuffer = get_current_framebuffer!();
        let rect = mxcfb_rect {
            top: ny as u32,
            left: nx as u32,
            height: nheight as u32,
            width: nwidth as u32,
        };
        if bdeep {
            framebuffer.partial_refresh(
                &rect,
                if bwait {
                    PartialRefreshMode::Wait
                } else {
                    PartialRefreshMode::Async
                },
                waveform_mode::WAVEFORM_MODE_GC16_FAST,
                display_temp::TEMP_USE_PAPYRUS,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                false,
            );
        } else {
            framebuffer.partial_refresh(
                &rect,
                if bwait {
                    PartialRefreshMode::Wait
                } else {
                    PartialRefreshMode::Async
                },
                waveform_mode::WAVEFORM_MODE_DU,
                display_temp::TEMP_USE_REMARKABLE_DRAW,
                dither_mode::EPDC_FLAG_EXP1,
                DRAWING_QUANT_BIT,
                false,
            );
        }
    }
}

pub fn lua_draw_text(
    y: hlua::AnyLuaValue,
    x: hlua::AnyLuaValue,
    text: hlua::AnyLuaValue,
    size: hlua::AnyLuaValue,
    color: hlua::AnyLuaValue,
) {
    if let (
        hlua::AnyLuaValue::LuaNumber(ny),
        hlua::AnyLuaValue::LuaNumber(nx),
        hlua::AnyLuaValue::LuaString(stext),
        hlua::AnyLuaValue::LuaNumber(nsize),
        hlua::AnyLuaValue::LuaNumber(ncolor),
    ) = (y, x, text, size, color)
    {
        let framebuffer = get_current_framebuffer!();
        // TODO: Expose the drawn region to Lua so that it can be updated that's
        // returned from this draw_text function.
        framebuffer.draw_text(
            ny as usize,
            nx as usize,
            stext,
            nsize as usize,
            color::GRAY(ncolor as u8),
            false,
        );
    };
}

pub fn lua_set_pixel(y: hlua::AnyLuaValue, x: hlua::AnyLuaValue, color: hlua::AnyLuaValue) {
    if let (
        hlua::AnyLuaValue::LuaNumber(ny),
        hlua::AnyLuaValue::LuaNumber(nx),
        hlua::AnyLuaValue::LuaNumber(ncolor),
    ) = (y, x, color)
    {
        let framebuffer = get_current_framebuffer!();
        framebuffer.write_pixel(ny as usize, nx as usize, color::GRAY(ncolor as u8));
    }
}

pub fn lua_clear() {
    let framebuffer = get_current_framebuffer!();
    framebuffer.clear();
    framebuffer.full_refresh(
        waveform_mode::WAVEFORM_MODE_INIT,
        display_temp::TEMP_USE_AMBIENT,
        dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        true,
    );
}
