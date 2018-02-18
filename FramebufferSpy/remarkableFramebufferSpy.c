#define _GNU_SOURCE 1
#include <string.h>
#include <stdio.h>
#include <stdint.h>
#include <stdarg.h>
#include <dlfcn.h>
#include <sys/types.h>
#include <sys/stat.h>

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

void print_mxcfb_update_data(mxcfb_update_data* x)
{
  printf("{\n"
         "   updateRegion: x: %u\n"
         "                 y: %u\n"
         "                 width: %u\n"
         "                 height: %u\n"
         "   waveformMode: %u,\n"
         "   updateMode:   %u\n"
         "   updateMarker: %u\n"
         "   temp: %d\n"
         "   flags: 0x%04x\n"
         "   alt_buffer_data: %p\n"
         "}",
         x->update_region.top,
         x->update_region.left,
         x->update_region.width,
         x->update_region.height,
         x->waveform_mode,
         x->update_mode,
         x->update_marker,
         x->temp,
         x->flags,
         x->alt_buffer_data);
}

int ioctl(int fd, int request, ...)
{
  static int (*func)(int fd, int request, ...);

  if (!func) {
    printf("Hooking ioctl(...)\n");
    func = (int (*)(int d, int request, ...)) dlsym(RTLD_NEXT, "ioctl");
  }

  va_list args;

  va_start(args, request);
  void *p = va_arg(args, void *);
  va_end(args);

  if (fd == 3) {
    printf("ioctl(%d, 0x%x, %p", fd, request, p);

    /* partial image update */
    if (request == 0x4048462e)
      print_mxcfb_update_data((mxcfb_update_data*)p);
  }

  int rc = func(fd, request, p);

  if (fd == 3) {
    printf(") == %d\n", rc);
  }
  return rc;
}
