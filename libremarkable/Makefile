CC = arm-linux-gnueabihf-gcc
AR = arm-linux-gnueabihf-ar
SOURCES = fb.c refresh.c serde.c bitmap.c
OBJECTS = $(SOURCES:.c=.o)
CCFLAGS = -fPIC -g
LDFLAGS =

all: static shared

static: $(OBJECTS)
	$(CC) -shared $(CCFLAGS) $(LDFLAGS) $(OBJECTS) -o libremarkable.so 

shared: $(OBJECTS)
	$(AR) -rsc libremarkable.a $(OBJECTS)

%.o: %.c
	$(CC) -c $(CCFLAGS) $< -o $@

clean:
	rm -rvf *.o || true
	rm -rvf *.so || true
	rm -rvf *.a || true