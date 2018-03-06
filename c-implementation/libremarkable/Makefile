CC = arm-linux-gnueabihf-gcc
AR = arm-linux-gnueabihf-ar
SOURCES = fb.c refresh.c serde.c bitmap.c freetype.c
OBJECTS = $(SOURCES:.c=.o)
FREETYPE_PATH = ../libfreetype/install
CCFLAGS = -fPIC -g -I$(FREETYPE_PATH)/usr/local/include -I$(FREETYPE_PATH)/usr/local/include/freetype2
LDFLAGS = $(FREETYPE_PATH)/usr/local/lib/libfreetype.a

all: static shared

static: $(OBJECTS)
	$(CC) -shared $(CCFLAGS) $(OBJECTS) $(LDFLAGS) -o libremarkable.so 

shared: $(OBJECTS)
	$(AR) -rsc libremarkable.a $(OBJECTS) $(LDFLAGS)

%.o: %.c
	$(CC) -c $(CCFLAGS) $< -o $@

clean:
	rm -rvf *.o || true
	rm -rvf *.so || true
	rm -rvf *.a || true