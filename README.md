# Remarkable Framebuffer Analysis Project

This repository contains a collection of scripts, code and general information on what makes Remarkable Paper Tablet tick.


## FrameBuffer Spy
A shared library that intercepts and displays undocumented framebuffer refresh ioctl calls for the Remarkable Paper Tablet.
Usage:
```sh
$ systemctl stop xochitl
$ LD_PRELOAD=./spy.so xochitl
...
12:06.842 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 191 ms (~DebugHelperClass() ../git/src/debug.h:16)
ioctl(3, 0x4048462e, 0x7ea2d290{
   updateRegion: x: 0
                 y: 0
                 width: 1404
                 height: 1872
   waveformMode: 3,
   updateMode:   0
   updateMarker: 45
   temp: 4096
   flags: 0000
   alt_buffer_data: 0x300f30
}) == 0
12:07.207 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 364 ms (~DebugHelperClass() ../git/src/debug.h:16)
12:07.384 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 175 ms (~DebugHelperClass() ../git/src/debug.h:16)
12:07.548 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 162 ms (~DebugHelperClass() ../git/src/debug.h:16)
12:07.705 DebugHelperClass    	 void DocumentWorker::loadCachedPage(int) 155 ms (~DebugHelperClass() ../git/src/debug.h:16)
ioctl(3, 0x4048462e, 0x7ea2d290{
   updateRegion: x: 0
                 y: 0
                 width: 1404
                 height: 1872
   waveformMode: 3,
   updateMode:   0
   updateMarker: 46
   temp: 4096
   flags: 0000
   alt_buffer_data: 0x300f30
}) == 0
```

## Additional Findings
Current framebuffer can be dumped with:
```bash
ssh root@10.11.99.1 "cat /dev/fb0" | convert -depth 16 -size 1408x1872+0 gray:- png:/tmp/frame;
```

Remarkable Paper Tablet has an undocumented API for partial refreshes on its eInk display, which is what's behind its magic that disappears when custom Qt applications are used to draw on the screen, even using the toolchain provided by Remarkable.

The `xochitl` program opens `/dev/fb0`, which always ends up being the `FD=3`. It then writes to this FD when it wants to update the screen and uses primarily the following `ioctl` call in order to perform its partial updates when the user draws on the device (`0x4048462e` is the `PARTIAL_UPDATE_MAGIC`, and the next argument is a pointer to `mxcfb_update_data`):

What is particularly interesting here is that `0x4048462e` also happens to be Kindle's `MXCFB_SEND_UPDATE` magic. Something to keep in mind.

```c
typedef struct {
  uint32_t top;    // 0x0000
  uint32_t left;   // 0x0000 
  uint32_t width;  // 0x0258 = 600
  uint32_t height; // 0x0320 = 800
} mxcfb_rect;

typedef struct {
  mxcfb_rect update_region;

  uint32_t waveform_mode;  // 0x0002 = WAVEFORM_MODE_GC16 
  uint32_t update_mode;    // 0x0000 = UPDATE_MODE_PARTIAL 
  uint32_t update_marker;  // 0x002a 
  
  int temp;   // 0x1001 = TEMP_USE_PAPYRUS
  uint flags; // 0x0000 
  
  void* alt_buffer_data; // must not used when flags is 0
} mxcfb_update_data;

ioctl(3, 0x4048462e, 0x7ea2d290{
   updateRegion: x: 0
                 y: 0
                 width: 1404
                 height: 1872
   waveformMode: 3,
   updateMode:   0
   updateMarker: 46
   temp: 4096
   flags: 0000
   alt_buffer_data: 0x300f30
}) == 0
```

## Partial Redraw Proof of Concept (poc)
Contains the proof of concept for directly interacting with the eink display driver to perform partial updates.
The key finding here is the following magic values and their usage in conjunction with the dumped `mxcfb_update_data` structure. Simply update the framebuffer and then call `ioctl` on the `/dev/fb0` FD with `REMARKABLE_PREFIX | MXCFB_SEND_UPDATE` and the redraw region set `data.update_region`.

```c
#define REMARKABLE_PREFIX                       0x40480000
#define MXCFB_SEND_UPDATE                       0x0000462e

// Untested
#define MXCFB_WAIT_FOR_VSYNC                    0x00004620
#define MXCFB_SET_GBL_ALPHA                     0x00004621
#define MXCFB_SET_CLR_KEY                       0x00004622
#define MXCFB_SET_OVERLAY_POS                   0x00004624
#define MXCFB_GET_FB_IPU_CHAN	                0x00004625
#define MXCFB_SET_LOC_ALPHA	                0x00004626
#define MXCFB_SET_LOC_ALP_BUF	                0x00004627
#define MXCFB_SET_GAMMA	                        0x00004628
#define MXCFB_GET_FB_IPU_DI	                0x00004629
#define MXCFB_GET_DIFMT	                        0x0000462a
#define MXCFB_GET_FB_BLANK	                0x0000462b
#define MXCFB_SET_WAVEFORM_MODES	        0x0000462b
#define MXCFB_SET_DIFMT	                        0x0000462c
#define MXCFB_SET_TEMPERATURE	                0x0000462c
#define MXCFB_SET_AUTO_UPDATE_MODE              0x0000462d
#define MXCFB_WAIT_FOR_UPDATE_COMPLETE	        0x0000462f
#define MXCFB_SET_PWRDOWN_DELAY	                0x00004630
#define MXCFB_GET_PWRDOWN_DELAY	                0x00004631
#define MXCFB_SET_UPDATE_SCHEME                 0x00004632
#define MXCFB_SET_MERGE_ON_WAVEFORM_MISMATCH    0x00004637

mxcfb_update_data data;
data.update_region.top = 0;
data.update_region.left = 0;
data.update_region.width = vinfo.xres;
data.update_region.height = vinfo.yres;
data.waveform_mode = 0x0002;
data.temp = 0x1001;
data.update_mode = 0x0000;
data.update_marker = 0x002a;
data.flags = 0;
data.alt_buffer_data = NULL;
ioctl(fb, REMARKABLE_PREFIX | MXCFB_SEND_UPDATE, &data);
```
