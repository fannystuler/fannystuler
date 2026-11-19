use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HostlistError {
    #[error("Ошибка чтения файла {path}: {source}")]
    ReadIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Ошибка записи файла {path}: {source}")]
    WriteIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Некорректный домен: '{0}'")]
    InvalidDomain(String),
}

pub fn normalize_domain(raw: &str) -> Result<Option<String>, HostlistError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return Ok(None);
    }

    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);

    let host_part = without_scheme
        .split(&['/', '?', '#'][..])
        .next()
        .unwrap_or("")
        .trim();

    let without_port = host_part.split(':').next().unwrap_or("").trim();

    let cleaned = without_port
        .trim_start_matches('.')
        .to_lowercase();

    if cleaned.is_empty() || !is_valid_host(&cleaned) {
        return Err(HostlistError::InvalidDomain(raw.trim().to_string()));
    }

    Ok(Some(cleaned))
}

fn is_valid_host(host: &str) -> bool {
    let candidate = host.strip_prefix("*.").unwrap_or(host);
    if candidate.is_empty() || candidate.len() > 253 {
        return false;
    }

    candidate.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

#[derive(Debug, Clone, Default)]
pub struct HostlistManager {
    pub domains: Vec<String>,
}

impl HostlistManager {
    pub fn new() -> Self {
        Self { domains: Vec::new() }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, HostlistError> {
        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path_buf).map_err(|source| HostlistError::ReadIo {
            path: path_buf.clone(),
            source,
        })?;

        let mut manager = Self::new();
        for line in content.lines() {
            if let Some(domain) = normalize_domain(line)? {
                manager.add_domain(domain)?;
            }
        }

        Ok(manager)
    }

    pub fn add_domain(&mut self, raw_domain: impl AsRef<str>) -> Result<bool, HostlistError> {
        let Some(norm) = normalize_domain(raw_domain.as_ref())? else {
            return Ok(false);
        };

        if self.domains.iter().any(|d| d == &norm) {
            return Ok(false);
        }

        self.domains.push(norm);
        Ok(true)
    }

    pub fn remove_domain(&mut self, domain: &str) -> bool {
        let target = domain.trim().to_lowercase();
        let before = self.domains.len();
        self.domains.retain(|d| d != &target);
        self.domains.len() < before
    }

    pub fn deduplicate(&mut self) -> usize {
        let mut seen = HashSet::with_capacity(self.domains.len());
        let before = self.domains.len();

        self.domains.retain(|d| seen.insert(d.clone()));
        before - self.domains.len()
    }

    pub fn sort_alphabetically(&mut self) {
        self.domains.sort();
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), HostlistError> {
        let path_buf = path.as_ref().to_path_buf();

        if let Some(parent) = path_buf.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|source| HostlistError::WriteIo {
                    path: path_buf.clone(),
                    source,
                })?;
            }
        }

        let mut output = self.domains.join("\n");
        if !output.is_empty() {
            output.push('\n');
        }

        fs::write(&path_buf, output).map_err(|source| HostlistError::WriteIo {
            path: path_buf,
            source,
        })
    }
}
