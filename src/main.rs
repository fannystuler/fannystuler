mod config;
mod elevation;
mod engine;
mod lists;

use std::path::PathBuf;
use std::sync::Arc;

use config::{build_args, LaunchProfile, Protocol, StrategyPreset};
use engine::{EngineProcessManager, ProfileStorage};
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

        // Загружаем настройки, обновляем встроенный профиль и сохраняем его в JSON.
        let storage = ProfileStorage::new("./config/profiles.json");
        let mut profiles = match storage.load_or_init() {
            Ok(config) => config,
            Err(err) => {
                eprintln!("Ошибка загрузки профилей: {err}");
                return;
            }
        };

        if let Some(existing) = profiles.profiles.iter_mut().find(|p| p.name == profile.name) {
            *existing = profile.clone();
        } else {
            profiles.profiles.push(profile.clone());
        }
        profiles.active_profile = Some(profile.name.clone());

        if let Err(err) = storage.save(&profiles) {
            eprintln!("Ошибка сохранения профилей: {err}");
            return;
        }

        let active_profile = profiles
            .active_profile
            .as_deref()
            .and_then(|name| profiles.profiles.iter().find(|p| p.name == name))
            .cloned()
            .unwrap_or(profile);
        println!("Сохранено профилей: {}", profiles.profiles.len());
        println!("Активный профиль: {}", active_profile.name);

        match build_args(&active_profile) {
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
