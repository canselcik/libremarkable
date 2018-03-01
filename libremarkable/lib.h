#pragma once
#include <linux/fb.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

// red     : offset = 11,  length =5,      msb_right = 0
// green   : offset = 5,   length =6,      msb_right = 0
// blue    : offset = 0,   length =5,      msb_right = 0
typedef uint8_t remarkable_color;
#define REMARKABLE_DARKEST                      0x00
#define REMARKABLE_BRIGHTEST                    0xFF
#define TO_REMARKABLE_COLOR(r, g, b)               ((r << 11) | (g << 5) | b)

#define YRES(remarkable_framebuffer_ptr)         remarkable_framebuffer_ptr->vinfo.yres 
#define YRES_VIRTUAL(remarkable_framebuffer_ptr) remarkable_framebuffer_ptr->vinfo.yres_virtual
#define XRES(remarkable_framebuffer_ptr)         remarkable_framebuffer_ptr->vinfo.xres 
#define XRES_VIRTUAL(remarkable_framebuffer_ptr) remarkable_framebuffer_ptr->vinfo.xres_virtual


// TODO: Figure out why this is used only when drawing (not for refresh) and only 
// when referring to width (not height, and not x-axis offset).
/*
  * GPU alignment restrictions dictate framebuffer parameters:
  * - 32-byte alignment for buffer width
  * - 128-byte alignment for buffer height
  * => 4K buffer alignment for buffer start
  */
#define to_remarkable_width(y) (y * 2)
#define from_remarkable_width(y) (y / 2)

#define to_remarkable_height(x) (x)
#define from_remarkable_height(x) (x)

// 0x4048 is the Remarkable Prefix
// 'F' (0x46) is the namespace
#define REMARKABLE_PREFIX(x) (0x40484600 | x)
typedef enum _eink_ioctl_command {
  MXCFB_SET_WAVEFORM_MODES	           = 0x2B, // takes struct mxcfb_waveform_modes
  MXCFB_SET_TEMPERATURE		             = 0x2C, // takes int32_t
  MXCFB_SET_AUTO_UPDATE_MODE           = 0x2D, // takes __u32
  MXCFB_SEND_UPDATE                    = 0x2E, // takes struct mxcfb_update_data
  MXCFB_WAIT_FOR_UPDATE_COMPLETE       = 0x2F, // takes struct mxcfb_update_marker_data
  MXCFB_SET_PWRDOWN_DELAY              = 0x30, // takes int32_t
  MXCFB_GET_PWRDOWN_DELAY              = 0x31, // takes int32_t
  MXCFB_SET_UPDATE_SCHEME              = 0x32, // takes __u32
  MXCFB_GET_WORK_BUFFER                = 0x34, // takes unsigned long
  MXCFB_SET_TEMP_AUTO_UPDATE_PERIOD    = 0x36, // takes int32_t
  MXCFB_DISABLE_EPDC_ACCESS            = 0x35,
  MXCFB_ENABLE_EPDC_ACCESS             = 0x36
} eink_ioctl_command;

typedef enum _auto_update_mode {
  AUTO_UPDATE_MODE_REGION_MODE         = 0,
  AUTO_UPDATE_MODE_AUTOMATIC_MODE      = 1
} auto_update_mode;

typedef enum _update_scheme {
  UPDATE_SCHEME_SNAPSHOT         = 0,
  UPDATE_SCHEME_QUEUE            = 1,
  UPDATE_SCHEME_QUEUE_AND_MERGE  = 2
} update_scheme;

typedef enum _update_mode
{
  UPDATE_MODE_PARTIAL   = 0,
  UPDATE_MODE_FULL      = 1
} update_mode;

typedef enum _waveform_mode {
  WAVEFORM_MODE_INIT         = 0x0,	                 /* Screen goes to white (clears) */
  WAVEFORM_MODE_GLR16			   = 0x4,                  /* Basically A2 (so partial refresh shouldnt be possible here) */
  WAVEFORM_MODE_GLD16			   = 0x5,                  /* Official -- and enables Regal D Processing */

  // Unsupported?
  WAVEFORM_MODE_DU           = 0x1,	                 /* [Direct Update] Grey->white/grey->black  -- remarkable uses this for drawing */
  WAVEFORM_MODE_GC16         = 0x2,	                 /* High fidelity (flashing) */
  WAVEFORM_MODE_GC4          = WAVEFORM_MODE_GC16,   /* For compatibility */
  WAVEFORM_MODE_GC16_FAST    = 0x3,                  /* Medium fidelity  -- remarkable uses this for UI */
  WAVEFORM_MODE_GL16_FAST    = 0x6,                  /* Medium fidelity from white transition */
  WAVEFORM_MODE_DU4          = 0x7,	                 /* Medium fidelity 4 level of gray direct update */
  WAVEFORM_MODE_REAGL	       = 0x8,	                 /* Ghost compensation waveform */
  WAVEFORM_MODE_REAGLD       = 0x9,	                 /* Ghost compensation waveform with dithering */
  WAVEFORM_MODE_GL4		       = 0xA,	                 /* 2-bit from white transition */
  WAVEFORM_MODE_GL16_INV		 = 0xB,	                 /* High fidelity for black transition */
  WAVEFORM_MODE_AUTO			   = 257                   /* Official */
} waveform_mode;

