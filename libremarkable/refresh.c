#include "lib.h"
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/mman.h>

#define min(a,b) (((a)<(b))?(a):(b))
#define max(a,b) (((a)>(b))?(a):(b))

// 0 is an invalid update_marker value
int gen = 1;

// rect=NULL for full-screen refresh
uint32_t remarkable_framebuffer_refresh(remarkable_framebuffer* fb,
                                        update_mode refresh_mode, waveform_mode waveform,
                                        display_temp temp, unsigned y, unsigned x,
                                        unsigned height, unsigned width) {
  if (fb == NULL)
    return -1;

  // TODO: Figure out the reason why this does it
  x = x / 2;
  width = to_remarkable_width(width);

  mxcfb_update_data data = {0};
  data.update_region.top = max(min(y, fb->vinfo.yres - 1), 0);
  data.update_region.left = max(min(x, fb->vinfo.xres - 1), 0);

  if (data.update_region.left + width >= fb->vinfo.xres)
    data.update_region.width = fb->vinfo.xres - data.update_region.left;
  else 
    data.update_region.width = width;

  if (data.update_region.top + height >= fb->vinfo.yres)
    data.update_region.height = fb->vinfo.yres - data.update_region.top;
  else 
    data.update_region.height = height;


  data.waveform_mode = waveform;
  data.temp = temp;

  data.update_mode = refresh_mode;
  data.update_marker = gen++;
  
  data.flags = 0;
  
  int res = ioctl(fb->fd, REMARKABLE_PREFIX(MXCFB_SEND_UPDATE), &data);
  if (res != 0) {
    printf("ioctl(.., MXCFB_SEND_UPDATE) = %d\n", res);
    return -1;
  }
  
  // Return the marker so that the caller can wait for it to finish drawing if needed
  return data.update_marker;
}

int remarkable_framebuffer_wait_refresh_marker(remarkable_framebuffer* fb, uint32_t marker) {
  if (fb == NULL)
    return -1;

  // TODO: Collusion test (2nd value) is an output param here. It's value might be useful.
	mxcfb_update_marker_data mdata = { marker, 0 };
  return ioctl(fb->fd, REMARKABLE_PREFIX(MXCFB_WAIT_FOR_UPDATE_COMPLETE), &mdata);
}