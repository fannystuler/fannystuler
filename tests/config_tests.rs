use std::path::PathBuf;

use zapret2_gui::config::{build_args, LaunchProfile, Protocol, StrategyPreset};

fn sample_profile(strategy: StrategyPreset) -> LaunchProfile {
    LaunchProfile {
        name: "test".into(),
        protocol: Protocol::Tcp,
        ports: vec![443],
        hostlist_path: Some(PathBuf::from("./lists/list-general.txt")),
        hostlist_exclude_path: None,
        lua_script_path: None,
        strategy,
        debug: false,
    }
}

#[test]
fn build_args_contains_protocol_and_port() {
    let profile = sample_profile(StrategyPreset::TlsSplit { split_pos: 2 });
    let args = build_args(&profile).expect("args should build");
    let joined = args.join(" ");
    assert!(joined.contains("443"), "args: {joined}");
}

#[test]
fn build_args_fake_and_split() {
    let profile = sample_profile(StrategyPreset::FakeAndSplit { ttl: 3, split_pos: 2 });
    let args = build_args(&profile).expect("args should build");
    assert!(!args.is_empty());
}

#[test]
fn build_args_fake_low_ttl() {
    let profile = sample_profile(StrategyPreset::FakeLowTtl { ttl: 5, repeats: 2 });
    let args = build_args(&profile).expect("args should build");
    assert!(!args.is_empty());
}

#[test]
fn build_args_udp() {
    let mut profile = sample_profile(StrategyPreset::TlsSplit { split_pos: 1 });
    profile.protocol = Protocol::Udp;
    profile.ports = vec![53];
    let args = build_args(&profile).expect("args should build");
    let joined = args.join(" ");
    assert!(joined.contains("53"), "args: {joined}");
}
