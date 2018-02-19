#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h> 
#include <stdlib.h>
#include <linux/fb.h>
#include <sys/ioctl.h>
#include "libremarkable/lib.h"

int get_random(int min, int max) {
   return min + rand() / (RAND_MAX / (max - min + 1) + 1);
}

void draw_rect(remarkable_framebuffer* fb, mxcfb_rect rect, remarkable_color color) {
  if (fb == NULL)
    return;

  int offset = 0;
  for (unsigned y = rect.top; y < rect.height + rect.top; ++y) {
    for (unsigned x = rect.left; x < rect.width + rect.left; ++x) {
      remarkable_framebuffer_set_pixel(fb, y, x, color);
    }
  }
}

int main(void) {
  remarkable_framebuffer* fb = remarkable_framebuffer_init("/dev/fb0");
  if (fb == NULL) {
    printf("remarkable_framebuffer_init('/dev/fb0') returned NULL. Exiting.\n");
    exit(1);
  }

  // Clear the screen and do a full refresh
  remarkable_framebuffer_fill(fb, REMARKABLE_BRIGHTEST);
  remarkable_framebuffer_refresh(fb, 
                                 NULL, 
                                 UPDATE_MODE_FULL,
                                 WAVEFORM_MODE_INIT,
                                 TEMP_USE_PAPYRUS);

  sleep(1);

  srand(time(NULL));

  // Draw a rectangle and only update that region
  mxcfb_rect rect;
  for (unsigned i = 0; i < 100; i++) {
    // Gives 2816px horizontally (res * 2)
    // And   3840px vertically (virtual res accounted for)
    rect.top = get_random(0, fb->vinfo.yres_virtual);
    rect.left = get_random(0, fb->vinfo.xres_virtual * 2);
    rect.height = 50;
    rect.width = 100;
    draw_rect(fb, rect, REMARKABLE_DARKEST);

    usleep(200000);

    // Partial/Quick refresh on the entire screen
    remarkable_framebuffer_refresh(fb, 
                                   NULL,
                                   UPDATE_MODE_PARTIAL,
                                   WAVEFORM_MODE_GLR16,
                                   TEMP_USE_MAX);
  }


  remarkable_framebuffer_destroy(fb);
  return 0;
}
