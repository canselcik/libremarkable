/*
 * Copyright (C) 2010-2012 Freescale Semiconductor, Inc. All Rights Reserved.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307 USA
 *
 */

/*
 * @file mxc_epdc_fb_test.c
 *
 * @brief Mxc EPDC framebuffer driver unit test application
 *
 */

#ifdef __cplusplus
extern "C"{
#endif

/*=======================================================================
					INCLUDE FILES
=======================================================================*/
/* Standard Include Files */
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <unistd.h>
#include <linux/mxcfb.h>
#include <sys/mman.h>
#include <math.h>
#include <string.h>
#include <malloc.h>
#include <getopt.h>

#include "ginger_rgb_800x600.c"
#include "fsl_rgb_480x360.c"
#include "colorbar_rgb_800x600.c"


#define TFAIL -1
#define TPASS 0

#define TRUE 1
#define FALSE 0

#define BUFFER_FB		0
#define BUFFER_OVERLAY		1

#define WAVEFORM_MODE_INIT	0x0	/* Screen goes to white (clears) */
#define WAVEFORM_MODE_DU	0x1	/* Grey->white/grey->black */
#define WAVEFORM_MODE_GC16	0x2	/* High fidelity (flashing) */
#define WAVEFORM_MODE_GC4	0x3	/* Lower fidelity */
#define WAVEFORM_MODE_A2	0x4	/* Fast black/white animation */

#define EPDC_STR_ID		"mxc_epdc_fb"

#define  ALIGN_PIXEL_128(x)  ((x+ 127) & ~127)

#define CROSSHATCH_ALTERNATING  0
#define CROSSHATCH_COLUMNS_ROWS 1

#define ALLOW_COLLISIONS	0
#define NO_COLLISIONS		1

#define NUM_TESTS		14

__u32 pwrdown_delay = 0;
__u32 scheme = UPDATE_SCHEME_QUEUE_AND_MERGE;
int test_map[NUM_TESTS];
typedef int (*testfunc)(void);
testfunc testfunc_array[NUM_TESTS] = {NULL};

int fd_fb;
int fd_fb_ioctl;
unsigned short * fb;
int g_fb_size;
struct fb_var_screeninfo screen_info;
__u32 marker_val = 1;
int use_animation = 0;
int num_flashes = 10;

void memset_dword(void *s, int c, size_t count)
{
	int i;
	for (i = 0; i < count; i++)
	{
		*((__u32 *)s + i) = c;
	}
}

static void copy_image_to_buffer(int left, int top, int width, int height, uint *img_ptr,
			uint target_buf, struct fb_var_screeninfo *screen_info)
{
	int i;
	uint *fb_ptr;

	if (target_buf == BUFFER_FB)
		fb_ptr =  (uint *)fb;
	else if (target_buf == BUFFER_OVERLAY)
		fb_ptr = (uint *)fb +
			(screen_info->xres_virtual * ALIGN_PIXEL_128(screen_info->yres) *
			screen_info->bits_per_pixel/8)/4;
	else {
		printf("Invalid target buffer specified!\n");
		return;
	}

	if ((width > screen_info->xres) || (height > screen_info->yres)) {
		printf("Bad image dimensions!\n");
		return;
	}

	for (i = 0; i < height; i++)
		memcpy(fb_ptr + ((i + top) * screen_info->xres_virtual + left) * 2 / 4, img_ptr + (i * width) * 2 /4, width * 2);
}

static __u32 update_to_display(int left, int top, int width, int height, int wave_mode,
	int wait_for_complete, uint flags)
{
	struct mxcfb_update_data upd_data;
	struct mxcfb_update_marker_data upd_marker_data;
	int retval;
	int wait = wait_for_complete | (flags & EPDC_FLAG_TEST_COLLISION);
	int max_retry = 10;

	upd_data.update_mode = UPDATE_MODE_PARTIAL;
	upd_data.waveform_mode = wave_mode;
	upd_data.update_region.left = left;
	upd_data.update_region.width = width;
	upd_data.update_region.top = top;
	upd_data.update_region.height = height;
	upd_data.temp = TEMP_USE_AMBIENT;
	upd_data.flags = flags;

	if (wait)
		/* Get unique marker value */
		upd_data.update_marker = marker_val++;
	else
		upd_data.update_marker = 0;

	retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	while (retval < 0) {
		/* We have limited memory available for updates, so wait and
		 * then try again after some updates have completed */
		sleep(1);
		retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
		if (--max_retry <= 0) {
			printf("Max retries exceeded\n");
			wait = 0;
			flags = 0;
			break;
		}
	}

	if (wait) {
		upd_marker_data.update_marker = upd_data.update_marker;

		/* Wait for update to complete */
		retval = ioctl(fd_fb_ioctl, MXCFB_WAIT_FOR_UPDATE_COMPLETE, &upd_marker_data);
		if (retval < 0) {
			printf("Wait for update complete failed.  Error = 0x%x", retval);
			flags = 0;
		}
	}

	if (flags & EPDC_FLAG_TEST_COLLISION) {
		printf("Collision test result = %d\n",
			upd_marker_data.collision_test);
		return upd_marker_data.collision_test;
	} else
		return upd_data.waveform_mode;
}

static void draw_rgb_crosshatch(struct fb_var_screeninfo * var, int mode)
{
	__u32 *stripe_start;
	int i, stripe_cnt;
	__u32 horizontal_color, vertical_color;
	int stripe_width;
	int square_area_side;
	struct mxcfb_update_data upd_data;
	int retval;
	int ioctl_tries;

	/* Draw crossing lines to generate collisions */
	square_area_side = 200;
	horizontal_color = 0x84108410;
	vertical_color = 0x41044104;
	stripe_width = 12;

	if (mode == CROSSHATCH_COLUMNS_ROWS) {
		stripe_cnt = 1;
		while(stripe_cnt * stripe_width * 2 + stripe_width < square_area_side) {

			/* draw vertical strip */
			for (i = 0; i < square_area_side; i++) {

				stripe_start = (__u32 *)fb + ((i * var->xres_virtual)
					+ stripe_width*2*stripe_cnt)/2;

				/* draw stripe */
				memset_dword(stripe_start, horizontal_color, stripe_width / 2);
			}

			upd_data.update_marker = 0;

			upd_data.update_mode = UPDATE_MODE_PARTIAL;
			upd_data.waveform_mode = WAVEFORM_MODE_AUTO;
			upd_data.update_region.left = stripe_width*2*stripe_cnt;
			upd_data.update_region.width = stripe_width;
			upd_data.update_region.top = 0;
			upd_data.update_region.height = square_area_side;
			upd_data.temp = TEMP_USE_AMBIENT;
			upd_data.flags = 0;
			ioctl_tries = 0;
			do {
				/* Insert delay after first try */
				if (ioctl_tries > 0) {
					sleep(1);
					printf("Retrying update\n");
				}
				retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
				ioctl_tries++;
			} while ((retval < 0) && (ioctl_tries < 5));
			if (retval < 0)
			{
				printf("Draw crosshatch vertical bar send update failed. retval = %d\n", retval);
			}

			stripe_cnt++;
		}

		stripe_cnt = 1;
		while(stripe_cnt * stripe_width * 2 + stripe_width < square_area_side) {

			/* draw horizontal strip */
			for (i = 0; i < stripe_width; i++) {
				stripe_start = (__u32 *)fb +
					((stripe_width*2*stripe_cnt + i) *
					var->xres_virtual)/2;

				/* draw stripe */
				memset_dword(stripe_start, vertical_color, square_area_side / 2);
			}

			upd_data.update_marker = 0;

			upd_data.update_mode = UPDATE_MODE_PARTIAL;
			upd_data.waveform_mode = WAVEFORM_MODE_AUTO;
			upd_data.update_region.left = 0;
			upd_data.update_region.width = square_area_side;
			upd_data.update_region.top = stripe_width*2*stripe_cnt;
			upd_data.update_region.height = stripe_width;
			upd_data.temp = TEMP_USE_AMBIENT;
			upd_data.flags = 0;
			ioctl_tries = 0;
			do {
				/* Insert delay after first try */
				if (ioctl_tries > 0) {
					sleep(1);
					printf("Retrying update\n");
				}
				retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
				ioctl_tries++;
			} while ((retval < 0) && (ioctl_tries < 5));
			if (retval < 0)
			{
				printf("Draw crosshatch horizontal bar send update failed. retval = %d\n", retval);
			}

			stripe_cnt++;
		}
	}

	if (mode == CROSSHATCH_ALTERNATING) {
		stripe_cnt = 1;
		while(stripe_cnt * stripe_width * 2 + stripe_width < square_area_side) {

			/* draw vertical strip */
			for (i = 0; i < square_area_side; i++) {

				stripe_start = (__u32 *)fb + ((i * var->xres_virtual)
					+ stripe_width*2*stripe_cnt)/2;

				/* draw stripe */
				memset_dword(stripe_start, horizontal_color, stripe_width / 2);
			}

			upd_data.update_marker = 0;

			upd_data.update_mode = UPDATE_MODE_PARTIAL;
			upd_data.waveform_mode = WAVEFORM_MODE_AUTO;
			upd_data.update_region.left = stripe_width*2*stripe_cnt;
			upd_data.update_region.width = stripe_width;
			upd_data.update_region.top = 0;
			upd_data.update_region.height = square_area_side;
			upd_data.temp = TEMP_USE_AMBIENT;
			upd_data.flags = 0;
			ioctl_tries = 0;
			do {
				/* Insert delay after first try */
				if (ioctl_tries > 0) {
					sleep(1);
					printf("Retrying update\n");
				}
				retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
				ioctl_tries++;
			} while ((retval < 0) && (ioctl_tries < 5));
			if (retval < 0)
			{
				printf("Draw crosshatch vertical bar send update failed. retval = %d\n", retval);
			}

			/* draw horizontal strip */
			for (i = 0; i < stripe_width; i++) {
				stripe_start = (__u32 *)fb +
					((stripe_width*2*stripe_cnt + i) *
					var->xres_virtual)/2;

				/* draw stripe */
				memset_dword(stripe_start, vertical_color, square_area_side / 2);
			}

			upd_data.update_marker = 0;

			upd_data.update_mode = UPDATE_MODE_PARTIAL;
			upd_data.waveform_mode = WAVEFORM_MODE_AUTO;
			upd_data.update_region.left = 0;
			upd_data.update_region.width = square_area_side;
			upd_data.update_region.top = stripe_width*2*stripe_cnt;
			upd_data.update_region.height = stripe_width;
			upd_data.temp = TEMP_USE_AMBIENT;
			upd_data.flags = 0;
			ioctl_tries = 0;
			do {
				/* Insert delay after first try */
				if (ioctl_tries > 0) {
					sleep(1);
					printf("Retrying update\n");
				}
				retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
				ioctl_tries++;
			} while ((retval < 0) && (ioctl_tries < 5));
			if (retval < 0)
			{
				printf("Draw crosshatch horizontal bar send update failed. retval = %d\n", retval);
			}

			stripe_cnt++;
		}
	}
}

static void draw_rgb_color_squares(struct fb_var_screeninfo * var)
{
	int i, j;
	int *fbp = (int *)fb;

	/* Draw red square */
	for (i = 50; i < 250; i++)
		for (j = 50; j < 250; j += 2)
			fbp[(i*var->xres_virtual+j)*2/4] = 0xF800F800;

	/* Draw green square */
	for (i = 50; i < 250; i++)
		for (j = 350; j < 550; j += 2)
			fbp[(i*var->xres_virtual+j)*2/4] = 0x07E007E0;

	/* Draw blue square */
	for (i = 350; i < 550; i++)
		for (j = 50; j < 250; j += 2)
			fbp[(i*var->xres_virtual+j)*2/4] = 0x001F001F;

	/* Draw black square */
	for (i = 350; i < 550; i++)
		for (j = 350; j < 550; j += 2)
			fbp[(i*var->xres_virtual+j)*2/4] = 0x00000000;
}

static void draw_y_colorbar(struct fb_var_screeninfo * var)
{
	uint *upd_buf_ptr, *stripe_start;
	int i, stripe_cnt;
	__u32 grey_val, grey_increment;
	__u32 grey_dword;
	int num_stripes, stripe_length, stripe_width;

	upd_buf_ptr = (uint *)fb;

	num_stripes = 16;
	stripe_width = var->xres / num_stripes;
	stripe_width += (4 - stripe_width % 4) % 4;
	grey_increment = 0x100 / num_stripes;

	grey_val = 0x00;
	grey_dword = 0x00000000;

	/*
	 * Generate left->right color bar
	 */
	for (stripe_cnt = 0; stripe_cnt < num_stripes; stripe_cnt++) {
		for (i = 0; i < var->yres; i++) {
			stripe_start =
			    upd_buf_ptr + ((i * var->xres_virtual) +
					   (stripe_width * stripe_cnt)) / 4;
			if ((stripe_width * (stripe_cnt + 1)) > var->xres)
				stripe_length =
				    var->xres - (stripe_width * stripe_cnt);
			else
				stripe_length = stripe_width;

			grey_dword =
			    grey_val | (grey_val << 8) | (grey_val << 16) |
			    (grey_val << 24);

			/* draw stripe */
			memset_dword(stripe_start, grey_dword,
				    stripe_length / 4);
		}
		/* increment grey value to darken next stripe */
		grey_val += grey_increment;
	}
}

static void draw_square_outline(int x, int y, int side, int thickness, __u16 color)
{
	int i, j;
	__u8 *fbp8 = (__u8*)fb;
	__u16 *fbp16 = (__u16*)fb;
	int bypp = screen_info.bits_per_pixel/8;
	int xres = screen_info.xres_virtual;

	if (bypp == 1) {
		/* Draw square top */
		for (i = y; i < y + thickness; i++)
			memset(fbp8 + (i*xres+x), color, side);

		/* Draw sides */
		for (i = y + thickness; i < y + side - thickness; i++) {
			memset(fbp8 + (i*xres+x), color, thickness);
			memset(fbp8 + (i*xres+x+side-thickness), color, thickness);
		}

		/* Draw square bottom */
		for (i = y + side - thickness; i < y + side; i++)
			memset(fbp8 + (i*xres+x), color, side);
	} else {
		/* Draw square top */
		for (i = y; i < y + thickness; i++)
			for (j = x; j < x + side; j++)
				fbp16[i*xres+j] = color;

		/* Draw sides */
		for (i = y + thickness; i < y + side - thickness; i++) {
			for (j = x; j < x + thickness; j++)
				fbp16[i*xres+j] = color;
			for (j = x + side - thickness; j < x + side; j++)
				fbp16[i*xres+j] = color;
		}

		/* Draw square bottom */
		for (i = y + side - thickness; i < y + side; i++)
			for (j = x; j < x + side; j++)
				fbp16[i*xres+j] = color;
	}
}

static void draw_rectangle(unsigned short *fb_ptr, int x, int y, int width,
						int height, __u16 color)
{
	int i, j;
	int *fbp = (int *)fb_ptr;
	__u32 colorval = color | (color << 16);

	for (i = y; i < y + height; i++)
		for (j = x; j < x + width; j += 2)
			fbp[(i*screen_info.xres_virtual+j)*2/4] = colorval;
}

static int test_updates(void)
{
	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_DU, TRUE, 0);

	printf("Crosshatch updates\n");
	draw_rgb_crosshatch(&screen_info, CROSSHATCH_ALTERNATING);

	sleep(3);

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_DU, TRUE, 0);

	printf("Squares update\n");
	draw_rgb_color_squares(&screen_info);

	/* Update each square */
	update_to_display(50, 50, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
	update_to_display(350, 50, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
	update_to_display(50, 350, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
	update_to_display(350, 350, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_DU, TRUE, 0);

	printf("FSL updates\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	copy_image_to_buffer(300, 0, 480, 360, fsl_rgb_480x360, BUFFER_FB, &screen_info);
	update_to_display(300, 0, 480, 560, WAVEFORM_MODE_AUTO, TRUE, 0);

	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	copy_image_to_buffer(300, 48, 480, 360, fsl_rgb_480x360, BUFFER_FB, &screen_info);
	update_to_display(300, 0, 480, 560, WAVEFORM_MODE_AUTO, TRUE, 0);

	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	copy_image_to_buffer(300, 100, 480, 360, fsl_rgb_480x360, BUFFER_FB, &screen_info);
	update_to_display(300, 0, 480, 560, WAVEFORM_MODE_AUTO, TRUE, 0);

	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	copy_image_to_buffer(300, 148, 480, 360, fsl_rgb_480x360, BUFFER_FB, &screen_info);
	update_to_display(300, 0, 480, 560, WAVEFORM_MODE_AUTO, TRUE, 0);

	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	copy_image_to_buffer(300, 200, 480, 360, fsl_rgb_480x360, BUFFER_FB, &screen_info);
	update_to_display(300, 0, 480, 560, WAVEFORM_MODE_AUTO, TRUE, 0);

	printf("Ginger update\n");
	copy_image_to_buffer(0, 0, 800, 600, ginger_rgb_800x600, BUFFER_FB, &screen_info);
	update_to_display(0, 0, 800, 600, WAVEFORM_MODE_AUTO, FALSE, 0);

	sleep(3);

	printf("Colorbar update\n");
	copy_image_to_buffer(0, 0, 800, 600, colorbar_rgb_800x600, BUFFER_FB, &screen_info);
	update_to_display(0, 0, 800, 600, WAVEFORM_MODE_AUTO, FALSE, 0);

	sleep(3);

	return TPASS;
}

static int test_rotation(void)
{
	int retval = TPASS;
	int i, j;

	for (i = FB_ROTATE_UR; i <= FB_ROTATE_CCW; i++) {
		printf("Blank screen\n");
		memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_DU, TRUE, 0);

		printf("Rotating FB 90 degrees\n");
		screen_info.rotate = i;
		screen_info.bits_per_pixel = 16;
		screen_info.grayscale = 0;
		retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
		if (retval < 0)
		{
			printf("Rotation failed\n");
			return TFAIL;
		}

		printf("New dimensions: xres = %d, xres_virtual = %d,"
			"yres = %d, yres_virtual = %d\n",
			screen_info.xres, screen_info.xres_virtual,
			screen_info.yres, screen_info.yres_virtual);

		printf("Rotated FSL\n");
		copy_image_to_buffer(0, 0, 480, 360, fsl_rgb_480x360, BUFFER_FB, &screen_info);
		update_to_display(0, 0, 480, 360, WAVEFORM_MODE_AUTO, FALSE, 0);

		sleep(1);
		printf("Blank screen\n");
		memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_DU, TRUE, 0);

		printf("Rotated squares\n");
		draw_rgb_color_squares(&screen_info);
		update_to_display(50, 50, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
		update_to_display(350, 50, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
		update_to_display(50, 350, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
		update_to_display(350, 350, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);

		sleep(1);

		printf("Blank screen\n");
		memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_DU, TRUE, 0);

		printf("Rotated crosshatch updates\n");
		draw_rgb_crosshatch(&screen_info, CROSSHATCH_COLUMNS_ROWS);

		sleep(3);

		printf("Blank screen\n");
		memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_DU, TRUE, 0);

		printf("Draw square outline (RGB)\n");
		for (j = 8; j >= 4; j--) {
			memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
			update_to_display(0, 0, screen_info.xres, screen_info.yres,
				WAVEFORM_MODE_DU, TRUE, 0);
			printf("*** Try again at %d,%d ***\n", j, j);
			draw_square_outline(8, 8, 96, 2, 0);
			update_to_display(j, j, 104 - j, 104 - j, WAVEFORM_MODE_AUTO, TRUE, 0);
		}

		sleep(1);

		printf("Unrotated, changing to y8\n");
		screen_info.bits_per_pixel = 8;
		screen_info.grayscale = GRAYSCALE_8BIT;
		retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
		if (retval < 0)
		{
			printf("Rotation failed\n");
			return TFAIL;
		}

		printf("Draw square outline (8-bit gray)\n");
		for (j = 8; j >= 0; j--) {
			memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
			update_to_display(0, 0, screen_info.xres, screen_info.yres,
				WAVEFORM_MODE_DU, TRUE, 0);
			printf("*** Try again at %d,%d ***\n", j, j);
			draw_square_outline(8, 8, 96, 2, 0);
			update_to_display(j, j, 104 - j, 104 - j, WAVEFORM_MODE_AUTO, TRUE, 0);
		}

		sleep(1);

		for (j = 9; j <= 14; j++) {
			memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
			update_to_display(0, 0, screen_info.xres, screen_info.yres,
				WAVEFORM_MODE_DU, TRUE, 0);
			printf("*** Shift square left/down by 1 pixel (%d,%d) ***\n", j, j);
			draw_square_outline(j, j, 96, 2, 0);
			update_to_display(j, j, 96, 96, WAVEFORM_MODE_AUTO, TRUE, 0);
		}

		sleep(1);
	}

	printf("Change back to non-inverted RGB565\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 16;
	screen_info.grayscale = 0;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Back to normal failed\n");
		return TFAIL;
	}

	return TPASS;
}

static int test_y8(void)
{
	int retval = TPASS;
	int iter;

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, 0);

	/*
	 * Do Y8 FB test sequence twice:
	 * - once for normal Y8 (grayscale = 1)
	 * - once for inverted Y8 (grayscale = 2)
	 */
	for (iter = 1; iter < 3; iter++) {
		if (iter == GRAYSCALE_8BIT)
			printf("Changing to Y8 Framebuffer\n");
		else if (iter == GRAYSCALE_8BIT_INVERTED)
			printf("Changing to Inverted Y8 Framebuffer\n");
		screen_info.rotate = FB_ROTATE_CW;
		screen_info.bits_per_pixel = 8;
		screen_info.grayscale = iter;
		retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
		if (retval < 0)
		{
			printf("Rotate and go to Y8 failed\n");
			return TFAIL;
		}

		printf("Top-half black\n");
		memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*
			screen_info.bits_per_pixel/8);
		memset(fb, 0x00, screen_info.xres_virtual*screen_info.yres*
			screen_info.bits_per_pixel/8/2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, FALSE, 0);

		sleep(3);

		printf("Draw Y8 colorbar\n");
		draw_y_colorbar(&screen_info);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, FALSE, 0);

		sleep(3);
	}

	printf("Change back to non-inverted RGB565\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 16;
	screen_info.grayscale = 0;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Back to normal failed\n");
		return TFAIL;
	}

	return TPASS;
}

