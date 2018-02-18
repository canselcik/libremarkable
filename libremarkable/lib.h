#pragma once
#include <stdint.h>
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdlib.h>

#define REMARKABLE_PREFIX                       0x40480000
#define MXCFB_SEND_UPDATE                       0x0000462e

// Untested
#define MXCFB_WAIT_FOR_VSYNC                    0x00004620
#define MXCFB_SET_GBL_ALPHA                     0x00004621
#define MXCFB_SET_CLR_KEY                       0x00004622
#define MXCFB_SET_OVERLAY_POS                   0x00004624
#define MXCFB_GET_FB_IPU_CHAN                   0x00004625
#define MXCFB_SET_LOC_ALPHA                     0x00004626
#define MXCFB_SET_LOC_ALP_BUF                   0x00004627
#define MXCFB_SET_GAMMA                         0x00004628
#define MXCFB_GET_FB_IPU_DI                     0x00004629
#define MXCFB_GET_DIFMT                         0x0000462a
#define MXCFB_GET_FB_BLANK                      0x0000462b
#define MXCFB_SET_WAVEFORM_MODES                0x0000462b
#define MXCFB_SET_DIFMT                         0x0000462c
#define MXCFB_SET_TEMPERATURE                   0x0000462c
#define MXCFB_SET_AUTO_UPDATE_MODE              0x0000462d
#define MXCFB_WAIT_FOR_UPDATE_COMPLETE          0x0000462f
#define MXCFB_SET_PWRDOWN_DELAY                 0x00004630
#define MXCFB_GET_PWRDOWN_DELAY                 0x00004631
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

void print_mxcfb_update_data(mxcfb_update_data* x);