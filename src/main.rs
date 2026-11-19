mod config;
mod elevation;
mod engine;
mod lists;

use std::path::PathBuf;
use std::sync::Arc;

use config::{build_args, LaunchProfile, Protocol, StrategyPreset};
use engine::EngineProcessManager;
use lists::HostlistManager;

#[tokio::main]
async fn main() {
    println!("Проверка прав администратора: {}", elevation::is_elevated());

    let profile = LaunchProfile {
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
        debug: true,
    };

    match build_args(&profile) {
        Ok(args) => println!("Аргументы запуска: {:?}", args),
        Err(err) => eprintln!("Ошибка конфигурации: {err}"),
    }

    let mut hostlist = HostlistManager::new();
    let _ = hostlist.add_domain("https://example.com/path");
    let _ = hostlist.add_domain("EXAMPLE.COM");
    println!("Доменов в списке: {:?}", hostlist.domains);

    let _manager = EngineProcessManager::new(Arc::new(|entry| {
        println!("[{}] {}", entry.stream, entry.message);
    }));
}
