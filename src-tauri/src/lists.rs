$listsPath = "src-tauri/src/lists.rs"
if (-not (Test-Path $listsPath)) {
    New-Item -ItemType Directory -Force -Path (Split-Path $listsPath) | Out-Null
    @'
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct HostlistManager {
    pub domains: Vec<String>,
}

impl HostlistManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_domain(&mut self, domain: &str) -> io::Result<()> {
        let cleaned = domain.trim().to_lowercase();
        if cleaned.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Пустой домен"));
        }
        if !self.domains.contains(&cleaned) {
            self.domains.push(cleaned);
        }
        Ok(())
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let content = self.domains.join("\n") + if self.domains.is_empty() { "" } else { "\n" };
        fs::write(path, content)
    }
}
'@ | Set-Content -Path $listsPath -Encoding UTF8
}
