#include <ft2build.h>
#include "lib.h"
#include <freetype/ftglyph.h>
#include FT_FREETYPE_H

// TODO: Optimizations. A lot of low hanging fruits here. Precomputing or caching glyphs, initializing the font once, 
// rendering the text in the end etc.
mxcfb_rect remarkable_framebuffer_draw_text(remarkable_framebuffer* fb, const char* fontFilename, 
                                            const char* text, unsigned top,
                                            unsigned left, int target_height) {
  FT_Library library;
  FT_Error error;
  error = FT_Init_FreeType(&library);
  if (error != 0) {
      printf("Failed to init freetype\n");
      exit(1);
  }

  FT_Face face;
  error = FT_New_Face(library, fontFilename, 0, &face);
  if (error != 0) {
      printf("Failed to init typeface\n");
      exit(1);
  }

  error = FT_Set_Char_Size(face, target_height * 64, 0, fb->vinfo.xres, fb->vinfo.yres);
  if (error != 0) {
      printf("Failed to set character size\n");
      exit(1);
  }

  FT_GlyphSlot slot = face->glyph;
  FT_Vector pen;
  pen.x = left;
  pen.y = top;

  error = FT_Load_Char(face, 'T', FT_LOAD_RENDER);
  if (error != 0) {
    printf("Failed to render 'T' for glyph approximation.\n");
    exit(1);
  }
  unsigned tOffsetY = face->glyph->bitmap_top;
  unsigned tOffsetX = face->glyph->bitmap_left;
  unsigned len = strlen(text);
  for (unsigned n = 0; n < len; n++) {
    // The old char face is overwritten
    error = FT_Load_Char(face, text[n], FT_LOAD_RENDER);
    if (error)
      continue;
      
    unsigned offsetY = (tOffsetY - face->glyph->bitmap_top);
    pen.y += offsetY;
    for (FT_Int i = pen.x, p = 0; i < pen.x + slot->bitmap.width; i++, p++)
      for (FT_Int j = pen.y, q = 0; j < pen.y + slot->bitmap.rows; j++, q++)
        remarkable_framebuffer_set_pixel(fb,
                                         j, i,
                                         slot->bitmap.buffer[q * slot->bitmap.width + p] == 0 ? REMARKABLE_BRIGHTEST
                                                                                              : REMARKABLE_DARKEST);
    pen.y -= offsetY;

    pen.x += slot->advance.x >> 6;
    pen.y += slot->advance.y >> 6;
  }

  FT_Done_Face(face);
  FT_Done_FreeType(library);

  mxcfb_rect updated;
  updated.top = top;
  updated.left = left;
  updated.height = target_height * 64;
  updated.width = pen.x - left;
  return updated;
}