use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod builder;
pub mod storage;

pub use builder::build_args;
pub use storage::{ProfileStorage, ProfilesConfig, StorageError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyPreset {
    TlsSplit { split_pos: usize },
    FakeLowTtl { ttl: u8, repeats: u8 },
    FakeAndSplit { ttl: u8, split_pos: usize },
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
