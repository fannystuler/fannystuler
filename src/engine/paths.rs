use std::env;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathsError {
    #[error("Бинарный файл движка {binary_name} не найден. Проверены пути: {checked_paths}")]
    BinaryNotFound {
        binary_name: String,
        checked_paths: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEnvironment {
    pub app_dir: PathBuf,
    pub engine_binary: Option<PathBuf>,
    pub windivert_sys: Option<PathBuf>,
    pub lists_dir: PathBuf,
    pub lua_dir: PathBuf,
    pub config_dir: PathBuf,
    pub missing_components: Vec<String>,
}

impl RuntimeEnvironment {
    pub fn is_ready(&self) -> bool {
        self.engine_binary.is_some()
    }
}

/// Определяет директорию, в которой лежит текущий исполняемый файл (или текущую рабочую папку).
pub fn resolve_app_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Ищет исполняемый файл движка (например, `winws2.exe`) рядом с приложением,
/// в подкаталогах `bin/` / `engine/` / `zapret2/` или в системном `PATH`.
pub fn find_engine_binary(custom_path: Option<&Path>, binary_name: &str) -> Result<PathBuf, PathsError> {
    let app_dir = resolve_app_dir();
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(custom) = custom_path {
        if custom.is_dir() {
            candidates.push(custom.join(binary_name));
        } else {
            candidates.push(custom.to_path_buf());
        }
    }

    candidates.extend([
        app_dir.join(binary_name),
        app_dir.join("bin").join(binary_name),
        app_dir.join("engine").join(binary_name),
        app_dir.join("zapret2").join(binary_name),
        PathBuf::from(".").join(binary_name),
        PathBuf::from(".").join("bin").join(binary_name),
    ]);

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }

    if let Some(from_path) = find_in_system_path(binary_name) {
        return Ok(from_path);
    }

    let checked_str = candidates
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(PathsError::BinaryNotFound {
        binary_name: binary_name.to_string(),
        checked_paths: checked_str,
    })
}

/// Ищет файл в директориях из переменной окружения `PATH`.
pub fn find_in_system_path(file_name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Собирает полную диагностику окружения: бинарник движка, драйвер WinDivert и папки данных.
pub fn inspect_environment(custom_engine_path: Option<&Path>, binary_name: &str) -> RuntimeEnvironment {
    let app_dir = resolve_app_dir();
    let mut missing_components = Vec::new();

    let engine_binary = match find_engine_binary(custom_engine_path, binary_name) {
        Ok(path) => Some(path),
        Err(_) => {
            missing_components.push(format!("Движок ({binary_name})"));
            None
        }
    };

    let windivert_candidates = [
        engine_binary
            .as_ref()
            .and_then(|b| b.parent())
            .map(|dir| dir.join("WinDivert64.sys")),
        Some(app_dir.join("WinDivert64.sys")),
        Some(app_dir.join("bin").join("WinDivert64.sys")),
        find_in_system_path("WinDivert64.sys"),
    ];

    let windivert_sys = windivert_candidates
        .into_iter()
        .flatten()
        .find(|p| p.is_file());

    if windivert_sys.is_none() {
        missing_components.push("Драйвер WinDivert64.sys".to_string());
    }

    let lists_dir = if app_dir.join("lists").is_dir() {
        app_dir.join("lists")
    } else {
        PathBuf::from("./lists")
    };

    if !lists_dir.exists() {
        missing_components.push("Папка списков (lists/)".to_string());
    }

    let lua_dir = if app_dir.join("lua").is_dir() {
        app_dir.join("lua")
    } else {
        PathBuf::from("./lua")
    };

    let config_dir = if app_dir.join("config").is_dir() {
        app_dir.join("config")
    } else {
        PathBuf::from("./config")
    };

    RuntimeEnvironment {
        app_dir,
        engine_binary,
        windivert_sys,
        lists_dir,
        lua_dir,
        config_dir,
        missing_components,
    }
}
