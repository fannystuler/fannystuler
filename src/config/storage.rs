use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::{LaunchProfile, Protocol, StrategyPreset};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Ошибка чтения файла профилей {path}: {source}")]
    ReadIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Ошибка записи файла профилей {path}: {source}")]
    WriteIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Ошибка разбора или сериализации JSON ({path}): {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesConfig {
    pub active_profile: Option<String>,
    pub profiles: Vec<LaunchProfile>,
}

impl Default for ProfilesConfig {
    fn default() -> Self {
        let default_profile = LaunchProfile {
            name: "HTTPS Основной".to_string(),
            protocol: Protocol::Tcp,
            ports: vec![443],
            hostlist_path: Some(PathBuf::from("./lists/list-general.txt")),
            hostlist_exclude_path: Some(PathBuf::from("./lists/list-exclude.txt")),
            lua_script_path: Some(PathBuf::from("./lua/zapret-antidpi.lua")),
            strategy: StrategyPreset::FakeAndSplit {
                ttl: 3,
                split_pos: 2,
            },
            debug: false,
        };

        Self {
            active_profile: Some(default_profile.name.clone()),
            profiles: vec![default_profile],
        }
    }
}

pub struct ProfileStorage {
    path: PathBuf,
}

impl ProfileStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Загружает профили из JSON-файла.
    /// Если файла ещё нет, создаёт его с профилем по умолчанию и возвращает его.
    pub fn load_or_init(&self) -> Result<ProfilesConfig, StorageError> {
        if !self.path.exists() {
            let default_cfg = ProfilesConfig::default();
            self.save(&default_cfg)?;
            return Ok(default_cfg);
        }

        let raw = fs::read_to_string(&self.path).map_err(|source| StorageError::ReadIo {
            path: self.path.clone(),
            source,
        })?;

        serde_json::from_str::<ProfilesConfig>(&raw).map_err(|source| StorageError::Json {
            path: self.path.clone(),
            source,
        })
    }

    /// Сохраняет текущую конфигурацию профилей в красивый JSON.
    pub fn save(&self, config: &ProfilesConfig) -> Result<(), StorageError> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|source| StorageError::WriteIo {
                    path: self.path.clone(),
                    source,
                })?;
            }
        }

        let json = serde_json::to_string_pretty(config).map_err(|source| StorageError::Json {
            path: self.path.clone(),
            source,
        })?;

        // Пишем во временный файл и переименовываем, чтобы не повредить конфиг при сбое
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, json).map_err(|source| StorageError::WriteIo {
            path: tmp_path.clone(),
            source,
        })?;

        fs::rename(&tmp_path, &self.path).map_err(|source| StorageError::WriteIo {
            path: self.path.clone(),
            source,
        })
    }

    /// Добавляет новый профиль или обновляет существующий с таким же именем, затем сохраняет файл.
    pub fn upsert_profile(&self, profile: LaunchProfile) -> Result<ProfilesConfig, StorageError> {
        let mut cfg = self.load_or_init()?;

        if let Some(existing) = cfg.profiles.iter_mut().find(|p| p.name == profile.name) {
            *existing = profile.clone();
        } else {
            cfg.profiles.push(profile.clone());
        }

        if cfg.active_profile.is_none() {
            cfg.active_profile = Some(profile.name);
        }

        self.save(&cfg)?;
        Ok(cfg)
    }

    /// Удаляет профиль по имени и при необходимости переключает активный профиль на первый доступный.
    pub fn delete_profile(&self, name: &str) -> Result<ProfilesConfig, StorageError> {
        let mut cfg = self.load_or_init()?;
        cfg.profiles.retain(|p| p.name != name);

        if cfg.active_profile.as_deref() == Some(name) {
            cfg.active_profile = cfg.profiles.first().map(|p| p.name.clone());
        }

        self.save(&cfg)?;
        Ok(cfg)
    }
}
