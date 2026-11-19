use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::LaunchProfile;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfilesConfig {
    pub profiles: Vec<LaunchProfile>,
    pub active_profile: Option<String>,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Ошибка чтения/записи файла профилей {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Некорректный JSON в файле профилей {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone)]
pub struct ProfileStorage {
    path: PathBuf,
}

impl ProfileStorage {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Загружает профили; если файл ещё не создан, создаёт пустую конфигурацию.
    pub fn load_or_init(&self) -> Result<ProfilesConfig, StorageError> {
        if !self.path.exists() {
            let config = ProfilesConfig::default();
            self.save(&config)?;
            return Ok(config);
        }

        let content = fs::read_to_string(&self.path).map_err(|source| StorageError::Io {
            path: self.path.clone(),
            source,
        })?;

        serde_json::from_str(&content).map_err(|source| StorageError::Json {
            path: self.path.clone(),
            source,
        })
    }

    /// Атомарно подготавливает JSON и сохраняет текущую конфигурацию профилей.
    pub fn save(&self, config: &ProfilesConfig) -> Result<(), StorageError> {
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|source| StorageError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let json = serde_json::to_string_pretty(config).map_err(|source| StorageError::Json {
            path: self.path.clone(),
            source,
        })?;
        fs::write(&self.path, format!("{json}\n")).map_err(|source| StorageError::Io {
            path: self.path.clone(),
            source,
        })
    }
}

#[allow(dead_code)]
fn _path_is_file(path: &Path) -> bool {
    path.is_file()
}
