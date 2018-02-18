CC = arm-linux-gnueabihf-gcc
AR = arm-linux-gnueabihf-ar

CCFLAGS = -fPIC -g

all: poc spy.so

libremarkable/%:
	make -C libremarkable

poc: libremarkable/libremarkable.a
	$(CC) -c poc.c -o poc.o
	$(CC) poc.o libremarkable/libremarkable.a -o poc

spy.so: libremarkable/libremarkable.a
	$(CC) -c spy.c -o spy.o
	$(CC) -shared spy.o libremarkable/libremarkable.a -ldl -o spy.so

clean:
	find . -name \*.so -delete
	find . -name \*.o -delete
	find . -name \*.a -delete
	rm -rvf poc || true