static int test_auto_waveform(void)
{
	int retval = TPASS;
	int i, j;

	/*
	 * Note: i.MX 6 EPDC does not support returning the waveform
	 * used in the update data structure, like the i.MX 508 does.
	 * This is because the i.MX 508 uses the PxP to select the the
	 * waveform and it is known before the update is sent to the EPDC.
	 */

	printf("Change to Y8 FB\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 8;
	screen_info.grayscale = GRAYSCALE_8BIT;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Change to Y8 failed\n");
		return TFAIL;
	}

	printf("Blank screen - auto-selected to DU\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	/* Draw 0x5 square */
	for (i = 10; i < 50; i++)
		for (j = 10; j < 50; j++)
			*((__u8 *)fb + (i*screen_info.xres_virtual+j)) = 0x50;

	/* Draw 0xA square */
	for (i = 60; i < 100; i++)
		for (j = 60; j < 100; j++)
			*((__u8 *)fb + (i*screen_info.xres_virtual+j)) = 0xA0;

	printf("Update auto-selected to GC4\n");
	update_to_display(0, 0, 100, 100, WAVEFORM_MODE_AUTO, FALSE, 0);

	sleep(2);

	/* Taint the GC4 region */
	fb[5] = 0x8080;

	printf("Update auto-selected to GC16\n");
	update_to_display(0, 0, 100, 100, WAVEFORM_MODE_AUTO, FALSE, 0);

	sleep(3);

	printf("Back to RGB565\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 16;
	screen_info.grayscale = 0;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Setting RGB565 failed\n");
		return TFAIL;
	}

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*
		screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	sleep(2);

	/* Test 8x8 alignment handling */
	printf("Draw gray junk at (18,0)\n");
	draw_rectangle(fb, 18, 0, 2, 18, 0xE71C);

	update_to_display(0, 0, 17, 18, WAVEFORM_MODE_AUTO, 0, 0);

	update_to_display(0, 0, 16, 18, WAVEFORM_MODE_AUTO, 0, 0);

	/* Test input alignment */
	printf("Draw gray junk at (1,0)\n");
	draw_rectangle(fb, 1, 0, 2, 18, 0xE71C);

	update_to_display(4, 0, 10, 18, WAVEFORM_MODE_AUTO, 0, 0);

	update_to_display(3, 0, 10, 18, WAVEFORM_MODE_AUTO, 0, 0);

	update_to_display(0, 0, 10, 18, WAVEFORM_MODE_AUTO, 0, 0);

	return TPASS;
}

static int test_auto_update(void)
{
	int retval = TPASS;
	int auto_update_mode;
	int i;

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_DU, TRUE, 0);

	printf("Change to auto-update mode\n");
	auto_update_mode = AUTO_UPDATE_MODE_AUTOMATIC_MODE;
	retval = ioctl(fd_fb_ioctl, MXCFB_SET_AUTO_UPDATE_MODE, &auto_update_mode);
	if (retval < 0)
	{
		return TFAIL;
	}

	printf("Auto-update 1st 5 lines\n");
	for (i = 0; i < 5; i++) {
		memset(fb, 0x00, screen_info.xres_virtual*5*screen_info.bits_per_pixel/8);
	}

	sleep(1);

	printf("Auto-update middle and lower stripes\n");
	for (i = 0; i < 5; i++)
		memset(fb + screen_info.xres_virtual*300*screen_info.bits_per_pixel/8/2, 0x00, screen_info.xres_virtual*5*screen_info.bits_per_pixel/8);
	for (i = 0; i < 5; i++)
		memset(fb + screen_info.xres_virtual*500*screen_info.bits_per_pixel/8/2, 0x00, screen_info.xres_virtual*5*screen_info.bits_per_pixel/8);

	sleep(1);
	printf("Auto-update blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);

	sleep(1);

	printf("Auto-update FSL logo\n");
	copy_image_to_buffer(0, 0, 480, 360, fsl_rgb_480x360, BUFFER_FB, &screen_info);

	sleep(2);

	printf("Change to region update mode\n");
	auto_update_mode = AUTO_UPDATE_MODE_REGION_MODE;
	retval = ioctl(fd_fb_ioctl, MXCFB_SET_AUTO_UPDATE_MODE, &auto_update_mode);
	if (retval < 0)
	{
		return TFAIL;
	}

	sleep(2);
	printf("Return rotation to 0 degrees\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 16;
	screen_info.grayscale = 0;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Rotation failed\n");
		return TFAIL;
	}

	return TPASS;
}


static int test_pan(void)
{
	int y, i;
	int retval;
	uint backbuf_offs;

	printf("Draw to offscreen region.\n");
	copy_image_to_buffer(0, 0, 800, 600, colorbar_rgb_800x600,
		BUFFER_OVERLAY, &screen_info);

	printf("Ginger update\n");
	copy_image_to_buffer(0, 0, 800, 600, ginger_rgb_800x600, BUFFER_FB,
		&screen_info);
	update_to_display(0, 0, 800, 600, WAVEFORM_MODE_AUTO, TRUE, 0);

	printf("Panned to colorbar\n");
	screen_info.yoffset = screen_info.yres;
	retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
	if (retval < 0) {
		printf("Pan fail!\n");
	}

	printf("Updating (0,0,40,40)\n");
	update_to_display(0, 0, 40, 40,
		WAVEFORM_MODE_AUTO, FALSE, 0);

	usleep(100);
	sleep(1);

	printf("Updating (300,300,40,40)\n");
	update_to_display(300, 300, 40, 40,
		WAVEFORM_MODE_AUTO, FALSE, 0);

	usleep(100);
	sleep(1);

	printf("Updating (15,400,100,100)\n");
	update_to_display(15, 400, 100, 100,
		WAVEFORM_MODE_AUTO, FALSE, 0);

	usleep(100);
	sleep(1);

	printf("Updating (300,21,43,43)\n");
	update_to_display(300, 21, 43, 43,
		WAVEFORM_MODE_AUTO, FALSE, 0);

	usleep(100);
	sleep(1);

	printf("Updating (400,300,399,299)\n");
	update_to_display(400, 300, 399, 299,
		WAVEFORM_MODE_AUTO, FALSE, 0);

	usleep(100);
	sleep(1);

	printf("Updating (400,0,400,600)\n");
	update_to_display(400, 0, 400, 600,
		WAVEFORM_MODE_AUTO, FALSE, 0);

	usleep(100);
	sleep(1);

	printf("Updating (0,0,400,600)\n");
	update_to_display(0, 0, 400, 600,
		WAVEFORM_MODE_AUTO, FALSE, 0);

	sleep(5);

	for (y = 0; (y + screen_info.yres <= screen_info.yres * 2) &&
		(y + screen_info.yres <= screen_info.yres_virtual); y+=50) {
		screen_info.yoffset = y;
		retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
		if (retval < 0) {
			printf("Pan fail!\n");
			break;
		}
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, TRUE, 0);
	}

	screen_info.yoffset = 0;
	retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
	if (retval < 0)
		printf("Pan fail!\n");

	printf("Returned to original panning offset.\n");

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_DU, TRUE, 0);

	backbuf_offs = screen_info.xres_virtual*screen_info.yres;

	printf("Use pan to flip between black & white buttons\n");
	for (i = 1; i <= 6; i++) {
		/* draw black */
		draw_rectangle(fb, 100*i, 100, 80, 80, 0);
		draw_rectangle(fb + backbuf_offs, 100*i, 100, 80, 80, 0);
		screen_info.yoffset = 0;
		retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
		if (retval < 0)
			printf("Pan fail!\n");
		update_to_display(100*i, 100, 80, 80, WAVEFORM_MODE_GC16, FALSE, 0);

		usleep(100000);

		/* draw white */
		draw_rectangle(fb, 100*i, 100, 80, 80, 0xFFFF);
		draw_rectangle(fb + backbuf_offs, 100*i, 100, 80, 80, 0xFFFF);
		screen_info.yoffset = screen_info.yres;
		retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
		if (retval < 0)
			printf("Pan fail!\n");
		update_to_display(100*i, 100, 80, 80, WAVEFORM_MODE_GC16, FALSE, 0);

		usleep(100000);
	}
	printf("Done panning sequence - display should be white\n");

	sleep(1);

	printf("Flash button using pixel inversion\n");
	draw_rectangle(fb, 100, 200, 80, 80, 0xFFFF);
	draw_rectangle(fb + backbuf_offs, 100, 200, 80, 80, 0xFFFF);
	for (i = 0; i < 4; i++) {
		update_to_display(100, 200, 80, 80, WAVEFORM_MODE_GC16, FALSE, EPDC_FLAG_ENABLE_INVERSION);
		update_to_display(100, 200, 80, 80, WAVEFORM_MODE_GC16, FALSE, 0);
	}
	printf("Done inversion sequence - display should be white\n");

	sleep(2);

	printf("Flash button using panning\n");
	for (i = 0; i < 4; i++) {
		/* Draw black button */
		draw_rectangle(fb, 0, 0, 80, 80, 0x0);
		draw_rectangle(fb + backbuf_offs, 0, 0, 80, 80, 0x0);
		screen_info.yoffset = 0;
		retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
		if (retval < 0)
			printf("Pan fail!\n");
		update_to_display(0, 0, 80, 80, WAVEFORM_MODE_GC16, FALSE, 0);
		usleep(100);

		/* Draw white button */
		draw_rectangle(fb, 0, 0, 80, 80, 0xFFFF);
		draw_rectangle(fb + backbuf_offs, 0, 0, 80, 80, 0xFFFF);
		screen_info.yoffset = screen_info.yres;
		retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
		if (retval < 0)
			printf("Pan fail!\n");
		update_to_display(0, 0, 80, 80, WAVEFORM_MODE_GC16, FALSE, 0);
		usleep(100);
	}
	printf("Done button panning sequence - display should be white\n");

	sleep(5);
	printf("Pan test done.\n");

	screen_info.yoffset = 0;
	retval = ioctl(fd_fb, FBIOPAN_DISPLAY, &screen_info);
	if (retval < 0)
		printf("Pan fail!\n");

	printf("Returned to original panning offset.\n");
	sleep(1);
	return TPASS;
}


