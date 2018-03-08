In order to build `librustpad`, `rustspy` and `rust-poc`, you'll need the following configuration after having installed the proper toolchain to your `$PATH`:
```
➜  rust-poc git:(master) ✗ cat ~/.cargo/config
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

After this, you will be able to build these projects with:
```
➜  rust-poc git:(master) ✗ cargo build --release --target=armv7-unknown-linux-gnueabihf
   ...
   Compiling librustpad v0.1.0 (file:///home/main/Desktop/RemarkableFramebuffer/librustpad)
   Compiling rust-poc v0.1.0 (file:///home/main/Desktop/RemarkableFramebuffer/rust-poc)
    Finished dev [unoptimized + debuginfo] target(s) in 24.85 secs
```

Note that the `--release` argument is important as this enables optimizations and without optimizations you'll be looking at ~70% CPU utilization even when idle. With optimizations, `rust-poc` runs really light, 0% CPU utilization when idle and 1-2% at peak.
