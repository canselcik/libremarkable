#define _GNU_SOURCE
#include <string.h>
#include <stdio.h>
#include <stdarg.h>
#include <dlfcn.h>
#include <sys/types.h>
#include <sys/stat.h>
#include "libremarkable/lib.h"

#define printcolor(color)	\
  printf (#color"\t: offset = %u,\tlength =%u,\tmsb_right = %u\n", \
    v->color.offset, v->color.length, v->color.msb_right)

void print_vinfo(struct fb_var_screeninfo* v) {
  if (v == NULL) {
    printf("vinfo is NULL");
    return;
  }
  printf("xres\t\t= %u\tyres\t\t= %u\n", v->xres, v->yres);
  printf("xres_virtual\t= %u\tyres_virtual\t= %u\n", v->xres_virtual, v->yres_virtual);
  printf("xoffset\t\t= %u\tyoffset\t\t= %u\n", v->xoffset, v->yoffset);
  printf("bits_per_pixel\t= %u\tgrayscale\t= %u\n", v->bits_per_pixel, v->grayscale);
  printcolor(red);
  printcolor(green);
  printcolor(blue);
  printcolor(transp);
  printf("nonstd\t\t= %u\n", v->nonstd);
  printf("activate\t= %u\n", v->activate);
  printf("height\t\t= 0x%x\nwidth\t\t= 0x%x\n", v->height, v->width);
  printf("accel_flags(OBSOLETE) = %u\n", v->accel_flags);
  printf("pixclock	= %u\n", v->pixclock);
  printf("left_margin	= %u\n", v->left_margin);
  printf("right_margin	= %u\n", v->right_margin);
  printf("upper_margin	= %u\n", v->upper_margin);
  printf("lower_margin	= %u\n", v->lower_margin);
  printf("hsync_len	= %u\nvsync_len       = %u\n", v->hsync_len, v->vsync_len);
  printf("sync		= %u\n", v->sync);
  printf("vmode		= %u\n", v->vmode);
  printf("rotate		= %u\n", v->rotate);
  printf("colorspace 	= %u\n", v->colorspace);
}

int ioctl(int fd, int request, ...) {
  static int (*func)(int fd, int request, ...);
  if (!func) {
    printf("Hooking ioctl(...)\n");
    func = (int (*)(int d, int request, ...)) dlsym(RTLD_NEXT, "ioctl");
  }

  va_list args;
  va_start(args, request);
  void *p = va_arg(args, void *);
  va_end(args);

  if (fd == 3) {
    printf("ioctl(%d, 0x%x, %p", fd, request, p);

    struct fb_var_screeninfo* vinfo;
    switch (request) {
      case REMARKABLE_PREFIX | MXCFB_SEND_UPDATE:
        print_mxcfb_update_data((mxcfb_update_data*)p);
        break;
      case FBIOPUT_VSCREENINFO:
        printf("===== SETTING VSCREEN INFO =====\n");
        print_vinfo((struct fb_var_screeninfo*)p);
        break;
      case FBIOGET_VSCREENINFO:
        break;
      default:
        printf(" (UNCLASSIFIED)");
        break;
    }
  }

  int rc = func(fd, request, p);
  if (fd == 3) {
    printf(") == %d\n", rc);
  }

  if (request == FBIOGET_VSCREENINFO) {
    printf("===== GETTING VSCREEN INFO =====\n");
    print_vinfo((struct fb_var_screeninfo*)p);
  }
  return rc;
}
