fn main() {
    // The Windows runner's MSVC linker can fail while merging generated
    // manifests with conflicting requestedExecutionLevel values (LNK1327).
    // This application is a console utility and does not require an embedded
    // execution-level manifest, so disable manifest generation for its binaries.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    }
}
