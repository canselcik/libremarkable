

## [![https://crates.io/crates/libremarkable](https://img.shields.io/crates/v/libremarkable.svg?style=for-the-badge)](https://crates.io/crates/libremarkable) libremarkable - A Framework for Remarkable Tablet

[![PoC](https://thumbs.gfycat.com/ScholarlyShadyElk-size_restricted.gif)](https://gfycat.com/ScholarlyShadyElk)

[![color](https://github.com/canselcik/libremarkable/raw/master/reference-material/color.jpg)](https://github.com/canselcik/libremarkable/raw/master/reference-material/color.jpg)

Everything from low latency partial updates to the eInk display to multitouch, physical button and Wacom Digitizer input is now understood and their minimal to complete implementations can be found in this repository.

The focus of this repository is now going to be the Rust library for providing these features. Potentially a `piston` backend might be created for `Remarkable`, allowing the use of `conrod` to simplify UI creation.

In cases where Rust implementation seems to contradict with the C implementation, the former can be taken as the source of truth as the `libremarkable` C implementation was the first-pass that came to being during the exploration stage.

For further documentation see the [wiki](https://github.com/canselcik/libremarkable/wiki) on this repository.

`https://github.com/canselcik/RemarkableFramebuffer` redirects to this repository for historical purposes.

### Build Instructions

#### Setting up the toolchain
In order to build `libremarkable` and the examples (`spy.so` and `demo`), you'll need the toolchain from Remarkable. Download the [installation script](https://storage.googleapis.com/remarkable-codex-toolchain/codex-x86_64-cortexa9hf-neon-rm10x-toolchain-3.1.2.sh) ([rM2](https://storage.googleapis.com/remarkable-codex-toolchain/codex-x86_64-cortexa7hf-neon-rm11x-toolchain-3.1.2.sh)) and install the toolchain. You can find more information on the [wiki](https://remarkablewiki.com/devel/toolchain).

You can then set up your Rust toolchain for cross compilation with: `rustup target add armv7-unknown-linux-gnueabihf`.

Once that's done, you should add the following to `.cargo/config` (replace `<path-to-installed-oecore-toochain>` with the directory you installed the Remarkable toolchain to):
```
[target.armv7-unknown-linux-gnueabihf]
linker = "<path-to-the-installed-oecore-toolchain>/sysroots/x86_64-oesdk-linux/usr/bin/arm-oe-linux-gnueabi/arm-oe-linux-gnueabi-gcc"
rustflags = [
  "-C", "link-arg=-march=armv7-a",
  "-C", "link-arg=-marm",
  "-C", "link-arg=-mfpu=neon",
  "-C", "link-arg=-mfloat-abi=hard",
  "-C", "link-arg=-mcpu=cortex-a9",
  "-C", "link-arg=--sysroot=<path-to-the-installed-oecore-toolchain>/sysroots/cortexa9hf-neon-oe-linux-gnueabi",
]
```

(`<path-to-the-installed-oecore-toolchain` will likely be `/usr/local/oecore-x86_64/`, if you did the default install on Linux.)

If you have further questions, feel free to ask in Issues.

You can also add this snippet to the above file in order to default to cross-compiling for this project:

```
[build]
# Set the default --target flag
target = "armv7-unknown-linux-gnueabihf"
```

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
