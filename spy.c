#define _GNU_SOURCE
#include <string.h>
#include <stdio.h>
#include <stdarg.h>
#include <dlfcn.h>
#include <sys/types.h>
#include <sys/stat.h>
#include "libremarkable/lib.h"

void breakhere(struct fb_var_screeninfo* vinfo) {
  return;
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
        vinfo = (struct fb_var_screeninfo*)p;  
        breakhere(vinfo);
        printf(" (SETTING) fb_var_screeninfo {\n"
          "  xres: %d\n"
          "  yres: %d\n"
          "  xres_virtual: %d\n"
          "  yres_virtual: %d ,... }", vinfo->xres, vinfo->yres, vinfo->xres_virtual, vinfo->yres_virtual);
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
  return rc;
}