static int test_overlay(void)
{
	struct mxcfb_update_data upd_data;
	struct mxcfb_update_marker_data update_marker_data;
	int retval;
	struct fb_fix_screeninfo fix_screen_info;
	__u32 ol_phys_addr;

	printf("Ginger update\n");
	copy_image_to_buffer(0, 0, 800, 600, ginger_rgb_800x600, BUFFER_FB,
		&screen_info);
	update_to_display(0, 0, 800, 600, WAVEFORM_MODE_AUTO, TRUE, 0);

	sleep(1);

	retval = ioctl(fd_fb, FBIOGET_FSCREENINFO, &fix_screen_info);
	if (retval < 0)
		return TFAIL;

	/* Fill overlay buffer with data */
	copy_image_to_buffer(0, 0, 800, 600, colorbar_rgb_800x600,
		BUFFER_OVERLAY, &screen_info);

	ol_phys_addr = fix_screen_info.smem_start +
		screen_info.xres_virtual*ALIGN_PIXEL_128(screen_info.yres)*screen_info.bits_per_pixel/8;

	upd_data.update_mode = UPDATE_MODE_FULL;
	upd_data.waveform_mode = WAVEFORM_MODE_AUTO;
	upd_data.update_region.left = 0;
	upd_data.update_region.width = screen_info.xres;
	upd_data.update_region.top = 0;
	upd_data.update_region.height = screen_info.yres;
	upd_data.temp = TEMP_USE_AMBIENT;
	upd_data.update_marker = marker_val++;

	upd_data.flags = EPDC_FLAG_USE_ALT_BUFFER;
	/* Overlay buffer data */
	upd_data.alt_buffer_data.phys_addr = ol_phys_addr;
	upd_data.alt_buffer_data.width = screen_info.xres_virtual;
	upd_data.alt_buffer_data.height = screen_info.yres;
	upd_data.alt_buffer_data.alt_update_region.left = 0;
	upd_data.alt_buffer_data.alt_update_region.width = screen_info.xres;
	upd_data.alt_buffer_data.alt_update_region.top = 0;
	upd_data.alt_buffer_data.alt_update_region.height = screen_info.yres;

	printf("Show full-screen overlay\n");

	retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	while (retval < 0) {
		/* We have limited memory available for updates, so wait and
		 * then try again after some updates have completed */
		sleep(1);
		retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	}

	/* Wait for update to complete */
	update_marker_data.update_marker = upd_data.update_marker;
	retval = ioctl(fd_fb_ioctl, MXCFB_WAIT_FOR_UPDATE_COMPLETE, &update_marker_data);
	if (retval < 0) {
		printf("Wait for update complete failed.  Error = 0x%x", retval);
	}

	sleep(2);

	printf("Show FB contents again\n");

	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	sleep(1);

	printf("Show top half overlay\n");

	/* Update region of overlay shown */
	upd_data.update_region.left = 0;
	upd_data.update_region.width = screen_info.xres;
	upd_data.update_region.top = 0;
	upd_data.update_region.height = screen_info.yres/2;
	upd_data.update_marker = marker_val++;
	upd_data.alt_buffer_data.alt_update_region.left = 0;
	upd_data.alt_buffer_data.alt_update_region.width = screen_info.xres;
	upd_data.alt_buffer_data.alt_update_region.top = 0;
	upd_data.alt_buffer_data.alt_update_region.height = screen_info.yres/2;

	retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	while (retval < 0) {
		/* We have limited memory available for updates, so wait and
		 * then try again after some updates have completed */
		sleep(1);
		retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	}

	/* Wait for update to complete */
	update_marker_data.update_marker = upd_data.update_marker;
	retval = ioctl(fd_fb_ioctl, MXCFB_WAIT_FOR_UPDATE_COMPLETE, &update_marker_data);
	if (retval < 0) {
		printf("Wait for update complete failed.  Error = 0x%x", retval);
	}

	sleep(2);

	printf("Show FB contents again\n");

	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	sleep(1);

	printf("Show cutout region of overlay\n");

	/* Update region of overlay shown */
	upd_data.update_region.left = screen_info.xres/4;
	upd_data.update_region.width = screen_info.xres/2;
	upd_data.update_region.top = screen_info.yres/4;
	upd_data.update_region.height = screen_info.yres/2;
	upd_data.update_marker = marker_val++;
	upd_data.alt_buffer_data.alt_update_region.left = screen_info.xres/4;
	upd_data.alt_buffer_data.alt_update_region.width = screen_info.xres/2;
	upd_data.alt_buffer_data.alt_update_region.top = screen_info.yres/4;
	upd_data.alt_buffer_data.alt_update_region.height = screen_info.yres/2;

	retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	while (retval < 0) {
		/* We have limited memory available for updates, so wait and
		 * then try again after some updates have completed */
		sleep(1);
		retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	}

	/* Wait for update to complete */
	update_marker_data.update_marker = upd_data.update_marker;
	retval = ioctl(fd_fb_ioctl, MXCFB_WAIT_FOR_UPDATE_COMPLETE, &update_marker_data);
	if (retval < 0) {
		printf("Wait for update complete failed.  Error = 0x%x", retval);
	}

	sleep(1);

	printf("Show cutout in upper corner\n");

	/* Update region of overlay shown */
	upd_data.update_region.left = 0;
	upd_data.update_region.width = screen_info.xres/2;
	upd_data.update_region.top = 0;
	upd_data.update_region.height = screen_info.yres/2;
	upd_data.update_marker = marker_val++;
	upd_data.alt_buffer_data.alt_update_region.left = screen_info.xres/4;
	upd_data.alt_buffer_data.alt_update_region.width = screen_info.xres/2;
	upd_data.alt_buffer_data.alt_update_region.top = screen_info.yres/4;
	upd_data.alt_buffer_data.alt_update_region.height = screen_info.yres/2;

	retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	while (retval < 0) {
		/* We have limited memory available for updates, so wait and
		 * then try again after some updates have completed */
		sleep(1);
		retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	}

	/* Wait for update to complete */
	update_marker_data.update_marker = upd_data.update_marker;
	retval = ioctl(fd_fb_ioctl, MXCFB_WAIT_FOR_UPDATE_COMPLETE, &update_marker_data);
	if (retval < 0) {
		printf("Wait for update complete failed.  Error = 0x%x", retval);
	}

	sleep(1);


	printf("Test overlay at 90 degree rotation\n");
	screen_info.rotate = FB_ROTATE_CW;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Rotation failed\n");
		return TFAIL;
	}

	/* FB to black */
	printf("Background to black\n");
	memset(fb, 0x00, screen_info.xres*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	printf("Show rotated overlay in center\n");
	copy_image_to_buffer(0, 0, 480, 360, fsl_rgb_480x360,
		BUFFER_OVERLAY, &screen_info);

	ol_phys_addr = fix_screen_info.smem_start +
		screen_info.xres_virtual*ALIGN_PIXEL_128(screen_info.yres)*screen_info.bits_per_pixel/8;
	upd_data.alt_buffer_data.phys_addr = ol_phys_addr;
	upd_data.alt_buffer_data.width = screen_info.xres_virtual;
	upd_data.alt_buffer_data.height = screen_info.yres;

	/* Update region of overlay shown */
	upd_data.update_region.left = (screen_info.xres - 480)/2;
	upd_data.update_region.width = 480;
	upd_data.update_region.top = (screen_info.yres - 360)/2;
	upd_data.update_region.height = 360;
	upd_data.update_marker = marker_val++;
	upd_data.alt_buffer_data.alt_update_region.left = 0;
	upd_data.alt_buffer_data.alt_update_region.width = 480;
	upd_data.alt_buffer_data.alt_update_region.top = 0;
	upd_data.alt_buffer_data.alt_update_region.height = 360;

	retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	while (retval < 0) {
		/* We have limited memory available for updates, so wait and
		 * then try again after some updates have completed */
		sleep(1);
		retval = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	}

	/* Wait for update to complete */
	update_marker_data.update_marker = upd_data.update_marker;
	retval = ioctl(fd_fb_ioctl, MXCFB_WAIT_FOR_UPDATE_COMPLETE, &update_marker_data);
	if (retval < 0) {
		printf("Wait for update complete failed.  Error = 0x%x", retval);
	}

	sleep(3);

	printf("Revert rotation\n");
	screen_info.rotate = FB_ROTATE_UR;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Rotation failed\n");
		return TFAIL;
	}

	return TPASS;
}

