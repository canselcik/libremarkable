#include "lib.h"
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/mman.h>

#define BUFSIZE 512

char* serialize_mxcfb_update_data(mxcfb_update_data* x) {
  char* buff = (char*)malloc(BUFSIZE);  
  snprintf(buff, BUFSIZE, 
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
  if (ioctl(buff->fd, FBIOGET_VSCREENINFO, &buff->info)) {
    printf("Could not get screen info for %s\n", device_path);
    close(buff->fd);
    free(buff);
    return NULL;
  }

  buff->len = buff->info.xres * buff->info.yres * buff->info.bits_per_pixel / 8;
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
    buff->info.xres,
    buff->info.yres,
    buff->info.bits_per_pixel);
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

int remarkable_framebuffer_partial_refresh(remarkable_framebuffer* fb, unsigned y, unsigned x, unsigned height, unsigned width) {
  if (fb == NULL)
    return -1;
  mxcfb_update_data data;
  data.update_region.top = y;
  data.update_region.left = x;
  data.update_region.width = width;
  data.update_region.height = height;
  data.waveform_mode = 0x0003; // or 2 
  data.temp = 0x0FFF;
  data.update_mode = 0x0000; // how thorough it is?
  data.update_marker = 0x0000; // kinda like a sequence num
  data.flags = 0;
  data.alt_buffer_data = NULL;
  return ioctl(fb->fd, REMARKABLE_PREFIX | MXCFB_SEND_UPDATE, &data);
}

int remarkable_framebuffer_refresh(remarkable_framebuffer* fb) {
  if (fb == NULL)
    return -1;
  return remarkable_framebuffer_partial_refresh(fb, 0, 0, fb->info.yres, fb->info.xres);
}

void remarkable_framebuffer_fill(remarkable_framebuffer* fb, uint16_t color) {
  if (fb == NULL)
    return;
  memset(fb->mapped_buffer, color, fb->len);
}