fn main() {
    #[cfg(target_os = "windows")]
    {
        let _ = embed_resource::compile("windows-app.manifest", embed_resource::NONE);
    }

    tauri_build::build();
}
