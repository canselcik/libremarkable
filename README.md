### libremarkable -- Application Development Framework for Remarkable Tablet

[![RustPoC](https://i.imgur.com/c9YCAsy.jpg)](https://i.imgur.com/c9YCAsy.jpg)

[![PoC](https://thumbs.gfycat.com/WelltodoImprobableAlligator-size_restricted.gif)](https://gfycat.com/gifs/detail/WelltodoImprobableAlligator)

Everything from low latency partial updates to the eInk display to at least minimal multitouch, physical button and Wacom Digitizer input is now understood and their minimal to complete implementations can be found in this repository.

The focus of this repository is now going to be the Rust library for providing these features. Potentially a piston backend might be created for Remarkable, allowing the use of conrod to simplify UI creation.

In cases where Rust implementation seems to contradict with the C implementation, the Rust implementation can be taken as the source of truth as the C-implementation was the first-pass that came to being during the exploration stage.

#### Build Instructions

In order to build `libremarkable` and the examples (`spy.so` and `demo`), you'll need the following configuration after having installed the proper toolchain to your `$PATH`. The installation of `arm-linux-gnueabihf-gcc` toolchain is described at [KnowledgeBase.md](https://github.com/canselcik/libremarkable/KnowledgeBase.md).

You can then set up your Rust toolchain for cross compilation with: `rustup target add armv7-unknown-linux-gnueabihf`.

Once that's done, you should add the following to your `~/.cargo/config`:
```
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

After this, you will be able to build these projects with:
```
➜  rust-poc git:(master) ✗ cargo build --release --target=armv7-unknown-linux-gnueabihf
   ...
   Compiling libremarkable v0.1.0 (file:///home/main/Desktop/RemarkableFramebuffer/libremarkable)
   Compiling rust-poc v0.1.0 (file:///home/main/Desktop/RemarkableFramebuffer/rust-poc)
    Finished dev [unoptimized + debuginfo] target(s) in 24.85 secs
```

Note that the `--release` argument is important as this enables optimizations and without optimizations you'll be looking at ~70% CPU utilization even when idle. With optimizations, `rust-poc` runs really light, 0% CPU utilization when idle and 1-2% at peak.

For further documentation see [KnowledgeBase.md](https://github.com/canselcik/libremarkable/KnowledgeBase.md).
