#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h> 
#include <stdlib.h>
#include <linux/fb.h>
#include <sys/ioctl.h>
#include <queue>
#include <list>
#include <linux/input.h>


#include <sys/types.h>
#include <sys/ipc.h>
#include <sys/sem.h>
#include <sched.h>

#define IDKEY 23003
#define SSZ 16384

struct thrinit {
    int sid;
    int tid;
    int *data;
};


extern "C" {
  #include "libremarkable/lib.h"
  #include "libremarkable/bitmap.h"
  #include "libremarkable/shapes.h"
}

#define BITS_PER_LONG (sizeof(long) * 8)
#define NBITS(x) ((((x)-1)/BITS_PER_LONG)+1)
#define OFF(x)  ((x)%BITS_PER_LONG)
#define BIT(x)  (1UL<<OFF(x))
#define LONG(x) ((x)/BITS_PER_LONG)
#define test_bit(bit, array)	((array[LONG(bit)] >> OFF(bit)) & 1)


void evdraw(remarkable_framebuffer* fb, const char* evDevicePath, remarkable_font* font) {
  if (fb == NULL || evDevicePath == NULL || font == NULL)
    return;

  int fd = open(evDevicePath, O_RDONLY);
  if (fd < 0) {
    printf("Unable to open %s\n", evDevicePath);
  	return;
  }

  int version;
  if (ioctl(fd, EVIOCGVERSION, &version)) {
    perror("evtest: can't get version");
    return;
  }

  printf("Input driver version is %d.%d.%d\n", version >> 16,
      (version >> 8) & 0xff, version & 0xff);


  unsigned short id[4];

  ioctl(fd, EVIOCGID, id);
  printf("Input device ID: bus 0x%x vendor 0x%x product 0x%x version 0x%x\n",
      id[ID_BUS], id[ID_VENDOR], id[ID_PRODUCT], id[ID_VERSION]);

  char name[256] = "Unknown";
  ioctl(fd, EVIOCGNAME(sizeof(name)), name);
  printf("Input device name: \"%s\"\n", name);

  int rd = 0;
  struct input_event ev[64];
  mxcfb_rect rect = {0};
  while (1) {
    rd = read(fd, ev, sizeof(struct input_event) * 64);
    if (rd < (int) sizeof(struct input_event)) {
      printf("evtest: error reading");
      return;
    }

    int x, y;
    for (unsigned i = 0; i < rd / sizeof(struct input_event); i++) {
      struct input_event& curr = ev[i];
      if (curr.type == EV_ABS) {
        if (curr.code == 0x00) {
          x = curr.value;
        }
        else if (curr.code == 0x01) {
          y = curr.value;
          char text[255] = {0};
          snprintf(text, 255, "Wacom Input:  y: %d   |   x: %d", y, x);

          // Clear the output area
          remarkable_framebuffer_draw_rect(fb, rect, REMARKABLE_BRIGHTEST);

          // Draw the text and refresh
          rect = remarkable_framebuffer_draw_text(fb, font, text, 1750, 1100);
          uint32_t refresh_marker = remarkable_framebuffer_refresh(fb,
                                                                   UPDATE_MODE_PARTIAL,
                                                                   WAVEFORM_MODE_GC16_FAST,
                                                                   TEMP_USE_PAPYRUS,
                                                                   EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                                                   0, 0, NULL,
                                                                   rect.top, rect.left, rect.height, rect.width);
          remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);
          break;
        }
      }
    }
    usleep(1000 * 100);
  }

}

int get_random(int min, int max) {
   return min + rand() / (RAND_MAX / (max - min + 1) + 1);
}

