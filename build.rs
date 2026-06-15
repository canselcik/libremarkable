use bindgen::callbacks::{IntKind, ParseCallbacks};
use std::env;
use std::path::PathBuf;

/// Use ulong for integers by default instead of trying to guess the size.
#[derive(Debug)]
struct IoctlInts(IntKind);

impl ParseCallbacks for IoctlInts {
    fn int_macro(&self, _name: &str, _value: i64) -> Option<IntKind> {
        Some(self.0)
    }
}

fn main() {
    let target_env = env::var("CARGO_CFG_TARGET_ENV").expect("known target env");
    let ioctl_op_type = match target_env.as_str() {
        "gnu" => IntKind::ULong,
        "musl" => IntKind::Int,
        _ => IntKind::ULong,
    };

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("./reference-material/mxcfb.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Infer the right type for our ioctls.
        .parse_callbacks(Box::new(IoctlInts(ioctl_op_type)))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("mxcfb.rs"))
        .expect("Couldn't write bindings!");
}
