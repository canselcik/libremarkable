#include "lib.h"
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/mman.h>

#define LOGBUFSIZE 512

char* serialize_mxcfb_update_data(mxcfb_update_data* x) {
  char* buff = (char*)malloc(LOGBUFSIZE);  
  snprintf(buff, LOGBUFSIZE, 
    "{\n"
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
  return buff;
}

void print_mxcfb_update_data(mxcfb_update_data* x) {
  char* buff = serialize_mxcfb_update_data(x);
  printf("%s", buff);
  free(buff);
}

remarkable_framebuffer* remarkable_framebuffer_init(const char* device_path) {
  remarkable_framebuffer* buff = (remarkable_framebuffer*)malloc(sizeof(remarkable_framebuffer));
  buff->fd_path = device_path;
  buff->fd = open(device_path, O_RDWR);
  if (buff->fd < 0) {
    printf("Could not open %s\n", device_path);
    free(buff);
    return NULL;
  }
  if (ioctl(buff->fd, FBIOGET_VSCREENINFO, &buff->vinfo)) {
    printf("Could not get screen vinfo for %s\n", device_path);
    close(buff->fd);
    free(buff);
    return NULL;
  }
  if (ioctl(buff->fd, FBIOGET_FSCREENINFO, &buff->finfo)) {
    printf("Could not get screen finfo for %s\n", device_path);
    close(buff->fd);
    free(buff);
    return NULL;
  }

  buff->len = buff->vinfo.xres * buff->vinfo.yres * buff->vinfo.bits_per_pixel / 8;
  buff->mapped_buffer = mmap(NULL, buff->len, PROT_READ | PROT_WRITE, MAP_SHARED, buff->fd, 0);
  if (buff->mapped_buffer == NULL) {
    printf("Failed to mmap the framebuffer\n");
    close(buff->fd);
    free(buff);
    return NULL;
  }
  printf("Framebuffer(%d - %s) with [x=%u, y=%u, depth=%u]\n",
    buff->fd,
    buff->fd_path,
    buff->vinfo.xres,
    buff->vinfo.yres,
    buff->vinfo.bits_per_pixel);
  return buff;
}

void remarkable_framebuffer_destroy(remarkable_framebuffer* fb) {
  if (fb == NULL)
    return;
  if (fb->mapped_buffer != NULL)
    munmap(fb->mapped_buffer, fb->len);
  if (fb->fd > 0)
    close(fb->fd);
  free(fb);
}



int remarkable_framebuffer_set_pixel(remarkable_framebuffer* fb, unsigned y, unsigned x, remarkable_color c) {
  if (fb == NULL)
    return 0;
  
  int c1_offset = y * fb->finfo.line_length + x;
  int c2_offset = c1_offset + fb->finfo.line_length;
  if (c2_offset >= fb->len)
    return 0;

  // We take twice as much on the horizontal direction
  *(fb->mapped_buffer + c1_offset) = c;
  *(fb->mapped_buffer + c2_offset) = c;
  return 1;
}

void remarkable_framebuffer_fill(remarkable_framebuffer* fb, remarkable_color color) {
  if (fb == NULL)
    return;
  memset(fb->mapped_buffer, color, fb->len);
}

int gen = 0;
// rect=NULL for full-screen refresh
int remarkable_framebuffer_refresh(remarkable_framebuffer* fb, mxcfb_rect* rect,
                                   update_mode refresh_mode, waveform_mode waveform,
                                   display_temp temp) {
  if (fb == NULL)
    return -1;

  mxcfb_update_data data = {0};
  if (rect == NULL) {
    data.update_region.top = 0;
    data.update_region.left = 0;
    // TODO: Determine if we are using the virtual size here or not
    data.update_region.height = fb->vinfo.yres;
    data.update_region.width = fb->vinfo.xres;
  }
  else {
    data.update_region = *rect;
  }

  data.waveform_mode = waveform;
  data.temp = temp;

  data.update_mode = refresh_mode;
  data.update_marker = gen++;
  
  data.flags = 0;
  
  return ioctl(fb->fd, REMARKABLE_PREFIX(MXCFB_SEND_UPDATE), &data);
}