static int test_animation_mode(void)
{
	int retval = TPASS;
	int iter;
	int wave_mode = use_animation ? WAVEFORM_MODE_A2 : WAVEFORM_MODE_AUTO;

	int i;

	printf("Blank screen\n");
	memset(fb, 0xFF, 800*600*2);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	sleep(2);

	/* Swap quickly back-and-forth between black and white screen */
	for (i = 0; i < num_flashes; i++) {
		/* black */
		memset(fb, 0x00, 800*600*2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			wave_mode, TRUE, EPDC_FLAG_FORCE_MONOCHROME);
		/* white */
		memset(fb, 0xFF, 800*600*2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			wave_mode, TRUE, EPDC_FLAG_FORCE_MONOCHROME);
	}

	draw_rgb_color_squares(&screen_info);

	printf("Squares update normal\n");
	update_to_display(50, 50, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
	update_to_display(350, 50, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
	update_to_display(50, 350, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);
	update_to_display(350, 350, 200, 200, WAVEFORM_MODE_AUTO, TRUE, 0);

	sleep(2);

	/*
	 * Rule for animiation mode is that you must enter and exit all
	 * white or all black
	 */
	if (use_animation) {
		printf("Blank screen\n");
		memset(fb, 0xFF, 800*600*2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, TRUE, 0);
		draw_rgb_color_squares(&screen_info);
	}

	printf("Squares update black/white\n");
	update_to_display(50, 50, 200, 200, wave_mode, TRUE,
		EPDC_FLAG_FORCE_MONOCHROME);
	update_to_display(350, 50, 200, 200, wave_mode, TRUE,
		EPDC_FLAG_FORCE_MONOCHROME);
	update_to_display(50, 350, 200, 200, wave_mode, TRUE,
		EPDC_FLAG_FORCE_MONOCHROME);
	update_to_display(350, 350, 200, 200, wave_mode, TRUE,
		EPDC_FLAG_FORCE_MONOCHROME);

	sleep(2);
	if (use_animation) {
		printf("Blank screen\n");
		memset(fb, 0xFF, 800*600*2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, TRUE, 0);
	}

	copy_image_to_buffer(0, 0, 800, 600, ginger_rgb_800x600, BUFFER_FB, &screen_info);
	printf("Ginger update normal\n");
	update_to_display(0, 0, 800, 600, WAVEFORM_MODE_AUTO, TRUE, 0);
	sleep(2);

	if (use_animation) {
		printf("Blank screen\n");
		memset(fb, 0xFF, 800*600*2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, TRUE, 0);
		copy_image_to_buffer(0, 0, 800, 600, ginger_rgb_800x600,
			BUFFER_FB, &screen_info);
	}

	printf("Ginger update black/white\n");
	update_to_display(0, 0, 800, 600, wave_mode, TRUE, EPDC_FLAG_FORCE_MONOCHROME);
	sleep(2);

	if (use_animation) {
		printf("Blank screen\n");
		memset(fb, 0xFF, 800*600*2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, TRUE, 0);
	}

	copy_image_to_buffer(0, 0, 800, 600, colorbar_rgb_800x600, BUFFER_FB, &screen_info);
	printf("Colorbar update normal\n");
	update_to_display(0, 0, 800, 600, WAVEFORM_MODE_AUTO, TRUE, 0);
	sleep(2);
	if (use_animation) {
		printf("Blank screen\n");
		memset(fb, 0xFF, 800*600*2);
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, TRUE, 0);
		copy_image_to_buffer(0, 0, 800, 600, colorbar_rgb_800x600,
			BUFFER_FB, &screen_info);
	}
	printf("Colorbar update black/white\n");
	update_to_display(0, 0, 800, 600, wave_mode, TRUE, EPDC_FLAG_FORCE_MONOCHROME);

	sleep(2);


	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, 0);

	sleep(2);

	/*
	 * Do Y8 FB test sequence twice:
	 * - once for normal Y8 (grayscale = 1)
	 * - once for inverted Y8 (grayscale = 2)
	 */
	for (iter = 1; iter < 3; iter++) {
		if (iter == GRAYSCALE_8BIT)
			printf("Changing to Y8 Framebuffer\n");
		else if (iter == GRAYSCALE_8BIT_INVERTED)
			printf("Changing to Inverted Y8 Framebuffer\n");
		screen_info.rotate = FB_ROTATE_CW;
		screen_info.bits_per_pixel = 8;
		screen_info.grayscale = iter;
		retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
		if (retval < 0)
		{
			printf("Rotate and go to Y8 failed\n");
			return TFAIL;
		}

		draw_y_colorbar(&screen_info);
		printf("Draw Y8 colorbar normal\n");
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			WAVEFORM_MODE_AUTO, TRUE, 0);

		sleep(2);
		if (use_animation) {
			printf("Blank screen\n");
			memset(fb, 0xFF, 800*600*2);
			update_to_display(0, 0, screen_info.xres,
				screen_info.yres,
				WAVEFORM_MODE_AUTO, TRUE, 0);
			draw_y_colorbar(&screen_info);
		}

		printf("Draw Y8 colorbar black/white\n");
		update_to_display(0, 0, screen_info.xres, screen_info.yres,
			wave_mode, TRUE, EPDC_FLAG_FORCE_MONOCHROME);

		sleep(2);
		if (use_animation) {
			printf("Blank screen\n");
			memset(fb, 0xFF, 800*600*2);
			update_to_display(0, 0, screen_info.xres,
				screen_info.yres,
				WAVEFORM_MODE_AUTO, TRUE, 0);
		}
	}

	printf("Change back to non-inverted RGB565\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 16;
	screen_info.grayscale = 0;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Back to normal failed\n");
		return TFAIL;
	}


	return retval;
}

