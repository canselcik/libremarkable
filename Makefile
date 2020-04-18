TARGET ?= armv7-unknown-linux-gnueabihf

all: library examples

.PHONY: examples
examples:
	cargo build --examples --release --target=$(TARGET)

bench:
	cargo build --examples --release --target=$(TARGET) --features "enable-runtime-benchmarking"

.PHONY: docker-env
docker-env:
	cd docker-toolchain && docker build \
		--build-arg UNAME=builder \
		--build-arg UID=$(shell id -u) \
		--build-arg GID=$(shell id -g) \
		--build-arg ostype=${shell uname} \
		--tag rust-build-remarkable:latest .

examples-docker: docker-env
	docker volume create cargo-registry
	docker run \
		--rm \
		--user builder \
		-v $(shell pwd):/home/builder/libremarkable:rw \
		-v cargo-registry:/home/builder/.cargo/registry \
		-w /home/builder/libremarkable \
		rust-build-remarkable:latest \
		cargo build --examples --release --target=$(TARGET)

library:
	cargo build --release --target=$(TARGET)

test:
	# Notice we aren't using the armv7 target here
	cargo test

DEVICE_IP ?= '10.11.99.1'
DEVICE_HOST ?= root@$(DEVICE_IP)
deploy-demo:
	ssh $(DEVICE_HOST) 'killall -q -9 demo || true; systemctl stop xochitl || true'
	scp ./target/$(TARGET)/release/examples/demo $(DEVICE_HOST):~/
	ssh $(DEVICE_HOST) './demo'
run-demo:
	ssh $(DEVICE_HOST) 'killall -q -9 demo || true; systemctl stop xochitl || true'
	ssh $(DEVICE_HOST) 'RUST_LOG=info ./demo'

run: examples deploy-demo

run-docker: examples-docker deploy-demo

live: examples
	ssh $(DEVICE_HOST) 'killall -q -9 live || true'
	scp ./target/$(TARGET)/release/examples/live $(DEVICE_HOST):~/
	ssh $(DEVICE_HOST) './live'

run-bench: bench
	ssh $(DEVICE_HOST) 'killall -q -9 demo || true; systemctl stop xochitl || true'
	scp ./target/$(TARGET)/release/examples/demo $(DEVICE_HOST):~/
	ssh $(DEVICE_HOST) './demo'

spy-xochitl: examples
	ssh $(DEVICE_HOST) 'systemctl stop xochitl'
	scp ./target/$(TARGET)/release/examples/libspy.so $(DEVICE_HOST):~/
	ssh $(DEVICE_HOST) 'LD_PRELOAD="/home/root/libspy.so" xochitl'

start-xochitl:
	ssh $(DEVICE_HOST) 'killall -q -9 demo || true; systemctl start xochitl'
