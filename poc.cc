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
  #include "libremarkable/shapes.h"
}

int get_random(int min, int max) {
   return min + rand() / (RAND_MAX / (max - min + 1) + 1);
}

void scanning_line(remarkable_framebuffer* fb, unsigned iter) {
  if (fb == NULL)
    return;
  mxcfb_rect tb = {450,121,1300,10};
  remarkable_framebuffer_draw_rect(fb, tb, REMARKABLE_DARKEST);
  int dir = 1;
  uint32_t refresh_marker = 0;
  for(unsigned i = 0; i < iter; i++) {
    remarkable_framebuffer_draw_rect(fb, tb, REMARKABLE_BRIGHTEST);

    if (tb.top > YRES(fb) - 450 || tb.top < 450)
      dir *= -1;
    tb.top += 5 * dir;
    remarkable_framebuffer_draw_rect(fb, tb, REMARKABLE_DARKEST);
    
    refresh_marker = remarkable_framebuffer_refresh(fb, 
                                                    UPDATE_MODE_PARTIAL,
                                                    WAVEFORM_MODE_DU,
                                                    TEMP_USE_PAPYRUS,
                                                    EPDC_FLAG_USE_DITHERING_FLOYD_STEINBERG,
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
  while (iter--) {
    // Gives 2816px horizontally (res * 2)
    // And   3840px vertically (virtual res accounted for)
    // TODO: Figure out the reason why this does it
    rect.left = get_random(0, to_remarkable_width(XRES(fb)));
    rect.top = get_random(500, YRES(fb) - 500);
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
                                                    WAVEFORM_MODE_GC16_FAST,
                                                    TEMP_USE_PAPYRUS,
                                                    EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                                    0,    // flags
                                                    0,    // quant_bit
                                                    NULL, // alt_buffer_data
                                                    rect.top, rect.left,
                                                    rect.height, rect.width);
    remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);
    usleep(10000);
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
                                 YRES(fb), XRES(fb));

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
                                 YRES(fb), XRES(fb));
  usleep(300 * 1000);
}

void draw_sample_shapes(remarkable_framebuffer* fb) {
  if (fb == NULL)
    return;
  remarkable_color* shapes[] = { rmShape_A, rmShape_B, rmShape_C, rmShape_smiley };
  for (unsigned i = 0; i < 4; i++) {
    remarkable_framebuffer_draw_shape(fb, shapes[i], 8, 8, 320, 1720 + (64 * i), 64, 64);
  }
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
                                 64, 64 * 4); // h: 64, w: 3 * 64 (4 shapes)
}

int main(void) {
  srand(time(NULL));

  remarkable_framebuffer* fb = remarkable_framebuffer_init("/dev/fb0");
  if (fb == NULL) {
    printf("remarkable_framebuffer_init('/dev/fb0') returned NULL. Exiting.\n");
    exit(1);
  }


  clear_display(fb);

  // display_bmp(fb, "/tmp/test.bmp");

  draw_sample_shapes(fb);
  mxcfb_rect updated_rect = remarkable_framebuffer_draw_text(fb,
                                                            "/usr/share/fonts/ttf/noto/NotoSans-Regular.ttf",
                                                            "ReMarkable",
                                                            120, 900, 10);
  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_PARTIAL,
                                 WAVEFORM_MODE_GC16_FAST,
                                 TEMP_USE_MAX,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,          // flags
                                 0,          // quant_bit
                                 NULL,       // alt_buffer
                                 updated_rect.top, updated_rect.left,
                                 updated_rect.height, updated_rect.width);

  while (true) {
    scanning_line(fb, 200);
    random_rects(fb, 1000);
  }
  remarkable_framebuffer_destroy(fb);
  return 0;
}