static int test_fast_square(void)
{
	int retval = TPASS;
	int xpos, ypos, last_pos;
	int side_len, increment;
	int first_go;
	int wait_upd_compl = NO_COLLISIONS;
	int wave_mode = use_animation ? WAVEFORM_MODE_A2 : WAVEFORM_MODE_AUTO;

	/********************************************
	* RGB image tests
	********************************************/

	printf("Blank screen\n");
	memset(fb, 0xFF, 800*600*2);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	xpos = 20;
	last_pos = 0;
	ypos = 110;
	side_len = 100;
	increment = 40;
	first_go = 1;

	while (xpos + side_len <= screen_info.xres)
	{
		/* Clear last square (set area to white) */
		if (!first_go) {
			draw_rectangle(fb, last_pos, ypos, side_len, side_len,
					0xFFFF);
			update_to_display(last_pos, ypos, side_len, side_len,
				wave_mode,
				wait_upd_compl, 0);
		}

		first_go = 0;

		/* Draw new grey square */
		draw_rectangle(fb, xpos, ypos, side_len, side_len, 0x0000);
		update_to_display(xpos, ypos, side_len, side_len,
			wave_mode, wait_upd_compl, 0);

		last_pos = xpos;
		xpos += increment;

		printf("xpos = %d, xpos + side_len = %d\n", xpos, xpos+side_len);
	}

	/* Clear last square (set area to white) */
	draw_rectangle(fb, last_pos, ypos, side_len, side_len, 0xFFFF);
	update_to_display(last_pos, ypos, side_len, side_len,
		WAVEFORM_MODE_DU,
		wait_upd_compl, 0);

	xpos = 700;
	ypos = 150;
	last_pos = 110;
	first_go = 1;

	while (ypos + side_len + increment < 600)
	{
		/* Clear last square (set area to white) */
		if (!first_go) {
			draw_rectangle(fb, xpos, last_pos, side_len, side_len,
					0xFFFF);
			update_to_display(xpos, last_pos, side_len,
				side_len,
				wave_mode,
				wait_upd_compl, 0);
		}
		else
			first_go = 0;

		/* Draw new black square */
		draw_rectangle(fb, xpos, ypos, side_len, side_len, 0x0000);

		/* Send to display */
		update_to_display(xpos, ypos, side_len,
			side_len + increment, wave_mode, wait_upd_compl, 0);

		last_pos = ypos;
		ypos += increment;
	}

	return retval;
}


