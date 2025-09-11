

## [![https://crates.io/crates/libremarkable](https://img.shields.io/crates/v/libremarkable.svg?style=for-the-badge)](https://crates.io/crates/libremarkable) libremarkable - A Framework for Remarkable Tablet

[![PoC](https://thumbs.gfycat.com/ScholarlyShadyElk-size_restricted.gif)](https://gfycat.com/ScholarlyShadyElk)

[![color](https://github.com/canselcik/libremarkable/raw/master/reference-material/color.jpg)](https://github.com/canselcik/libremarkable/raw/master/reference-material/color.jpg)

Everything from low latency partial updates to the eInk display to multitouch, physical button and Wacom Digitizer input is now understood and their minimal to complete implementations can be found in this repository.

This repository implements a Rust library for providing these features.
Potentially a `piston` backend might be created for `Remarkable`, allowing the use of `conrod` to simplify UI creation.
For further documentation see the [wiki](https://github.com/canselcik/libremarkable/wiki) on this repository.

`https://github.com/canselcik/RemarkableFramebuffer` redirects to this repository for historical purposes.

### Build Instructions

#### Setting up the toolchain
In order to build `libremarkable` and the examples (`spy.so` and `demo`), you'll need the toolchain from Remarkable. Download the [installation script](https://storage.googleapis.com/remarkable-codex-toolchain/codex-x86_64-cortexa9hf-neon-rm10x-toolchain-3.1.2.sh) ([rM2](https://storage.googleapis.com/remarkable-codex-toolchain/codex-x86_64-cortexa7hf-neon-rm11x-toolchain-3.1.2.sh)) and install the toolchain. You can find more information on the [wiki](https://web.archive.org/web/20230616024159/https://remarkablewiki.com/devel/toolchain).

You can then set up your Rust toolchain for cross compilation with: `rustup target add armv7-unknown-linux-gnueabihf`.

In order for rust to leverage the toolchain a `.cargo/config` file is required. This file can be generated using `gen_cargo_config.py`. First the toolchain environment must be
sourced. Its location is can be found within the toolchain installation directory. The correct path is also referenced in the toolchain [wiki](https://web.archive.org/web/20230616024159/https://remarkablewiki.com/devel/toolchain).
After the environment is loaded the script will read the environment variables to generate the correct `.cargo/config` file for your toolchain.

The resulting config file will look something like this:
```
[target.armv7-unknown-linux-gnueabihf]
linker = "<toolchain_install_path>/sysroots/x86_64-codexsdk-linux/usr/bin/arm-remarkable-linux-gnueabi/arm-remarkable-linux-gnueabi-gcc"
rustflags = [
  "-C", "link-arg=-march=armv7-a",
  "-C", "link-arg=-marm",
  "-C", "link-arg=-mfpu=neon",
  "-C", "link-arg=-mfloat-abi=hard",
  "-C", "link-arg=-mcpu=cortex-a9",
  "-C", "link-arg=--sysroot=<toolchain_install_path>/sysroots/cortexa7hf-neon-remarkable-linux-gnueabi",
]
```

You can also add this snippet to the above file in order to default to cross-compiling for this project:

```
[build]
# Set the default --target flag
target = "armv7-unknown-linux-gnueabihf"
```

### MSRV

Since libremarkable 0.7.0, the minimum supported rust version (MSRV) is [**1.80**](https://releases.rs/docs/1.80.0/).

#### Building libremarkable and the examples
A simple Makefile wrapper is created for convenience. It exposes the following verbs:
  - `examples`: Builds examples
  - `library`: Builds library
  - `all`: library + examples

#### Testing libremarkable and the examples on the device
The provided `Makefile` assumes the device is reachable at `10.11.99.1` and that SSH Key-Based Authentication is set up for SSH so that you won't be prompted a password every time. The following actions are available:
  - `run`: Builds and runs `demo.rs` on the device after stopping `xochitl`
  - `start-xochitl`: Stops all `xochitl` and `demo` instances and starts `xochitl` normally
  - `spy-xochitl`: Builds `spy.rs` and `LD_PRELOAD`s it to a new instance of `xochitl` after
                   stopping the current instance. This allows discovery of new enums used by
                   official programs in calls to `ioctl`.

#### Further build instructions for manual builds
If you choose to skip the `Makefile` and call `cargo` yourself, make sure to include `--release --target=armv7-unknown-linux-gnueabihf` in your arguments like:
```
➜  rust-poc git:(master) ✗ cargo build --release --target=armv7-unknown-linux-gnueabihf
   ...
   Compiling libremarkable v0.1.0 (file:///home/main/Desktop/libremarkable)
   Compiling rust-poc v0.1.0 (file:///home/main/Desktop/RemarkableFramebuffer/rust-poc)
    Finished dev [unoptimized + debuginfo] target(s) in 24.85 secs
```
The `--release` argument is important as this enables optimizations and without optimizations you'll be looking at ~70% CPU utilization even when idle. With optimizations, the framework runs really light, 0% CPU utilization when idle and 1-2% at peak.

#### Building with [`cross`](https://github.com/rust-embedded/cross)
*Building this way does not require reMarkable's toolchain nor building on Ubuntu 16.04 with Docker so setting up should be easier.*

Install `cross` with `cargo install cross`. Make sure the reMarkable toolchain is not in use first.

To build, deploy and run the `demo`, simply:
```shell
make TARGET=armv7-unknown-linux-gnueabihf deploy-x-demo
# This builds with
#   cross build --example demo --release --target=armv7-unknown-linux-gnueabihf
# then deploys the demo
```
##### Using [`musl`](https://musl.libc.org/)
1. Compile with `cross build --example demo --release --target=armv7-unknown-linux-musleabihf` (or `make x-demo`)
1. Run the demo: `make deploy-x-demo`

**Regarding apps for the rM2**: you will need the [display](https://github.com/ddvk/remarkable2-framebuffer) package from [Toltec](https://toltec-dev.org/). Only the server part though as the client is built into this lib.

### Legacy C implementation

The first draft of `libremarkable` was a C library, built while reverse engineering the tablet.
It's no longer maintained, but can be found on the [`legacy-c-impl`](https://github.com/canselcik/libremarkable/tree/legacy-c-impl) branch.
