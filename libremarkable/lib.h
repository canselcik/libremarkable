#pragma once
#include <linux/fb.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>

typedef uint8_t remarkable_color;

#define REMARKABLE_DARKEST                      0x00
#define REMARKABLE_BRIGHTEST                    0xFF

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

// Untested
#define MXCFB_WAIT_FOR_VSYNC                    0x00004620
#define MXCFB_SET_GBL_ALPHA                     0x00004621
#define MXCFB_SET_CLR_KEY                       0x00004622
#define MXCFB_SET_OVERLAY_POS                   0x00004624
#define MXCFB_GET_FB_IPU_CHAN                   0x00004625
#define MXCFB_SET_LOC_ALPHA                     0x00004626
#define MXCFB_SET_LOC_ALP_BUF                   0x00004627
#define MXCFB_SET_GAMMA                         0x00004628
#define MXCFB_GET_FB_IPU_DI                     0x00004629
#define MXCFB_GET_DIFMT                         0x0000462a
#define MXCFB_GET_FB_BLANK                      0x0000462b
#define MXCFB_SET_WAVEFORM_MODES                0x0000462b
#define MXCFB_SET_DIFMT                         0x0000462c
#define MXCFB_SET_TEMPERATURE                   0x0000462c
#define MXCFB_SET_AUTO_UPDATE_MODE              0x0000462d
#define MXCFB_WAIT_FOR_UPDATE_COMPLETE          0x0000462f
#define MXCFB_SET_PWRDOWN_DELAY                 0x00004630
#define MXCFB_GET_PWRDOWN_DELAY                 0x00004631
#define MXCFB_SET_UPDATE_SCHEME                 0x00004632
#define MXCFB_SET_MERGE_ON_WAVEFORM_MISMATCH    0x00004637

typedef enum _update_mode
{
  UPDATE_MODE_PARTIAL   = 0x0,
  UPDATE_MODE_FULL      = 0x1
} update_mode;

typedef enum _waveform_mode {
  WAVEFORM_MODE_INIT         = 0x0,	                 /* Screen goes to white (clears) */
  WAVEFORM_MODE_GLR16			   = 4,                    /* Official */
  WAVEFORM_MODE_GLD16			   = 5,                    /* Official */

  // Unsupported?
  WAVEFORM_MODE_DU           = 0x1,	                 /* Grey->white/grey->black */
  WAVEFORM_MODE_GC16         = 0x2,	                 /* High fidelity (flashing) */
  WAVEFORM_MODE_GC4          = WAVEFORM_MODE_GC16,   /* For compatibility */
  WAVEFORM_MODE_GC16_FAST    = 0x3,                  /* Medium fidelity */
  WAVEFORM_MODE_GL16_FAST    = 0x6,                  /* Medium fidelity from white transition */
  WAVEFORM_MODE_DU4          = 0x7,	                 /* Medium fidelity 4 level of gray direct update */
  WAVEFORM_MODE_REAGL	       = 0x8,	                 /* Ghost compensation waveform */
  WAVEFORM_MODE_REAGLD       = 0x9,	                 /* Ghost compensation waveform with dithering */
  WAVEFORM_MODE_GL4		       = 0xA,	                 /* 2-bit from white transition */
  WAVEFORM_MODE_GL16_INV		 = 0xB,	                 /* High fidelity for black transition */
  WAVEFORM_MODE_AUTO			   = 257                   /* Official */
} waveform_mode;

typedef enum _display_temp {
  TEMP_USE_AMBIENT = 0x1000,
  TEMP_USE_PAPYRUS = 0X1001,
  TEMP_USE_MAX     = 0xFFFF
} display_temp;

typedef struct {
  uint32_t top;    // 0x0000
  uint32_t left;   // 0x0000
  uint32_t width;  // 0x0258 = 600
  uint32_t height; // 0x0320 = 800
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

  uint32_t waveform_mode;  // 0x0002 = WAVEFORM_MODE_GC16
  uint32_t update_mode;    // 0x0000 = UPDATE_MODE_PARTIAL
  uint32_t update_marker;  // 0x002a

  int temp;                // 0x1001 = TEMP_USE_PAPYRUS
  int flags;               // 0x0000

  int dither_mode;
	int quant_bit;

  mxcfb_alt_buffer_data alt_buffer_data;  // not used when flags is 0x0000
} mxcfb_update_data;

char* serialize_mxcfb_update_data(mxcfb_update_data* x);
void print_mxcfb_update_data(mxcfb_update_data* x);

// =======================





/*
  ===== ORIGINAL VSCREEN INFO =====
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

  ===== VSCREEN INFO AFTER XOCHITL MAKES ADJUSTMENTS =====
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
*/
/*
   finfo = { id = "mxc_epdc_fb\000\000\000\000", 
             smem_start = 2282749952,
             smem_len = 10813440,
             type = 0,
             type_aux = 0,
             visual = 2,
             xpanstep = 1,
             ypanstep = 1,
             ywrapstep = 0,
             line_length = 2816,
             mmio_start = 0,
             mmio_len = 0,
             accel = 0,
             capabilities = 0,
             reserved = {0, 0} }
*/

typedef struct {
  int fd;
  const char* fd_path;
  struct fb_var_screeninfo vinfo;
  struct fb_fix_screeninfo finfo;
  remarkable_color* mapped_buffer;
  unsigned len;
} remarkable_framebuffer;

remarkable_framebuffer* remarkable_framebuffer_init(const char* device_path);
void remarkable_framebuffer_destroy(remarkable_framebuffer* fb);

int  remarkable_framebuffer_set_pixel(remarkable_framebuffer* fb, unsigned y, unsigned x, remarkable_color c);
void remarkable_framebuffer_fill(remarkable_framebuffer* fb, remarkable_color color);

int  remarkable_framebuffer_refresh(remarkable_framebuffer* fb, mxcfb_rect* rect,
                                    update_mode refresh_mode, waveform_mode waveform,
                                    display_temp temp);

int  remarkable_framebuffer_wait_refresh_marker(remarkable_framebuffer* fb, uint32_t marker);