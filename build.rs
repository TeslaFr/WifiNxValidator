use std::{env, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=assets/windows.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    // Development/test executables do not trigger UAC. Distributed releases do.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        && env::var("PROFILE").as_deref() == Ok("release")
        && env::var_os("CARGO_FEATURE_DESKTOP").is_some()
    {
        let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("assets/windows.manifest");
        println!("cargo:rustc-link-arg-bin=wifinx-validator=/MANIFEST:EMBED");
        // The supplied manifest owns the UAC section; do not merge a generated asInvoker section.
        println!("cargo:rustc-link-arg-bin=wifinx-validator=/MANIFESTUAC:NO");
        println!(
            "cargo:rustc-link-arg-bin=wifinx-validator=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