static int test_partial_to_full(void) {
	int retval = TPASS;
	int i, j, k;
	__u16 greys[16] = {0x0000, 0x1082, 0x2104, 0x3186, 0x4408, 0x548A, 0x630C,
	0x738E, 0x8410, 0x9492, 0xA514, 0xB596, 0xC618, 0xD69A, 0xE71C, 0xF79E};
	int grey_cnt = 0;
	int left, top;
	struct mxcfb_update_data upd_data;
	struct mxcfb_update_marker_data update_marker_data;
	int ret;

	printf("Blank screen\n");
	memset(fb, 0xFF, 800*600*2);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	/* Square 1 - Dark Grey */
	draw_rectangle(fb, 50, 0, 200, 200, 0x8410);

	for (i = 0; i < 1; i++) {
		for (j = 0; j * 100 + 109 < 800; j++) {
			left = 9 + (j * 100);
			for (k = 0; k * 100 + 109 < 600; k++, grey_cnt++) {
				grey_cnt = grey_cnt % 16;
				top = 9 + (k * 100);
				draw_rectangle(fb, left, top, 100, 100, greys[grey_cnt]);
				update_to_display(left, top, 100, 100, WAVEFORM_MODE_AUTO,
					FALSE, 0);
			}
		}
	}

	sleep(2);

	upd_data.update_mode = UPDATE_MODE_FULL;
	upd_data.waveform_mode = WAVEFORM_MODE_AUTO;
	upd_data.update_region.left = 0;
	upd_data.update_region.width = screen_info.xres;
	upd_data.update_region.top = 0;
	upd_data.update_region.height = screen_info.yres;
	upd_data.temp = TEMP_USE_AMBIENT;
	upd_data.flags = 0;
	upd_data.update_marker = marker_val++;

	ret = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	while (ret < 0) {
		/* We have limited memory available for updates, so wait and
		 * then try again after some updates have completed */
		sleep(1);
		ret = ioctl(fd_fb_ioctl, MXCFB_SEND_UPDATE, &upd_data);
	}

	/* Wait for update to complete */
	update_marker_data.update_marker = upd_data.update_marker;
	ret = ioctl(fd_fb_ioctl, MXCFB_WAIT_FOR_UPDATE_COMPLETE, &update_marker_data);
	if (ret < 0)
		printf("Wait for update complete failed.  Error = 0x%x", ret);

	return retval;
}

static int test_shift(void) {
	int retval = TPASS;
	int y;

	printf("Blank screen\n");
	memset(fb, 0xFF, 800*600*2);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	/* Draw 2-pixel-wide line at 300,169 down to 300, 278 */
	for (y = 169; y < 278; y++) {
		*((__u16 *)fb + y*screen_info.xres + 300) = 0x0000;
		*((__u16 *)fb + y*screen_info.xres + 301) = 0x0000;
	}

	update_to_display(300, 169, 141, 109, WAVEFORM_MODE_DU, FALSE, 0);

	printf("Update at 300,169\n");
	sleep(5);

	update_to_display(301, 200, 141, 109, WAVEFORM_MODE_DU, FALSE, 0);

	printf("Update at 301,169\n");

	return retval;
}

static __u16 einkfb_8bpp_gray[256] =
{
	0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
	0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF,
	0xFFFF,

	/* ..0x10 */
	0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE,
	0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE, 0xEEEE,
	0xEEEE,

	/* ..0x21 */
	0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD,
	0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD, 0xDDDD,
	0xDDDD,

	/* ..0x32 */
	0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC,
	0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC, 0xCCCC,
	0xCCCC,

	/* ..0x43 */
	0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB,
	0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB, 0xBBBB,
	0xBBBB,

	/* ..0x54 */
	0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA,
	0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA, 0xAAAA,
	0xAAAA,

	/* ..0x65 */
	0x9999, 0x9999, 0x9999, 0x9999, 0x9999, 0x9999, 0x9999, 0x9999,
	0x9999, 0x9999, 0x9999, 0x9999, 0x9999, 0x9999, 0x9999, 0x9999,
	0x9999,

	/* ..0x76 */
	0x8888, 0x8888, 0x8888, 0x8888, 0x8888, 0x8888, 0x8888, 0x8888,
	0x8888, 0x8888, 0x8888, 0x8888, 0x8888, 0x8888, 0x8888, 0x8888,
	0x8888,

	/* ..0x87 */
	0x7777, 0x7777, 0x7777, 0x7777, 0x7777, 0x7777, 0x7777, 0x7777,
	0x7777, 0x7777, 0x7777, 0x7777, 0x7777, 0x7777, 0x7777, 0x7777,
	0x7777,

	/* ..0x98 */
	0x6666, 0x6666, 0x6666, 0x6666, 0x6666, 0x6666, 0x6666, 0x6666,
	0x6666, 0x6666, 0x6666, 0x6666, 0x6666, 0x6666, 0x6666, 0x6666,
	0x6666,

	/* ..0xA9 */
	0x5555, 0x5555, 0x5555, 0x5555, 0x5555, 0x5555, 0x5555, 0x5555,
	0x5555, 0x5555, 0x5555, 0x5555, 0x5555, 0x5555, 0x5555, 0x5555,
	0x5555,

	/* ..0xBA */
	0x4444, 0x4444, 0x4444, 0x4444, 0x4444, 0x4444, 0x4444, 0x4444,
	0x4444, 0x4444, 0x4444, 0x4444, 0x4444, 0x4444, 0x4444, 0x4444,
	0x4444,

	/* ..0xCB */
	0x3333, 0x3333, 0x3333, 0x3333, 0x3333, 0x3333, 0x3333, 0x3333,
	0x3333, 0x3333, 0x3333, 0x3333, 0x3333, 0x3333, 0x3333, 0x3333,
	0x3333,

	/* ..0xDC */
	0x2222, 0x2222, 0x2222, 0x2222, 0x2222, 0x2222, 0x2222, 0x2222,
	0x2222, 0x2222, 0x2222, 0x2222, 0x2222, 0x2222, 0x2222, 0x2222,
	0x2222,

	/* ..0xED */
	0x1111, 0x1111, 0x1111, 0x1111, 0x1111, 0x1111, 0x1111, 0x1111,
	0x1111, 0x1111, 0x1111, 0x1111, 0x1111, 0x1111, 0x1111, 0x1111,
	0x1111,

	/* ..0xEF */
	0x0000,
};

