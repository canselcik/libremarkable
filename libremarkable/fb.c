#include "lib.h"
#include <sys/ioctl.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/mman.h>

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

  // Using fb_videomode es103cs1_mode as a reference (ES103CS1)
  buff->vinfo.accel_flags = 0x01;
  buff->vinfo.width = buff->vinfo.xres;
  buff->vinfo.height  = buff->vinfo.yres;
  buff->vinfo.rotate = 1;
  buff->vinfo.pixclock = 160000000;
  buff->vinfo.xres = 1872;
  buff->vinfo.yres = 1404;
  buff->vinfo.left_margin = 32;
  buff->vinfo.right_margin = 326;
  buff->vinfo.upper_margin = 4;
  buff->vinfo.lower_margin = 12;
  buff->vinfo.hsync_len = 44;
  buff->vinfo.vsync_len = 1;
  buff->vinfo.sync = 0;
  buff->vinfo.vmode = FB_VMODE_NONINTERLACED;
  buff->vinfo.accel_flags = 0;

  // Let's set it to our liking and see what happens
  if (ioctl(buff->fd, FBIOPUT_VSCREENINFO, &buff->vinfo)) {
    printf("Failed to set the var screen info for %s\n", device_path);
    close(buff->fd);
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

  buff->len = buff->vinfo.yres_virtual * buff->finfo.line_length;
  buff->mapped_buffer = mmap(0, buff->len, PROT_READ | PROT_WRITE, MAP_SHARED, buff->fd, (off_t)0);
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

int remarkable_framebuffer_set_epdc_access(remarkable_framebuffer* fb, int enabled) {
  if (fb == NULL)
    return 1;
  return ioctl(fb->fd, REMARKABLE_PREFIX(enabled == 0 ? MXCFB_DISABLE_EPDC_ACCESS : MXCFB_ENABLE_EPDC_ACCESS));
}

int remarkable_framebuffer_set_auto_update_mode(remarkable_framebuffer* fb, auto_update_mode mode) {
  if (fb == NULL)
    return 1;
  return ioctl(fb->fd, REMARKABLE_PREFIX(MXCFB_SET_AUTO_UPDATE_MODE), (uint32_t*)&mode);
}

int remarkable_framebuffer_set_auto_update_period(remarkable_framebuffer* fb, int period) {
  if (fb == NULL)
    return 1;
  return ioctl(fb->fd, REMARKABLE_PREFIX(MXCFB_SET_TEMP_AUTO_UPDATE_PERIOD), &period);
}

int remarkable_framebuffer_set_update_scheme(remarkable_framebuffer* fb, update_scheme scheme) {
  if (fb == NULL)
    return 1;
  uint32_t val = 0;
  return ioctl(fb->fd, REMARKABLE_PREFIX(MXCFB_SET_UPDATE_SCHEME), &val);
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

int remarkable_framebuffer_set_pixel(remarkable_framebuffer* fb, const unsigned y, const unsigned x, const remarkable_color c) {
  if (fb == NULL)
    return 1;
  
  int c1_offset = y * fb->finfo.line_length + x;
  if (c1_offset >= fb->len)
    return 1;

  *(fb->mapped_buffer + c1_offset) = c;
  return 0;
}

void remarkable_framebuffer_draw_shape(remarkable_framebuffer* fb, remarkable_color* shape, unsigned rows,   unsigned cols,
                                                                                            unsigned y,      unsigned x,
                                                                                            unsigned height, unsigned width) {
  if (fb == NULL)
    return;
  float boxWidth = width / (float)cols;
  float boxHeight = height / (float)rows;
  for (unsigned iY = 0; iY < rows; iY++) {
    for (unsigned iX = 0; iX < cols; iX++) {
      remarkable_color color = shape[cols * iY + iX];

      // top, left, width, height
      mxcfb_rect tb = {0};
      tb.top = y + iY * boxHeight;
      tb.left = x + iX * boxWidth;
      tb.width = boxWidth;
      tb.height = boxHeight;
      remarkable_framebuffer_draw_rect(fb, tb, color);
    }
  }
}

void remarkable_framebuffer_draw_rect(remarkable_framebuffer* fb, mxcfb_rect rect, remarkable_color color) {
  if (fb == NULL)
    return;

  if (rect.height == 0 && rect.width == 0)
    return;

  // TODO: Figure out the reason why this does it
  rect.width = to_remarkable_width(rect.width);

  int offset = 0;
  for (unsigned y = rect.top; y < rect.height + rect.top; ++y) {
    for (unsigned x = rect.left; x < rect.width + rect.left; ++x) {
      remarkable_framebuffer_set_pixel(fb, y, x, color);
    }
  }
}


void remarkable_framebuffer_fill(remarkable_framebuffer* fb, remarkable_color color) {
  if (fb == NULL)
    return;
  memset(fb->mapped_buffer, color, fb->len);
}
