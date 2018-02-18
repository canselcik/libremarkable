#include <stdio.h>
#include <fcntl.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <linux/fb.h>
#include <sys/ioctl.h>

int fb;
unsigned no_row;
unsigned no_columns;
unsigned depth;
unsigned currentrow;
struct fb_var_screeninfo vinfo;

void fb_write_line(unsigned rowpattern)
{
  unsigned i;
  unsigned j;
  unsigned pixel;
  for(i=0; i<vinfo.xres; i++) {
    pixel = (rowpattern & (1<<(i%32))) > 0 ? 0xffffffff : 0;
    for(j=0; j<(vinfo.bits_per_pixel/8); j++) {
      write(fb, &pixel, 1);
    }
  }
  currentrow++;
}

//returns 1 when screen is painted
unsigned fb_write_32(unsigned rowpattern, unsigned columnpattern)
{
  unsigned i;
  for(i=0; i<32; i++) {
    fb_write_line( (columnpattern & (1<<i)) > 0 ? 0xffffffff : rowpattern);
    if (currentrow > vinfo.yres)
      break;
  }
  if(vinfo.yres > 2000)
    return 1;
  return (currentrow > vinfo.yres) ? 1 : 0;
}

#define REMARKABLE_PREFIX                       0x40480000
#define MXCFB_SEND_UPDATE                       0x0000462e

// Untested
#define MXCFB_WAIT_FOR_VSYNC                    0x00004620
#define MXCFB_SET_GBL_ALPHA                     0x00004621
#define MXCFB_SET_CLR_KEY                       0x00004622
#define MXCFB_SET_OVERLAY_POS                   0x00004624
#define MXCFB_GET_FB_IPU_CHAN	                0x00004625
#define MXCFB_SET_LOC_ALPHA	                0x00004626
#define MXCFB_SET_LOC_ALP_BUF	                0x00004627
#define MXCFB_SET_GAMMA	                        0x00004628
#define MXCFB_GET_FB_IPU_DI	                0x00004629
#define MXCFB_GET_DIFMT	                        0x0000462a
#define MXCFB_GET_FB_BLANK	                0x0000462b
#define MXCFB_SET_WAVEFORM_MODES	        0x0000462b
#define MXCFB_SET_DIFMT	                        0x0000462c
#define MXCFB_SET_TEMPERATURE	                0x0000462c
#define MXCFB_SET_AUTO_UPDATE_MODE              0x0000462d
#define MXCFB_WAIT_FOR_UPDATE_COMPLETE	        0x0000462f
#define MXCFB_SET_PWRDOWN_DELAY	                0x00004630
#define MXCFB_GET_PWRDOWN_DELAY	                0x00004631
#define MXCFB_SET_UPDATE_SCHEME                 0x00004632
#define MXCFB_SET_MERGE_ON_WAVEFORM_MISMATCH    0x00004637

typedef struct {
  uint32_t top;    // 0x0000
  uint32_t left;   // 0x0000
  uint32_t width;  // 0x0258 = 600
  uint32_t height; // 0x0320 = 800
} mxcfb_rect;

typedef struct {
  mxcfb_rect update_region;

  uint32_t waveform_mode;  // 0x0002 = WAVEFORM_MODE_GC16
  uint32_t update_mode;    // 0x0000 = UPDATE_MODE_PARTIAL
  uint32_t update_marker;  // 0x002a

  int temp;   // 0x1001 = TEMP_USE_PAPYRUS
  uint flags; // 0x0000

  void* alt_buffer_data; // must not used when flags is 0
} mxcfb_update_data;

int main(void)
{
  char *fb_dev_file = "/dev/fb0";
  fb = open(fb_dev_file, O_RDWR);
  if (fb < 0) {
    printf("Could not open %s, exiting...\n", fb_dev_file);
    return 1;
  }
  currentrow = 0;

  if (ioctl(fb, FBIOGET_VSCREENINFO, &vinfo)) {
    printf("Could not get screen info %s, exiting...\n", fb_dev_file);
    close(fb);
    return 1;
  }
  printf("fb %s is of size x: %u, y: %u, depth %u\n", fb_dev_file, vinfo.xres, vinfo.yres, vinfo.bits_per_pixel);

  mxcfb_update_data data;
  data.update_region.top = 0;
  data.update_region.left = 0;
  data.update_region.width = vinfo.xres;
  data.update_region.height = vinfo.yres;
  data.waveform_mode = 0x0002;
  data.temp = 0x1001;
  data.update_mode = 0x0000;
  data.update_marker = 0x002a;
  data.flags = 0;
  data.alt_buffer_data = NULL;

  while (fb_write_32(0x80018001, 0x80018001) == 0);
  printf("Painted %u lines\n\n", currentrow);

  int res = ioctl(fb, REMARKABLE_PREFIX | MXCFB_SEND_UPDATE, &data);
  printf("ioctl(fb, 0x%08x, &data) = %d\n", REMARKABLE_PREFIX | MXCFB_SEND_UPDATE, res);

  close(fb);
  return 0;
}
