CC=arm-linux-gnueabihf-gcc

remarkableFramebufferSpy:
	$(CC) -g -fPIC -rdynamic -c remarkableFramebufferSpy.c
	$(CC) -g -shared -nostartfiles -o remarkableFramebufferSpy.so remarkableFramebufferSpy.o -ldl

clean:
	rm -rf *.o
	rm -rf *.so
