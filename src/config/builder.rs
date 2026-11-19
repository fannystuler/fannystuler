use crate::config::{LaunchProfile, Protocol, StrategyPreset};
use std::path::Path;

fn format_ports(ports: &[u16]) -> String {
    ports
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

pub fn build_args(profile: &LaunchProfile) -> Result<Vec<String>, String> {
    if profile.ports.is_empty() {
        return Err("Не указан ни один порт для перехвата".to_string());
    }

    let mut args: Vec<String> = Vec::new();
    let ports_csv = format_ports(&profile.ports);

    if profile.debug {
        args.push("--debug".to_string());
    }

    match profile.protocol {
        Protocol::Tcp => {
            args.push(format!("--wf-tcp={ports_csv}"));
        }
        Protocol::Udp => {
            args.push(format!("--wf-udp={ports_csv}"));
        }
    }

    if let Some(hostlist) = profile.hostlist_path.as_deref() {
        args.push(format!("--hostlist={}", path_str(hostlist)));
    }
    if let Some(exclude) = profile.hostlist_exclude_path.as_deref() {
        args.push(format!("--hostlist-exclude={}", path_str(exclude)));
    }
    if let Some(lua) = profile.lua_script_path.as_deref() {
        args.push(format!("--lua-init={}", path_str(lua)));
    }

    match &profile.strategy {
        StrategyPreset::TlsSplit { split_pos } => {
            if *split_pos == 0 {
                return Err("Смещение фрагментации должно быть больше 0".to_string());
            }
            args.push("--dpi-desync=split".to_string());
            args.push(format!("--dpi-desync-split-pos={split_pos}"));
        }
        StrategyPreset::FakeLowTtl { ttl, repeats } => {
            if *ttl == 0 {
                return Err("TTL должен быть больше 0".to_string());
            }
            args.push("--dpi-desync=fake".to_string());
            args.push(format!("--dpi-desync-ttl={ttl}"));
            args.push(format!("--dpi-desync-repeats={repeats}"));
        }
        StrategyPreset::FakeAndSplit { ttl, split_pos } => {
            if *ttl == 0 || *split_pos == 0 {
                return Err("TTL и смещение фрагментации должны быть больше 0".to_string());
            }
            args.push("--dpi-desync=fake,split".to_string());
            args.push(format!("--dpi-desync-ttl={ttl}"));
            args.push(format!("--dpi-desync-split-pos={split_pos}"));
        }
    }

    Ok(args)
}
