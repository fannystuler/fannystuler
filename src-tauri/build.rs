fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=windows-app.manifest");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:windows-app.manifest");
    }

    tauri_build::build();
}