struct fb_cmap einkfb_8bpp_cmap =
{
	.len        = 256,
	.red        = einkfb_8bpp_gray,
	.green   = einkfb_8bpp_gray,
	.blue      = einkfb_8bpp_gray
};

static int test_colormap(void)
{
	int retval = TPASS;
	int i;

	printf("Changing to Y8 Framebuffer\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 8;
	screen_info.grayscale = GRAYSCALE_8BIT;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		printf("Go to Y8 failed\n");
		return TFAIL;
	}

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*
		screen_info.bits_per_pixel/8);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, 0);

	/* Get default colormap */
	retval = ioctl(fd_fb, FBIOGETCMAP, &einkfb_8bpp_cmap);
	if (retval < 0)
	{
		printf("Unable to get colormap from FB. ret = %d\n", retval);
		return TFAIL;
	}

	/* Create colormap */
	for (i = 0; i < 256; i++)
		einkfb_8bpp_cmap.red[i] = ~i;

	retval = ioctl(fd_fb, FBIOPUTCMAP, &einkfb_8bpp_cmap);
	if (retval < 0)
	{
		printf("Unable to set new colormap. ret = %d\n", retval);
		return TFAIL;
	}

	/* Make update request using new colormap */
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, EPDC_FLAG_USE_CMAP);
	printf("Screen should be black\n");

	sleep(2);

	/* Make update request without using new colormap */
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, 0);
	printf("Screen should be white now\n");

	sleep(2);

	/* Make update request using new colormap with inversion */
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, EPDC_FLAG_USE_CMAP | EPDC_FLAG_ENABLE_INVERSION);
	printf("Screen should be white still\n");

	sleep(2);

	/* Draw colorbar */
	draw_y_colorbar(&screen_info);

	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, EPDC_FLAG_USE_CMAP);
	printf("Should be inverted color bar (white to black, left to right)\n");

	sleep(2);

	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, 0);
	printf("Colorbar again, with no CMAP (black to white, left to right)\n");

	sleep(2);

	/* Change colormap to black-white posterize */
	for (i = 0; i < 256; i++)
		einkfb_8bpp_gray[i] = i < 128 ? 0x0000 : 0xFFFF;
	retval = ioctl(fd_fb, FBIOPUTCMAP, &einkfb_8bpp_cmap);
	if (retval < 0)
	{
		printf("Unable to set new colormap. ret = %d\n", retval);
		return TFAIL;
	}

	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_GC16, TRUE, EPDC_FLAG_USE_CMAP);
	printf("Posterized colorbar\n");

	sleep(2);

	printf("Change back to non-inverted RGB565\n");
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.bits_per_pixel = 16;
	screen_info.grayscale = 0;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)	{
		printf("Back to normal failed\n");
		return TFAIL;
	}

	return retval;
}

static int test_dry_run(void)
{
	int retval = TPASS;
	__u32 coll;

	if (scheme == UPDATE_SCHEME_SNAPSHOT) {
		printf("Unable to run collision test with SNAPSHOT scheme.\n");
		return TPASS;
	}

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*2);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	/*
	 * Draw gray rectangle
	 * Use dry run first, which should not result in collision
	 */
	draw_rectangle(fb, 0, 0, 300, 250, 0x8410);
	coll = update_to_display(0, 0, 300, 250, WAVEFORM_MODE_AUTO, FALSE, EPDC_FLAG_TEST_COLLISION);
	if (coll)
		retval = TFAIL;
	update_to_display(0, 0, 300, 250, WAVEFORM_MODE_AUTO, FALSE, 0);

	/* Draw overlapping rectangle */
	draw_rectangle(fb, 250, 200, 300, 250, 0x4104);
	update_to_display(250, 200, 300, 250, WAVEFORM_MODE_AUTO, FALSE, 0);

	sleep(3);

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*2);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	sleep(1);

	/* Draw gray rectangle */
	draw_rectangle(fb, 0, 0, 300, 250, 0x8410);
	update_to_display(0, 0, 300, 250, WAVEFORM_MODE_AUTO, FALSE, 0);

	usleep(400);

	/* Draw overlapping rectangle, use dry-run test to check for collision */
	draw_rectangle(fb, 250, 200, 300, 250, 0x4104);
	coll = update_to_display(250, 200, 300, 250, WAVEFORM_MODE_AUTO, FALSE, EPDC_FLAG_TEST_COLLISION);
	if (coll)
		sleep(1);
	else
		retval = TFAIL;
	/* Now that collision resolved, finally submit update */
	update_to_display(250, 200, 300, 250, WAVEFORM_MODE_AUTO, FALSE, 0);

	return retval;
}

static int test_stress(void)
{
	int retval = TPASS;
	int x, y, width, height, i, j;
	__u16 color;
	__u16 greys[16] = {0x0000, 0x1082, 0x2104, 0x3186, 0x4408, 0x548A, 0x630C,
	0x738E, 0x8410, 0x9492, 0xA514, 0xB596, 0xC618, 0xD69A, 0xE71C, 0xFFFF};
	uint flags;

	if (scheme == UPDATE_SCHEME_SNAPSHOT) {
		printf("Unable to run stress test with SNAPSHOT scheme.\n");
		return TPASS;
	}

	printf("Blank screen\n");
	memset(fb, 0xFF, screen_info.xres_virtual*screen_info.yres*2);
	update_to_display(0, 0, screen_info.xres, screen_info.yres,
		WAVEFORM_MODE_AUTO, TRUE, 0);

	for (i = 0; i < 200; i++) {

		screen_info.rotate = i % 4;
		screen_info.bits_per_pixel = 16;
		screen_info.grayscale = 0;
		printf("Rotating FB 90 degrees to %d\n", screen_info.rotate);
		retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
		if (retval < 0)
		{
			printf("Rotation failed\n");
			return TFAIL;
		}

		for (j = 0; j < 1000; j++) {
			width = (rand() % (screen_info.xres)) + 1;
			height = (rand() % (screen_info.yres)) + 1;
			if (width == screen_info.xres)
				x = 0;
			else
				x = rand() % (screen_info.xres - width);
			if (height == screen_info.yres)
				y = 0;
			else
				y = rand() % (screen_info.yres - height);
			color = rand() % 32;
			if (color < 16)
				color = greys[color];
			else if (color < 24)
				color = greys[0];
			else
				color = greys[15];
			/*
			printf("x = %d, y = %d, w = %d, h = %d, color = 0x%x\n",
				x, y, width, height, color);
			*/
			draw_rectangle(fb, x, y, width, height, color);
			/* 1 out of 10 updates will be a dry-run */
			if (rand() / 10 == 1)
				flags = EPDC_FLAG_TEST_COLLISION;
			else
				flags = 0;
			update_to_display(x, y, width, height,
				WAVEFORM_MODE_AUTO, FALSE, flags);
		}
	}

	return retval;
}

