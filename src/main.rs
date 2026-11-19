mod config;
mod elevation;
mod engine;
mod lists;

use std::path::PathBuf;
use std::sync::Arc;

use config::{build_args, LaunchProfile, ProfileStorage, Protocol, StrategyPreset};
use engine::EngineProcessManager;
use lists::HostlistManager;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("не удалось создать Tokio runtime");

    rt.block_on(async {
        println!("Проверка прав администратора: {}", elevation::is_elevated());

        let env_info = engine::inspect_environment(None, "winws2.exe");
        println!("Готов к запуску движка: {}", env_info.is_ready());
        println!("Найденный бинарник: {:?}", env_info.engine_binary);
        if !env_info.missing_components.is_empty() {
            println!("Не найдено: {:?}", env_info.missing_components);
        }

        let storage = ProfileStorage::new("./config/profiles.json");
        match storage.load_or_init() {
            Ok(cfg) => println!(
                "Загружено профилей: {}, активный: {:?}",
                cfg.profiles.len(),
                cfg.active_profile
            ),
            Err(err) => eprintln!("Ошибка хранения профилей: {err}"),
        }

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
    });
}
