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
        let cleaned = normalize_domain(domain)?;
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

        let mut content = self.domains.join("\n");
        if !content.is_empty() {
            content.push('\n');
        }
        fs::write(path, content)
    }
}

pub fn normalize_domain(domain: &str) -> io::Result<String> {
    let trimmed = domain.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Пустой домен"));
    }

    let without_scheme = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    let host = without_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if host.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Пустой домен"));
    }

    if host.len() > 253 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Домен слишком длинный"));
    }

    // Разрешены только буквы, цифры, дефис, точка и подчёркивание.
    let valid = host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_');
    if !valid {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Недопустимые символы в домене",
        ));
    }

    if host.starts_with('.') || host.ends_with('.') || host.contains("..") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Некорректный формат домена",
        ));
    }

    Ok(host)
}
