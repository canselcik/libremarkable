all: library examples

.PHONY: examples
examples:
	cargo build --examples --release --target=armv7-unknown-linux-gnueabihf

bench:
	cargo build --examples --release --target=armv7-unknown-linux-gnueabihf --features "enable-runtime-benchmarking"

library:
	cargo build --release --target=armv7-unknown-linux-gnueabihf

test:
	# Notice we aren't using the armv7 target here
	cargo test

DEVICE_IP ?= "10.11.99.1"
run: examples
	ssh root@$(DEVICE_IP) 'kill -9 `pidof demo` || true; systemctl stop xochitl || true'
	scp ./target/armv7-unknown-linux-gnueabihf/release/examples/demo root@$(DEVICE_IP):~/
	ssh root@$(DEVICE_IP) './demo'

live: examples
	ssh root@$(DEVICE_IP) 'kill -9 `pidof live` || true'
	scp ./target/armv7-unknown-linux-gnueabihf/release/examples/live root@$(DEVICE_IP):~/
	ssh root@$(DEVICE_IP) './live'

run-bench: bench
	ssh root@$(DEVICE_IP) 'kill -9 `pidof demo` || true; systemctl stop xochitl || true'
	scp ./target/armv7-unknown-linux-gnueabihf/release/examples/demo root@$(DEVICE_IP):~/
	ssh root@$(DEVICE_IP) './demo'

spy-xochitl: examples
	ssh root@$(DEVICE_IP) 'systemctl stop xochitl'
	scp ./target/armv7-unknown-linux-gnueabihf/release/examples/libspy.so root@$(DEVICE_IP):~/
	ssh root@$(DEVICE_IP) 'LD_PRELOAD="/home/root/libspy.so" xochitl'

start-xochitl:
	ssh root@$(DEVICE_IP) 'kill -9 `pidof demo` || true; systemctl start xochitl'
	