void scanning_line(remarkable_framebuffer* fb, unsigned iter) {
  if (fb == NULL)
    return;
  mxcfb_rect tb = {450,0,XRES(fb),10};
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

    while (q.size() > 1) {
      mxcfb_rect &removal = q.front();
      remarkable_framebuffer_draw_rect(fb, removal, REMARKABLE_BRIGHTEST);
      refresh_marker = remarkable_framebuffer_refresh(fb,
                                     UPDATE_MODE_PARTIAL,
                                     WAVEFORM_MODE_DU,
                                     TEMP_USE_PAPYRUS,
                                     EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                     0,    // flags
                                     0,    // quant_bit
                                     NULL, // alt_buffer_data
                                     removal.top, removal.left,
                                     removal.height, removal.width);
      q.pop();
    }

    remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);

    // Partial refresh on the portion of the screen that contains the new rectangle
    refresh_marker = remarkable_framebuffer_refresh(fb, 
                                                    UPDATE_MODE_PARTIAL,
                                                    WAVEFORM_MODE_DU,
                                                    TEMP_USE_PAPYRUS,
                                                    EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                                    0,    // flags
                                                    0,    // quant_bit
                                                    NULL, // alt_buffer_data
                                                    rect.top, rect.left,
                                                    rect.height, rect.width);
    remarkable_framebuffer_wait_refresh_marker(fb, refresh_marker);
    usleep(1000 * 100);
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
                                 WAVEFORM_MODE_DU,
                                 TEMP_USE_MAX,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,    // flags
                                 0,    // quant_bit
                                 NULL, /* alt_buffer_data (not very useful -- not even here as the phys_addr
                                                                             needs to be within finfo->smem) */
                                 320, 1720,   // y, x
                                 64, 64 * 4); // h: 64, w: 3 * 64 (4 shapes)
}

int demo(void* thrdata) {
  struct thrinit* initData = (struct thrinit*)thrdata;
  remarkable_framebuffer* fb = (remarkable_framebuffer*)initData->data;
  while (true) {
    random_rects((remarkable_framebuffer*)fb, 30);
    scanning_line((remarkable_framebuffer*)fb, 395);
  }
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

  struct remarkable_font* largeFont  = remarkable_framebuffer_font_init(fb, "/usr/share/fonts/ttf/noto/NotoSans-Regular.ttf", 640);
  struct remarkable_font* mediumFont = remarkable_framebuffer_font_init(fb, "/usr/share/fonts/ttf/noto/NotoSansUI-Regular.ttf", 180);

  mxcfb_rect updated_rect = remarkable_framebuffer_draw_text(fb, largeFont, "ReMarkable", 120, 900);
  remarkable_framebuffer_refresh(fb, 
                                 UPDATE_MODE_PARTIAL,
                                 WAVEFORM_MODE_GC16_FAST,
                                 TEMP_USE_REMARKABLE_DRAW,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,          // flags
                                 0,          // quant_bit
                                 NULL,       // alt_buffer
                                 updated_rect.top, updated_rect.left,
                                 updated_rect.height, updated_rect.width);

  updated_rect = remarkable_framebuffer_draw_text(fb, mediumFont, "The quick brown fox jumps over the lazy dog", 1550, 900);
  remarkable_framebuffer_refresh(fb,
                                 UPDATE_MODE_PARTIAL,
                                 WAVEFORM_MODE_GC16_FAST,
                                 TEMP_USE_REMARKABLE_DRAW,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,          // flags
                                 0,          // quant_bit
                                 NULL,       // alt_buffer
                                 updated_rect.top, updated_rect.left,
                                 updated_rect.height, updated_rect.width);

  // Kick off the dynamic demo -- avoiding using pthreads because of a GLIBC version mismatch
  struct thrinit demoThread = {1, 0, (int*)fb};
  unsigned char* demoThreadStack = (unsigned char*)malloc(SSZ);
  clone(demo, demoThreadStack + SSZ - 1, CLONE_VM | CLONE_SYSVSEM, &demoThread);

  // Read from Wacom
  evdraw(fb, "/dev/input/event0", mediumFont);

  remarkable_framebuffer_refresh(fb,
                                 UPDATE_MODE_PARTIAL,
                                 WAVEFORM_MODE_DU,
                                 TEMP_USE_MAX,
                                 EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                                 0,    // flags
                                 0,    // quant_bit
                                 NULL, // alt_buffer_data
                                 0, 0,  // y, x
                                 YRES(fb), XRES(fb));
  remarkable_framebuffer_destroy(fb);
  return 0;
}
