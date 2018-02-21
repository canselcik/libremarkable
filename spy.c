#define _GNU_SOURCE
#include <string.h>
#include <stdio.h>
#include <stdarg.h>
#include <dlfcn.h>
#include <sys/types.h>
#include <sys/stat.h>
#include "libremarkable/lib.h"

void hexDump (char *desc, void *addr, int len) {
    int i;
    unsigned char buff[17];
    unsigned char *pc = (unsigned char*)addr;

    // Output description if given.
    if (desc != NULL)
        printf ("%s:\n", desc);

    if (len == 0) {
        printf("  ZERO LENGTH\n");
        return;
    }
    if (len < 0) {
        printf("  NEGATIVE LENGTH: %i\n",len);
        return;
    }

    // Process every byte in the data.
    for (i = 0; i < len; i++) {
        // Multiple of 16 means new line (with line offset).

        if ((i % 16) == 0) {
            // Just don't print ASCII for the zeroth line.
            if (i != 0)
                printf ("  %s\n", buff);

            // Output the offset.
            printf ("  %04x ", i);
        }

        // Now the hex code for the specific character.
        printf (" %02x", pc[i]);

        // And store a printable ASCII character for later.
        if ((pc[i] < 0x20) || (pc[i] > 0x7e))
            buff[i % 16] = '.';
        else
            buff[i % 16] = pc[i];
        buff[(i % 16) + 1] = '\0';
    }

    // Pad out last line if not exactly 16 characters.
    while ((i % 16) != 0) {
        printf ("   ");
        i++;
    }

    // And print the final ASCII bit.
    printf ("  %s\n", buff);
}

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
    printf("ioctl(%d, 0x%x (addr: %p), %p (addr: %p)", fd, request, &request, p, &p);

    struct fb_var_screeninfo* vinfo;
    switch (request) {
      case REMARKABLE_PREFIX(MXCFB_SEND_UPDATE):
        print_mxcfb_update_data((mxcfb_update_data*)p);
        hexDump("mxcfb_update_data", p, sizeof(mxcfb_update_data));
        break;
      case FBIOPUT_VSCREENINFO:
        printf("(FBIOPUT_VSCREENINFO)\n");
        print_vinfo((struct fb_var_screeninfo*)p);
        break;
      case FBIOGET_VSCREENINFO:
        printf("(FBIOGET_VSCREENINFO)\n");
        print_vinfo((struct fb_var_screeninfo*)p);
        break;
      case REMARKABLE_PREFIX(MXCFB_WAIT_FOR_UPDATE_COMPLETE):
        hexDump("MXCFB_WAIT_FOR_UPDATE_COMPLETE(mxcfb_update_marker_data)", p, sizeof(mxcfb_update_marker_data));
      default:
        printf(" (unknown)");
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
