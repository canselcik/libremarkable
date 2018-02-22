#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h> 
#include <stdlib.h>
#include <linux/fb.h>
#include <sys/ioctl.h>
#include <queue>

extern "C" {
  #include "libremarkable/lib.h"
  #include "libremarkable/bitmap.h"
  #include "libremarkable/chars.h"
}

int get_random(int min, int max) {
   return min + rand() / (RAND_MAX / (max - min + 1) + 1);
}

void scanning_line(remarkable_framebuffer* fb, unsigned iter) {
  if (fb == NULL)
    return;
  mxcfb_rect tb = {20,120,1300,10};
  remarkable_framebuffer_draw_rect(fb, tb, REMARKABLE_DARKEST);
  int dir = 1;
  uint32_t refresh_marker = 0;
  for(unsigned i = 0; i < iter; i++) {
    remarkable_framebuffer_draw_rect(fb, tb, REMARKABLE_BRIGHTEST);

    if (tb.top > fb->vinfo.yres || tb.top < 0)
      dir *= -1;
    tb.top += 5 * dir;
    remarkable_framebuffer_draw_rect(fb, tb, REMARKABLE_DARKEST);
    
    refresh_marker = remarkable_framebuffer_refresh(fb, 
                                                    UPDATE_MODE_PARTIAL,
                                                    WAVEFORM_MODE_GC16_FAST,
                                                    TEMP_USE_PAPYRUS,
                                                    EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                                    0,    // flags
                                                    0,    // quant_bit
                                                    NULL, // alt_buffer_data
                                                    tb.top - 10, tb.left,
                                                    tb.height + 20, tb.width);
    remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);

    usleep(35000);
  }
}

void random_rects(remarkable_framebuffer* fb, unsigned iter) {
  if (fb == NULL)
    return;

  std::queue<mxcfb_rect> q;
  mxcfb_rect rect;
  uint32_t refresh_marker = 0;
  // for (unsigned i = 0; i < iter; i++) {
  while(true) {
    // Gives 2816px horizontally (res * 2)
    // And   3840px vertically (virtual res accounted for)
    // TODO: Figure out the reason why this does it
    rect.left = get_random(0, to_remarkable_width(fb->vinfo.xres));
    rect.top = get_random(0, fb->vinfo.yres);
    rect.height = 50;
    rect.width = 50;
    remarkable_framebuffer_draw_rect(fb, rect, REMARKABLE_DARKEST);
    q.push(rect);

    while (q.size() > 50) {
      remarkable_framebuffer_draw_rect(fb, q.front(), REMARKABLE_BRIGHTEST);
      q.pop();
    }

    // Partial refresh on the portion of the screen that contains the new rectangle
    refresh_marker = remarkable_framebuffer_refresh(fb, 
                                                    UPDATE_MODE_PARTIAL,
                                                    WAVEFORM_MODE_DU,
                                                    TEMP_USE_PAPYRUS,
                                                    EPDC_FLAG_USE_DITHERING_ATKINSON,
                                                    EPDC_FLAG_USE_DITHERING_Y4 | EPDC_FLAG_USE_REGAL | EPDC_FLAG_GROUP_UPDATE, // flags
                                                    0,    // quant_bit
                                                    NULL, // alt_buffer_data
                                                    rect.top, rect.left,
                                                    rect.height, rect.width);
    remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);
  }
    
}

void display_bmp(remarkable_framebuffer* fb, const char* path) {
  bmp_img bitmap = { 0 };
  bmp_img_read(&bitmap, path);

  printf("Bitmap loaded [size=%u, width=%u, height=%u]\n", bitmap.img_header.bfSize, bitmap.img_header.biWidth, bitmap.img_header.biHeight);
  unsigned left = 2000;
  unsigned top = 200;
  for (unsigned y = 0; y < bitmap.img_header.biHeight; y++) {
    for (unsigned x = 0; x < bitmap.img_header.biWidth; x++) {
        unsigned char r = bitmap.img_pixels[y][x].red;
        unsigned char g = bitmap.img_pixels[y][x].green;
        unsigned char b = bitmap.img_pixels[y][x].blue;
        remarkable_framebuffer_set_pixel(fb, top + y, left + x, TO_REMARKABLE_COLOR(r, g, b));
    }
  }
  remarkable_framebuffer_refresh(fb,
                                 UPDATE_MODE_FULL,
                                 WAVEFORM_MODE_GC16_FAST,
                                 TEMP_USE_MAX,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,    // flags
                                 0,    // quant_bit
                                 NULL, /* alt_buffer_data (not very useful -- not even here as the phys_addr
                                                                              needs to be within finfo->smem) */
                                 0, 0,  // y, x
                                 fb->vinfo.yres, fb->vinfo.xres);
}

void clear_display(remarkable_framebuffer* fb) {
  if (fb == NULL)
    return;
  remarkable_framebuffer_fill(fb, REMARKABLE_BRIGHTEST);
  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_FULL,
                                 WAVEFORM_MODE_INIT,
                                 TEMP_USE_MAX,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,    // flags
                                 0,    // quant_bit
                                 NULL, // alt_buffer_data
                                 0, 0,  // y, x
                                 fb->vinfo.yres, fb->vinfo.xres);
}


int main(void) {
  srand(time(NULL));

  remarkable_framebuffer* fb = remarkable_framebuffer_init("/dev/fb0");
  if (fb == NULL) {
    printf("remarkable_framebuffer_init('/dev/fb0') returned NULL. Exiting.\n");
    exit(1);
  }

  // scanning_line(fb, 500);

  clear_display(fb);
  sleep(1);

  // display_bmp(fb, "/tmp/test.bmp");
  // random_rects(fb, 5000);


  remarkable_framebuffer_draw_shape(fb, rmChar_A, 8, 8, 50, 50, 64, 64);
  remarkable_framebuffer_draw_shape(fb, rmChar_B, 8, 8, 50, 50+64, 64, 64);
  remarkable_framebuffer_draw_shape(fb, rmChar_C, 8, 8, 50, 50+128, 64, 64);
  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_PARTIAL,
                                 WAVEFORM_MODE_GC16_FAST,
                                 TEMP_USE_MAX,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,    // flags
                                 0,    // quant_bit
                                 NULL, /* alt_buffer_data (not very useful -- not even here as the phys_addr
                                                                             needs to be within finfo->smem) */
                                 50, 50,  // y, x
                                 64, 64 * 3); // h: 64, w: 3 * 64 (3 letters)
  sleep(10);

  // Full refresh before exit just to get the whole fb content drawn
  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_FULL,
                                 WAVEFORM_MODE_GC16_FAST,
                                 TEMP_USE_MAX,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,    // flags
                                 0,    // quant_bit
                                 NULL, // alt_buffer_data
                                 0, 0,  // y, x
                                 fb->vinfo.yres, fb->vinfo.xres);
  remarkable_framebuffer_destroy(fb);
  return 0;
}
