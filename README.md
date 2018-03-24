### libremarkable -- Application Development Framework for Remarkable Tablet [![https://crates.io/crates/libremarkable](https://img.shields.io/crates/v/libremarkable.svg?style=for-the-badge)](https://crates.io/crates/libremarkable)

[![RustPoC](https://i.imgur.com/xw8glBk.jpg)](https://i.imgur.com/xw8glBk.jpg)
[![PoC](https://thumbs.gfycat.com/WelltodoImprobableAlligator-size_restricted.gif)](https://gfycat.com/gifs/detail/WelltodoImprobableAlligator)

Everything from low latency partial updates to the eInk display to at least minimal multitouch, physical button and Wacom Digitizer input is now understood and their minimal to complete implementations can be found in this repository.

The focus of this repository is now going to be the Rust library for providing these features. Potentially a `piston` backend might be created for `Remarkable`, allowing the use of `conrod` to simplify UI creation.

In cases where Rust implementation seems to contradict with the C implementation, the former can be taken as the source of truth as the `libremarkable` C implementation was the first-pass that came to being during the exploration stage.

For further documentation see the [Knowledge Base](https://github.com/canselcik/libremarkable/wiki/The-Undocumented-Remarkable-Low-Latency-I-O).

### Build Instructions

#### Setting up the toolchain
In order to build `libremarkable` and the examples (`spy.so` and `demo`), you'll need the following configuration after having installed the proper toolchain to your `$PATH`. The `arm-linux-gnueabihf-gcc` toolchain is used to build both implementations.

The toolchain that would be acquired from either of these sources would be able to cross-compile for the Remarkable Tablet:
```
AUR:
  https://aur.archlinux.org/packages/arm-linux-gnueabihf-gcc/
Remarkable:
  https://remarkable.engineering/deploy/sdk/poky-glibc-x86_64-meta-toolchain-qt5-cortexa9hf-neon-toolchain-2.1.3.sh
```

You can then set up your Rust toolchain for cross compilation with: `rustup target add armv7-unknown-linux-gnueabihf`.

Once that's done, you should add the following to your `~/.cargo/config`:
```
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
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
