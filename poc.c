#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h> 
#include <stdlib.h>
#include <linux/fb.h>
#include <sys/ioctl.h>
#include "libremarkable/lib.h"
#include "libremarkable/bitmap.h"

int get_random(int min, int max) {
   return min + rand() / (RAND_MAX / (max - min + 1) + 1);
}

void draw_rect(remarkable_framebuffer* fb, mxcfb_rect rect, remarkable_color color) {
  if (fb == NULL)
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

void scanning_line(remarkable_framebuffer* fb, unsigned iter) {
  if (fb == NULL)
    return;
  mxcfb_rect tb = {20,120,1300,10};
  draw_rect(fb, tb, REMARKABLE_DARKEST);
  int dir = 1;
  uint32_t refresh_marker = 0;
  for(unsigned i = 0; i < iter; i++) {
    draw_rect(fb, tb, REMARKABLE_BRIGHTEST);

    if (tb.top > fb->vinfo.yres || tb.top < 0)
      dir *= -1;
    tb.top += 10 * dir;
    draw_rect(fb, tb, REMARKABLE_DARKEST);
    
    refresh_marker = remarkable_framebuffer_refresh(fb,
                                                    UPDATE_MODE_PARTIAL,
                                                    WAVEFORM_MODE_REAGLD,
                                                    TEMP_USE_PAPYRUS, tb.top-20, tb.left,
                                                    tb.height+40, tb.width);
    remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);

    usleep(100000);
  }
}

void random_rects(remarkable_framebuffer* fb, unsigned iter) {
  if (fb == NULL)
    return;

  // Draw a rectangle and only update that region
  mxcfb_rect rect;
  uint32_t refresh_marker = 0;
  for (unsigned i = 0; i < iter; i++) {
    // Gives 2816px horizontally (res * 2)
    // And   3840px vertically (virtual res accounted for)
    // TODO: Figure out the reason why this does it
    rect.left = get_random(0, to_remarkable_width(fb->vinfo.xres));
    rect.top = get_random(0, fb->vinfo.yres);
    rect.height = 50;
    rect.width = 50;
    draw_rect(fb, rect, REMARKABLE_DARKEST);

    // Partial/Quick refresh on the entire screen
    refresh_marker = remarkable_framebuffer_refresh(fb, 
                                                    UPDATE_MODE_PARTIAL,
                                                    WAVEFORM_MODE_GLR16,
                                                    TEMP_USE_PAPYRUS,
                                                    rect.top,
                                                    rect.left,
                                                    rect.height,
                                                    rect.width);
    remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);
    usleep(90 * 1000);
  }
    
}

void display_bmp(remarkable_framebuffer* fb, char* path) {
  bmp_img bitmap = { 0 };
  bmp_img_read(&bitmap, path);

  printf("Bitmap loaded [size=%u, width=%u, height=%u]\n", bitmap.img_header.bfSize, bitmap.img_header.biWidth, bitmap.img_header.biHeight);
  unsigned left = 2000;
  unsigned top = 200;
  for (unsigned y = 0; y < bitmap.img_header.biHeight; y++) {
    for (unsigned x = 0; x < bitmap.img_header.biWidth; x++) {
        // TODO: Color interp
        unsigned char r = bitmap.img_pixels[y][x].red;
        unsigned char g = bitmap.img_pixels[y][x].green;
        unsigned char b = bitmap.img_pixels[y][x].blue;
        remarkable_framebuffer_set_pixel(fb, top + y, left + x, r > 200 ? REMARKABLE_DARKEST : REMARKABLE_BRIGHTEST);
    }
  }
  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_FULL,
                                 WAVEFORM_MODE_INIT,
                                 TEMP_USE_MAX, 0, 0,
                                 fb->vinfo.yres, fb->vinfo.xres);
}

void clear_display(remarkable_framebuffer* fb) {
  if (fb == NULL)
    return;
  remarkable_framebuffer_fill(fb, REMARKABLE_BRIGHTEST);
  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_FULL,
                                 WAVEFORM_MODE_INIT,
                                 TEMP_USE_MAX, 0, 0,
                                 fb->vinfo.yres, fb->vinfo.xres);
}

int main(void) {
  srand(time(NULL));

  remarkable_framebuffer* fb = remarkable_framebuffer_init("/dev/fb0");
  if (fb == NULL) {
    printf("remarkable_framebuffer_init('/dev/fb0') returned NULL. Exiting.\n");
    exit(1);
  }

  clear_display(fb); 

  // scanning_line(fb, 50000);
  // display_bmp(fb, "/tmp/sample.bmp");
  random_rects(fb, 5000);

  usleep(10000);

  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_FULL,
                                 WAVEFORM_MODE_GLR16,
                                 TEMP_USE_MAX, 0, 0,
                                 fb->vinfo.yres, fb->vinfo.xres);

  remarkable_framebuffer_destroy(fb);
  return 0;
}
