use std::fs;
use std::path::PathBuf;

pub struct HostlistManager {
    pub path: PathBuf,
    pub domains: Vec<String>,
}

impl HostlistManager {
    pub fn new() -> Self {
        let path = PathBuf::from("./lists/list-general.txt");
        let domains = fs::read_to_string(&path)
            .map(|content| {
                content
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty() && !l.starts_with('#'))
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        Self { path, domains }
    }

    pub fn add_domain(&mut self, domain: &str) -> Result<(), String> {
        let normalized = domain
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase();

        if normalized.is_empty() || normalized.contains(' ') {
            return Err("Некорректный формат домена".to_string());
        }

        if !self.domains.contains(&normalized) {
            self.domains.push(normalized);
            self.save()?;
        }

        Ok(())
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Не удалось создать директорию {}: {e}", parent.display()))?;
            }
        }

        let mut data = self.domains.join("\n");
        if !data.is_empty() {
            data.push('\n');
        }

        fs::write(&self.path, data)
            .map_err(|e| format!("Не удалось сохранить {}: {e}", self.path.display()))
    }
}

impl Default for HostlistManager {
    fn default() -> Self {
        Self::new()
    }
}
