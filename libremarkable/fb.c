#include "lib.h"

void print_mxcfb_update_data(mxcfb_update_data* x) {
  printf("{\n"
         "   updateRegion: x: %u\n"
         "                 y: %u\n"
         "                 width: %u\n"
         "                 height: %u\n"
         "   waveformMode: %u,\n"
         "   updateMode:   %u\n"
         "   updateMarker: %u\n"
         "   temp: %d\n"
         "   flags: 0x%04x\n"
         "   alt_buffer_data: %p\n"
         "}",
         x->update_region.top,
         x->update_region.left,
         x->update_region.width,
         x->update_region.height,
         x->waveform_mode,
         x->update_mode,
         x->update_marker,
         x->temp,
         x->flags,
         x->alt_buffer_data);
}