void usage(char *app)
{
	printf("EPDC framebuffer driver test program.\n");
	printf("Usage: mxc_epdc_fb_test [-h] [-a] [-p delay] [-u s/q/m] [-n <expression>]\n");
	printf("\t-h\t  Print this message\n");
	printf("\t-a\t  Enabled animation waveforms for fast updates (tests 8-9)\n");
	printf("\t-p\t  Provide a power down delay (in ms) for the EPDC driver\n");
	printf("\t\t  0 - Immediate (default)\n");
	printf("\t\t  -1 - Never\n");
	printf("\t\t  <ms> - After <ms> milliseconds\n");
	printf("\t-u\t  Select an update scheme\n");
	printf("\t\t  s - Snapshot update scheme\n");
	printf("\t\t  q - Queue update scheme\n");
	printf("\t\t  m - Queue and merge update scheme (default)\n");
	printf("\t-n\t  Execute the tests specified in expression\n");
	printf("\t\t  Expression is a set of comma-separated numeric ranges\n");
	printf("\t\t  If not specified, tests 1-13 are run\n");
	printf("Example:\n");
	printf("%s -n 1-3,5,7\n", app);
	printf("\nEPDC tests:\n");
	printf("1 - Basic Updates\n");
	printf("2 - Rotation Updates\n");
	printf("3 - Grayscale Framebuffer Updates\n");
	printf("4 - Auto-waveform Selection Updates\n");
	printf("5 - Panning Updates\n");
	printf("6 - Overlay Updates\n");
	printf("7 - Auto-Updates\n");
	printf("8 - Animation Mode Updates\n");
	printf("9 - Fast Updates\n");
	printf("10 - Partial to Full Update Transitions\n");
	printf("11 - Test Pixel Shifting Effect\n");
	printf("12 - Colormap Updates\n");
	printf("13 - Collision Test Mode\n");
	printf("14 - Stress Test\n");
}

int parse_test_nums(char *num_str)
{
	char * opt;
	int i, start, end;

	if (!num_str)
		return 0;

	/* Set tests to 0 and enable just the specified range */
	memset(test_map, 0, sizeof(test_map));

	while ((opt = strsep(&num_str, ",")) != NULL) {
		if (!*opt)
			continue;

		printf("opt = %s\n", opt);

		start = atoi(opt);
		if ((start <= 0) || (start > NUM_TESTS)) {
			printf("Invalid test number(s) specified\n");
			return -1;
		}

		if (opt[1] == '-') {
			end = atoi(opt+2);

			if ((end < start) || (end > NUM_TESTS)) {
				printf("Invalid test number(s) specified\n");
				return -1;
			}

			for (i = start; i <= end; i++)
				test_map[i-1] = TRUE;
		} else {
			test_map[start-1] = TRUE;
		}
	}

	return 0;
}

int
main(int argc, char **argv)
{
	int retval = TPASS;
	int auto_update_mode;
	struct mxcfb_waveform_modes wv_modes;
	char fb_dev[10] = "/dev/fb";
	int fb_num = 0;
	struct fb_fix_screeninfo screen_info_fix;

	int i, rt;

	/* Initialize test map so all tests (except stress test) will run */
	for(i = 0; i < NUM_TESTS - 1; i++)
		test_map[i] = TRUE;

	while ((rt = getopt(argc, argv, "hau:n:p:f:")) >= 0) {
		switch (rt) {
		case 'h':
			usage(argv[0]);
			return TPASS;
		case 'a':
			use_animation = 1;
			break;
		case 'f':
			num_flashes = atoi(optarg);
			printf("number of flashes = %d\n", num_flashes);
			break;
		case 'u':
			if (strcmp(optarg, "s") == 0) {
				scheme = UPDATE_SCHEME_SNAPSHOT;
			} else if (strcmp(optarg, "q") == 0) {
				scheme = UPDATE_SCHEME_QUEUE;
			} else if (strcmp(optarg, "m") == 0) {
				scheme = UPDATE_SCHEME_QUEUE_AND_MERGE;
			} else {
				printf("%s: -d Requires either an "
					"'s', 'q', or 'm' argument\n", argv[0]);
				usage(argv[0]);
			}
			break;
		case 'n':
			if (parse_test_nums(optarg) < 0) {
				usage(argv[0]);
				return TFAIL;
			}
			break;
		case 'p':
			pwrdown_delay = atoi(optarg);
			printf("powerdown delay %d\n", pwrdown_delay);
			break;
		default:
			usage(argv[0]);
			break;
		}
	}

	/* Find EPDC FB device */
	while (1) {
		fb_dev[7] = '0' + fb_num;
		fd_fb = open(fb_dev, O_RDWR, 0);
		if (fd_fb < 0) {
			printf("Unable to open %s\n", fb_dev);
			retval = TFAIL;
			goto err0;
		}

		/* Check that fb device is EPDC */
		/* First get screen_info */
		retval = ioctl(fd_fb, FBIOGET_FSCREENINFO, &screen_info_fix);
		if (retval < 0)
		{
			printf("Unable to read fixed screeninfo for %s\n", fb_dev);
			goto err1;
		}

		/* If we found EPDC, exit loop */
		if (!strcmp(EPDC_STR_ID, screen_info_fix.id)) {
			printf("Opened EPDC fb device %s\n", fb_dev);
			break;
		}

		fb_num++;
	}

	/*
	 * If kernel test driver exists, we default
	 * to using it for EPDC ioctls
	 */
	fd_fb_ioctl = open("/dev/epdc_test", O_RDWR, 0);
	if (fd_fb_ioctl >= 0)
		printf("\n****Using EPDC kernel module test driver!****\n\n");
	else
		fd_fb_ioctl = fd_fb;

	retval = ioctl(fd_fb, FBIOGET_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		goto err1;
	}
	printf("Set the background to 16-bpp\n");
	screen_info.bits_per_pixel = 16;
	screen_info.grayscale = 0;
	screen_info.yoffset = 0;
	screen_info.rotate = FB_ROTATE_UR;
	screen_info.activate = FB_ACTIVATE_FORCE;
	retval = ioctl(fd_fb, FBIOPUT_VSCREENINFO, &screen_info);
	if (retval < 0)
	{
		goto err1;
	}
	g_fb_size = screen_info.xres_virtual * screen_info.yres_virtual * screen_info.bits_per_pixel / 8;

	printf("screen_info.xres_virtual = %d\nscreen_info.yres_virtual = %d\nscreen_info.bits_per_pixel = %d\n",
		screen_info.xres_virtual, screen_info.yres_virtual, screen_info.bits_per_pixel);

	printf("Mem-Mapping FB\n");

	/* Map the device to memory*/
	fb = (unsigned short *)mmap(0, g_fb_size,PROT_READ | PROT_WRITE, MAP_SHARED, fd_fb, 0);
	if ((int)fb <= 0)
	{
		printf("\nError: failed to map framebuffer device 0 to memory.\n");
		goto err1;
	}

	printf("Set to region update mode\n");
	auto_update_mode = AUTO_UPDATE_MODE_REGION_MODE;
	retval = ioctl(fd_fb_ioctl, MXCFB_SET_AUTO_UPDATE_MODE, &auto_update_mode);
	if (retval < 0)
	{
		printf("\nError: failed to set update mode.\n");
		goto err2;
	}

	printf("Set waveform modes\n");
	wv_modes.mode_init = WAVEFORM_MODE_INIT;
	wv_modes.mode_du = WAVEFORM_MODE_DU;
	wv_modes.mode_gc4 = WAVEFORM_MODE_GC4;
	wv_modes.mode_gc8 = WAVEFORM_MODE_GC16;
	wv_modes.mode_gc16 = WAVEFORM_MODE_GC16;
	wv_modes.mode_gc32 = WAVEFORM_MODE_GC16;
	retval = ioctl(fd_fb_ioctl, MXCFB_SET_WAVEFORM_MODES, &wv_modes);
	if (retval < 0)
	{
		printf("\nError: failed to set waveform mode.\n");
		goto err2;
	}

	printf("Set update scheme - %d\n", scheme);
	retval = ioctl(fd_fb_ioctl, MXCFB_SET_UPDATE_SCHEME, &scheme);
	if (retval < 0)
	{
		printf("\nError: failed to set update scheme.\n");
		goto err2;
	}

	printf("Set pwrdown_delay - %d\n", pwrdown_delay);
	retval = ioctl(fd_fb_ioctl, MXCFB_SET_PWRDOWN_DELAY, &pwrdown_delay);
	if (retval < 0)
	{
		printf("\nError: failed to set power down delay.\n");
		goto err2;
	}

	testfunc_array[0] = &test_updates;
	testfunc_array[1] = &test_rotation;
	testfunc_array[2] = &test_y8;
	testfunc_array[3] = &test_auto_waveform;
	testfunc_array[4] = &test_pan;
	testfunc_array[5] = &test_overlay;
	testfunc_array[6] = &test_auto_update;
	testfunc_array[7] = &test_animation_mode;
	testfunc_array[8] = &test_fast_square;
	testfunc_array[9] = &test_partial_to_full;
	testfunc_array[10] = &test_shift;
	testfunc_array[11] = &test_colormap;
	testfunc_array[12] = &test_dry_run;
	testfunc_array[13] = &test_stress;

	for (i = 0; i < NUM_TESTS; i++)
		if (test_map[i])
			if ((*testfunc_array[i])()) {
				printf("\nError: Test #%d failed.\n", i + 1);
				goto err2;
			}

err2:
	munmap(fb, g_fb_size);
err1:
	close(fd_fb);
	if (fd_fb != fd_fb_ioctl)
		close(fd_fb_ioctl);
err0:
	return retval;
}
