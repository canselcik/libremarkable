#include <ft2build.h>
#include "lib.h"
#include <freetype/ftglyph.h>
#include FT_FREETYPE_H

#define FREETYPE_RIGHTSHIFT 6

struct remarkable_font {
  FT_Library library;
  FT_Face face;
  unsigned tOffsetX, tOffsetY;
  unsigned target_height;
};

struct remarkable_font* remarkable_framebuffer_font_init(remarkable_framebuffer* fb, const char* fontFilename, unsigned target_height) {
  if (fontFilename == NULL)
    return NULL;

  // Zero init
  struct remarkable_font* ptr = (struct remarkable_font*)malloc(sizeof(struct remarkable_font));
  memset(ptr, 0, sizeof(struct remarkable_font));

  FT_Error error;
  error = FT_Init_FreeType(&ptr->library);
  if (error != 0) {
    printf("Failed to init freetype\n");
    free(ptr);
    return NULL;
  }

  error = FT_New_Face(ptr->library, fontFilename, 0, &ptr->face);
  if (error != 0) {
    printf("Failed to init typeface\n");
    free(ptr);
    return NULL;
  }

  error = FT_Set_Char_Size(ptr->face, target_height, 0, XRES(fb), YRES(fb));
  if (error != 0) {
    printf("Failed to set character size\n");
    free(ptr);
    return NULL;
  }

  FT_GlyphSlot slot = ptr->face->glyph;
  error = FT_Load_Char(ptr->face, 'T', FT_LOAD_RENDER);
  if (error != 0) {
    printf("Failed to render 'T' for glyph approximation.\n");
    free(ptr);
    return NULL;
  }

  ptr->tOffsetY = slot->bitmap_top;
  ptr->tOffsetX = slot->bitmap_left;
  ptr->target_height = target_height;
  return ptr;
}

void remarkable_framebuffer_font_destroy(struct remarkable_font* font) {
  if (font == NULL)
    return;

  if (font->face != NULL) {
    FT_Done_Face(font->face);
    font->face = NULL;
  }
  if (font->library != NULL) {
    FT_Done_FreeType(font->library);
    font->library = NULL;
  }

  free(font);
}

// TODO: Optimizations. A lot of low hanging fruits here. Precomputing or caching glyphs, rendering the text in the end etc.
mxcfb_rect remarkable_framebuffer_draw_text(remarkable_framebuffer* fb,
                                            struct remarkable_font* font,
                                            const char* text,
                                            unsigned top, unsigned left) {
  mxcfb_rect rect = {0};
  if (fb == NULL || font == NULL)
    return rect;

  FT_Vector pen;
  pen.x = left;
  pen.y = top;

  FT_Error error;
  FT_GlyphSlot slot;
  unsigned len = strlen(text);
  for (unsigned n = 0; n < len; n++) {
    // The old char face is overwritten
    error = FT_Load_Char(font->face, text[n], FT_LOAD_RENDER);
    if (error)
      continue;
      
    slot = font->face->glyph;
    unsigned offsetY = (font->tOffsetY - slot->bitmap_top);
    pen.y += offsetY;
    for (FT_Int i = pen.x, p = 0; i < pen.x + slot->bitmap.width; i++, p++) {
      for (FT_Int j = pen.y, q = 0; j < pen.y + slot->bitmap.rows; j++, q++) {
        remarkable_framebuffer_set_pixel(fb,
                                         j, i,
                                         slot->bitmap.buffer[q * slot->bitmap.width + p] == 0 ? REMARKABLE_BRIGHTEST
                                                                                              : REMARKABLE_DARKEST);
      }
    }
    pen.y -= offsetY;

    pen.x += slot->advance.x >> FREETYPE_RIGHTSHIFT;
    pen.y += slot->advance.y >> FREETYPE_RIGHTSHIFT;
  }

  rect.top = top;
  rect.left = left;
  rect.height = font->target_height;
  rect.width = pen.x - left;
  return rect;
}
