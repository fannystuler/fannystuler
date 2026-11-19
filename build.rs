fn main() {
    #[cfg(target_os = "windows")]
    {
        let manifest = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#;

        let out_dir = std::env::var_os("OUT_DIR").expect("OUT_DIR не задан");
        let manifest_path = std::path::PathBuf::from(out_dir).join("app.manifest");
        std::fs::write(&manifest_path, manifest).expect("не удалось записать app.manifest");

        println!("cargo:rustc-link-arg-bin=zapret2-gui=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=zapret2-gui=/MANIFESTINPUT:{}",
            manifest_path.display()
        );
        println!("cargo:rerun-if-changed=build.rs");
    }
}