typedef enum _display_temp {
  TEMP_USE_AMBIENT           = 0x1000,
  TEMP_USE_PAPYRUS           = 0X1001,
  TEMP_USE_REMARKABLE_DRAW   = 0x0018,
  TEMP_USE_MAX               = 0xFFFF
} display_temp;

typedef struct {
  uint32_t top;
  uint32_t left;
  uint32_t width;
  uint32_t height;
} mxcfb_rect;

typedef struct {
	uint32_t update_marker;
	uint32_t collision_test;
} mxcfb_update_marker_data;

typedef struct {
	uint32_t phys_addr;
	uint32_t width;                   /* width of entire buffer */
	uint32_t height;	                /* height of entire buffer */
	mxcfb_rect alt_update_region;	    /* region within buffer to update */
} mxcfb_alt_buffer_data;

typedef struct {
	mxcfb_rect update_region;
  uint32_t waveform_mode;

  // Choose between FULL and PARTIAL
  uint32_t update_mode;

  // Checkpointing
  uint32_t update_marker;

  int temp;                         // 0x1001 = TEMP_USE_PAPYRUS
  unsigned int flags;               // 0x0000


  /*
   * Dither mode is entirely unused since the following means v1 is used not v2
   *
   * arch/arm/configs/zero-gravitas_defconfig
      173:CONFIG_FB_MXC_EINK_PANEL=y

     firmware/Makefile
      68:fw-shipped-$(CONFIG_FB_MXC_EINK_PANEL) += \

     drivers/video/fbdev/mxc/mxc_epdc_fb.c
      4969:#ifdef CONFIG_FB_MXC_EINK_AUTO_UPDATE_MODE
      5209:#ifdef CONFIG_FB_MXC_EINK_AUTO_UPDATE_MODE

     drivers/video/fbdev/mxc/mxc_epdc_v2_fb.c
      5428:#ifdef CONFIG_FB_MXC_EINK_AUTO_UPDATE_MODE
      5662:#ifdef CONFIG_FB_MXC_EINK_AUTO_UPDATE_MODE

     drivers/video/fbdev/mxc/Makefile
      10:obj-$(CONFIG_FB_MXC_EINK_PANEL)      += mxc_epdc_fb.o
      11:obj-$(CONFIG_FB_MXC_EINK_V2_PANEL)   += mxc_epdc_v2_fb.o
   *
   */
  int dither_mode;
	int quant_bit; // used only when dither_mode is > PASSTHROUGH and < MAX

  mxcfb_alt_buffer_data alt_buffer_data;  // not used when flags is 0x0000
} mxcfb_update_data;

typedef enum _mxcfb_dithering_mode {
	EPDC_FLAG_USE_DITHERING_PASSTHROUGH = 0x0,
	EPDC_FLAG_USE_DITHERING_DRAWING     = 0x1,
	// Dithering Processing (Version 1.0 - for i.MX508 and i.MX6SL)
  EPDC_FLAG_USE_DITHERING_Y1          = 0x002000,
  EPDC_FLAG_USE_REMARKABLE_DITHER     = 0x300f30,
  EPDC_FLAG_USE_DITHERING_Y4          = 0x004000

} mxcfb_dithering_mode;


////// fb_data->epdc_fb_var.grayscale
#define GRAYSCALE_8BIT                          0x1
#define GRAYSCALE_8BIT_INVERTED                 0x2
#define GRAYSCALE_4BIT                          0x3
#define GRAYSCALE_4BIT_INVERTED                 0x4

////// FLAGS
/*
* If no processing required, skip update processing
* No processing means:
*   - FB unrotated
*   - FB pixel format = 8-bit grayscale
*   - No look-up transformations (inversion, posterization, etc.)
*/
// Enables PXP_LUT_INVERT transform on the buffer
#define EPDC_FLAG_ENABLE_INVERSION              0x0001
// Enables PXP_LUT_BLACK_WHITE transform on the buffer
#define EPDC_FLAG_FORCE_MONOCHROME              0x0002
// Enables PXP_USE_CMAP transform on the buffer
#define EPDC_FLAG_USE_CMAP                      0x0004

