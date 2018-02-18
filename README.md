# Remarkable Framebuffer Analysis Project

This repository contains a collection of scripts, code and general information on what makes Remarkable Paper Tablet tick.


## RemarkableFramebufferSpy
A shared library that intercepts and displays undocumented framebuffer refresh ioctl calls for the Remarkable Paper Tablet.
Usage:
```sh
$ systemctl stop xochitl
$ LD_PRELOAD=./remarkableFramebufferSpy.so xochitl
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

