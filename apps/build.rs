use std::process::Command;
use std::{env, str};

/// Path to the .proto source files, relative to `apps` directory
const PROTO_SRC: &str = "../proto";
/// The version should match the one we use in the `Makefile`
const RUSTFMT_TOOLCHAIN: &str = "nightly-2021-03-09";

fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rerun-if-changed={}", PROTO_SRC);

    // Try to find the path to rustfmt.
    if let Ok(output) = Command::new("rustup")
        .args(&["which", "rustfmt", "--toolchain", RUSTFMT_TOOLCHAIN])
        .output()
    {
        if let Ok(rustfmt) = str::from_utf8(&output.stdout) {
            // Set the command to be used by tonic_build below to format the
            // generated files
            let rustfmt = rustfmt.trim();
            if !rustfmt.is_empty() {
                println!("using rustfmt from path \"{}\"", rustfmt);
                env::set_var("RUSTFMT", rustfmt);
            }
        }
    }

    tonic_build::configure()
        .out_dir("src/lib/proto/generated")
        .format(true)
        // TODO try to add json encoding to simplify use for user
        // .type_attribute("types.Intent", "#[derive(serde::Serialize,
        // serde::Deserialize)]")
        .compile(
            &[format!("{}/services.proto", PROTO_SRC)],
            &[PROTO_SRC.into()],
        )
        .unwrap();
}