// This is basically double buffering. We give it the bitmap we want to
// update, it swaps them.
#define EPDC_FLAG_USE_ALT_BUFFER                0x0100

// An update won't be merged upon a conflict in case of a collusion if
// either update has this flag set, unless they are identical regions (same y,x,h,w)
#define EPDC_FLAG_TEST_COLLISION                0x0200
#define EPDC_FLAG_GROUP_UPDATE                  0x0400

// Both are verified to be respected by the flags in the update_data

#define DRAWING_QUANT_BIT 0x76143b24

/*
  vinfo:
    xres            = 1404  yres            = 1872
    xres_virtual    = 1408  yres_virtual    = 3840
    xoffset         = 0     yoffset         = 0
    bits_per_pixel  = 16    grayscale       = 0
    red     : offset = 11,  length =5,      msb_right = 0
    green   : offset = 5,   length =6,      msb_right = 0
    blue    : offset = 0,   length =5,      msb_right = 0
    transp  : offset = 0,   length =0,      msb_right = 0
    nonstd          = 0
    activate        = 128
    height          = 0xffffffff
    width           = 0xffffffff
    accel_flags(OBSOLETE) = 0
    pixclock        = 6250
    left_margin     = 32
    right_margin    = 326
    upper_margin    = 4
    lower_margin    = 12
    hsync_len       = 44
    vsync_len       = 1
    sync            = 0
    vmode           = 0
    rotate          = 1
    colorspace      = 0

  finfo:
    id = "mxc_epdc_fb\000\000\000\000",
    smem_start = 2282749952
    smem_len = 10813440
    type = 0
    type_aux = 0
    visual = 2
    xpanstep = 1
    ypanstep = 1
    ywrapstep = 0
    line_length = 2816
    mmio_start = 0
    mmio_len = 0
    accel = 0
    capabilities = 0
    reserved = {0, 0}
*/

typedef struct {
  int fd;
  const char* fd_path;
  struct fb_var_screeninfo vinfo;
  struct fb_fix_screeninfo finfo;
  remarkable_color* mapped_buffer;
  unsigned len;
} remarkable_framebuffer;

/* fb.c */
remarkable_framebuffer* remarkable_framebuffer_init(const char* device_path);
int  remarkable_framebuffer_set_epdc_access(remarkable_framebuffer* fb, int enabled);
int  remarkable_framebuffer_set_auto_update_mode(remarkable_framebuffer* fb, auto_update_mode mode);
int  remarkable_framebuffer_set_auto_update_period(remarkable_framebuffer* fb, int period);
int  remarkable_framebuffer_set_update_scheme(remarkable_framebuffer* fb, update_scheme scheme);
void remarkable_framebuffer_destroy(remarkable_framebuffer* fb);
int  remarkable_framebuffer_set_pixel(remarkable_framebuffer* fb, const unsigned y, const unsigned x, const remarkable_color c);
void remarkable_framebuffer_draw_shape(remarkable_framebuffer* fb, remarkable_color* shape, unsigned rows,   unsigned cols,
                                                                                            unsigned y,      unsigned x,
                                                                                            unsigned height, unsigned width);
void remarkable_framebuffer_draw_rect(remarkable_framebuffer* fb, mxcfb_rect rect, remarkable_color color);
void remarkable_framebuffer_fill(remarkable_framebuffer* fb, remarkable_color color);

/* refresh.c */
uint32_t remarkable_framebuffer_refresh(remarkable_framebuffer* fb,
                                        update_mode refresh_mode,
                                        waveform_mode waveform,
                                        display_temp temp,
                                        mxcfb_dithering_mode dither_mode,
                                        int flags,
                                        unsigned int quant_bit,
                                        mxcfb_alt_buffer_data* alt_buffer_data,
                                        unsigned y, unsigned x,
                                        unsigned height, unsigned width);
uint32_t remarkable_framebuffer_wait_refresh_marker(remarkable_framebuffer* fb, uint32_t marker);

/* serde.c */
char* serialize_mxcfb_update_data(mxcfb_update_data* x);
void  print_mxcfb_update_data(mxcfb_update_data* x);

/* freetype.c */
struct remarkable_font;

struct remarkable_font*    remarkable_framebuffer_font_init(remarkable_framebuffer* fb, const char* fontFilename, unsigned target_height);
mxcfb_rect                 remarkable_framebuffer_draw_text(remarkable_framebuffer* fb,
                                                            struct remarkable_font* font,
                                                            const char* text,
                                                            unsigned top, unsigned left);
void                       remarkable_framebuffer_font_destroy(struct remarkable_font* font);
