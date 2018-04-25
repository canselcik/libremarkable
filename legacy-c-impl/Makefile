CC = arm-linux-gnueabihf-gcc
CXX = arm-linux-gnueabihf-g++
AR = arm-linux-gnueabihf-ar
HOST=arm-linux-genueabihf

CONFIG?=

include $(CONFIG)

CFLAGS ?= -fPIC -g -static

all: poc spy.so evtest

libremarkable/%:
	make -C libremarkable

FREETYPE_PATH = ./libfreetype/install
freetype:
	cd libfreetype && CXX=$(CXX) CC=$(CC) ./configure --host=$(HOST) --without-zlib --without-png --enable-static=yes --enable-shared=no             
	cd libfreetype && make -j4
	cd libfreetype && DESTDIR=$(shell readlink -f libfreetype/install) make install

evtest:
	$(CC) $(CFLAGS) evtest.c -o evtest

poc: libremarkable/libremarkable.a
	$(CXX) $(CFLAGS) -c poc.cc -o poc.o -I$(FREETYPE_PATH)/usr/local/include -I$(FREETYPE_PATH)/usr/local/include/freetype2
	$(CXX) $(CFLAGS) poc.o libremarkable/libremarkable.a $(FREETYPE_PATH)/usr/local/lib/libfreetype.a -o poc 

spy.so: libremarkable/libremarkable.a
	$(CC) $(CFLAGS) -c spy.c -o spy.o
	$(CC) $(CFLAGS) -shared spy.o libremarkable/libremarkable.a -ldl -o spy.so

clean:
	$(RM) *.so
	$(RM) *.o
	$(RM) poc
	$(RM) evtest
	make -C libremarkable clean
