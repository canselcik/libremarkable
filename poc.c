#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdlib.h>
#include <linux/fb.h>
#include <sys/ioctl.h>
#include "libremarkable/lib.h"

void draw_hlines(remarkable_framebuffer* fb, int start_dark, int line_height) {
  if (fb == NULL)
    return;

  uint16_t val = start_dark == 0 ? REMARKABLE_DARKEST : REMARKABLE_BRIGHTEST;
  for (unsigned row = 0; row < fb->info.xres; ++row) {
    if (row % line_height == 0) {
      val = (val == REMARKABLE_BRIGHTEST) ? REMARKABLE_DARKEST 
                                          : REMARKABLE_BRIGHTEST;
    }
    memset(fb->mapped_buffer + (fb->info.yres * row), val, fb->info.yres);
  }
}

void draw_rect(remarkable_framebuffer* fb, mxcfb_rect rect) {
  if (fb == NULL)
    return;
  // TODO: Not implemented
}

int main(void) {
  remarkable_framebuffer* fb = remarkable_framebuffer_init("/dev/fb0");
  if (fb == NULL) {
    printf("remarkable_framebuffer_init('/dev/fb0') returned NULL. Exiting.\n");
    exit(1);
  }

  for (unsigned i = 0; i < 10; i++) {
    draw_hlines(fb, i % 2, 10);
    remarkable_framebuffer_refresh(fb);
    sleep(1.5);
  }

  remarkable_framebuffer_fill(fb, REMARKABLE_BRIGHTEST);
  remarkable_framebuffer_refresh(fb);

  remarkable_framebuffer_destroy(fb);
  return 0;
}
