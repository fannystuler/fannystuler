use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrategyPreset {
    FakeAndSplit { ttl: u8, split_pos: usize },
    SplitOnly { split_pos: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchProfile {
    pub name: String,
    pub protocol: Protocol,
    pub ports: Vec<u16>,
    pub hostlist_path: Option<PathBuf>,
    pub hostlist_exclude_path: Option<PathBuf>,
    pub lua_script_path: Option<PathBuf>,
    pub strategy: StrategyPreset,
    pub debug: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesConfig {
    pub active_profile: Option<String>,
    pub profiles: Vec<LaunchProfile>,
}

impl Default for ProfilesConfig {
    fn default() -> Self {
        let default_profile = LaunchProfile {
            name: "Default HTTPS".to_string(),
            protocol: Protocol::Tcp,
            ports: vec![443],
            hostlist_path: Some(PathBuf::from("./lists/list-general.txt")),
            hostlist_exclude_path: None,
            lua_script_path: None,
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
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }

    pub fn load_or_init(&self) -> Result<ProfilesConfig, String> {
        if !self.path.exists() {
            let cfg = ProfilesConfig::default();
            self.save(&cfg)?;
            return Ok(cfg);
        }

        let content = fs::read_to_string(&self.path)
            .map_err(|e| format!("Не удалось прочитать {}: {e}", self.path.display()))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Некорректный JSON в {}: {e}", self.path.display()))
    }

    pub fn save(&self, cfg: &ProfilesConfig) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Не удалось создать директорию {}: {e}", parent.display()))?;
            }
        }

        let json = serde_json::to_string_pretty(cfg)
            .map_err(|e| format!("Ошибка сериализации конфига: {e}"))?;
        fs::write(&self.path, json)
            .map_err(|e| format!("Не удалось записать {}: {e}", self.path.display()))
    }
}

fn format_path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub fn build_args(profile: &LaunchProfile) -> Result<Vec<String>, String> {
    if profile.ports.is_empty() {
        return Err("Список портов не может быть пустым".to_string());
    }
    if profile.ports.iter().any(|&p| p == 0) {
        return Err("Порт не может быть 0".to_string());
    }

    let mut args = Vec::new();
    if profile.debug {
        args.push("--debug".to_string());
    }

    let ports_str = profile
        .ports
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");

    match profile.protocol {
        Protocol::Tcp => {
            args.push("--filter-tcp".to_string());
            args.push(ports_str);
        }
        Protocol::Udp => {
            args.push("--filter-udp".to_string());
            args.push(ports_str);
        }
    }

    if let Some(ref hl) = profile.hostlist_path {
        args.push("--hostlist".to_string());
        args.push(format_path(hl));
    }

    if let Some(ref excl) = profile.hostlist_exclude_path {
        args.push("--hostlist-exclude".to_string());
        args.push(format_path(excl));
    }

    if let Some(ref lua) = profile.lua_script_path {
        args.push("--lua-init".to_string());
        args.push(format_path(lua));
    }

    match profile.strategy {
        StrategyPreset::FakeAndSplit { ttl, split_pos } => {
            if ttl == 0 {
                return Err("TTL должен быть больше 0".to_string());
            }
            if split_pos == 0 || split_pos > 1500 {
                return Err("split_pos должен быть в диапазоне 1..=1500".to_string());
            }
            args.push("--dpi-desync".to_string());
            args.push("fake,split2".to_string());
            args.push(format!("--dpi-desync-ttl={ttl}"));
            args.push(format!("--dpi-desync-split-pos={split_pos}"));
        }
        StrategyPreset::SplitOnly { split_pos } => {
            if split_pos == 0 || split_pos > 1500 {
                return Err("split_pos должен быть в диапазоне 1..=1500".to_string());
            }
            args.push("--dpi-desync".to_string());
            args.push("split2".to_string());
            args.push(format!("--dpi-desync-split-pos={split_pos}"));
        }
    }

    Ok(args)
}
