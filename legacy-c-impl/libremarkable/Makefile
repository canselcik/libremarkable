CC = arm-linux-gnueabihf-gcc
AR = arm-linux-gnueabihf-ar
CFLAGS += -fPIC -g
CFLAGS += -I$(FREETYPE_PATH)/usr/local/include
CFLAGS += -I$(FREETYPE_PATH)/usr/local/include/freetype2
CFLAGS += -std=c99
LDFLAGS = $(FREETYPE_PATH)/usr/local/lib/libfreetype.a

CONFIG?=config.mk

include $(CONFIG)

SOURCES = fb.c refresh.c serde.c bitmap.c freetype.c
OBJECTS = $(SOURCES:.c=.o)
FREETYPE_PATH = ../libfreetype/install

all: static shared

shared: $(OBJECTS)
	$(CC) -shared $(CCFLAGS) $(OBJECTS) $(LDFLAGS) -o libremarkable.so

static: $(OBJECTS)
	$(AR) -rsc libremarkable.a $(OBJECTS) $(LDFLAGS)

%.o: %.c
	$(CC) -c $(CFLAGS) $< -o $@

clean:
	$(RM) *.o
	$(RM) *.so
	$(RM) *.a
