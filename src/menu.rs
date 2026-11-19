use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::{build_args, LaunchProfile, ProfileStorage, Protocol, StrategyPreset};
use crate::elevation;
use crate::engine::{inspect_environment, EngineProcessManager};
use crate::lists::HostlistManager;

fn read_line_trimmed(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().lock().read_line(&mut input);
    input.trim().to_string()
}

fn save_hostlist_to_file(path: &PathBuf, domains: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("не удалось создать папку: {e}"))?;
        }
    }
    let content = if domains.is_empty() {
        String::new()
    } else {
        format!("{}\n", domains.join("\n"))
    };
    fs::write(path, content).map_err(|e| format!("не удалось записать файл: {e}"))
}

pub async fn run_menu() {
    let storage = ProfileStorage::new("./config/profiles.json");
    let mut hostlist = HostlistManager::new();
    let manager = EngineProcessManager::new(Arc::new(|entry| {
        println!("[{}] {}", entry.stream, entry.message);
    }));

    loop {
        let env_info = inspect_environment(None, "winws2.exe");
        let cfg = storage.load_or_init().unwrap_or_default();
        let running = manager.is_running().await;

        println!("\n=== ZAPRET2 MANAGER ===");
        println!("Администратор: {}", elevation::is_elevated());
        println!("Движок найден: {}", env_info.is_ready());
        println!("Статус процесса: {}", if running { "ЗАПУЩЕН" } else { "остановлен" });
        println!(
            "Активный профиль: {}",
            cfg.active_profile.as_deref().unwrap_or("не выбран")
        );
        println!("1. Показать профили и выбрать активный");
        println!("2. Создать профиль HTTPS (TCP 443)");
        println!("3. Показать аргументы запуска активного профиля");
        println!("4. Добавить домен в hostlist");
        println!("5. Сохранить hostlist в файл");
        println!("6. Запустить движок (активный профиль)");
        println!("7. Остановить движок");
        println!("8. Диагностика путей");
        println!("0. Выход");

        match read_line_trimmed("Выберите пункт: ").as_str() {
            "1" => {
                if cfg.profiles.is_empty() {
                    println!("Профилей пока нет — создайте через пункт 2.");
                    continue;
                }
                for (i, p) in cfg.profiles.iter().enumerate() {
                    let mark = if cfg.active_profile.as_deref() == Some(p.name.as_str()) {
                        "*"
                    } else {
                        " "
                    };
                    println!("  {mark} [{}] {}", i + 1, p.name);
                }
                let choice = read_line_trimmed("Номер профиля (Enter — отмена): ");
                if let Ok(idx) = choice.parse::<usize>() {
                    if let Some(selected) = cfg.profiles.get(idx.saturating_sub(1)) {
                        let mut updated = cfg.clone();
                        updated.active_profile = Some(selected.name.clone());
                        if let Err(e) = storage.save(&updated) {
                            eprintln!("Ошибка сохранения: {e}");
                        } else {
                            println!("Выбран профиль: {}", selected.name);
                        }
                    }
                }
            }
            "2" => {
                let name = read_line_trimmed("Название профиля: ");
                if name.is_empty() {
                    println!("Имя не может быть пустым.");
                    continue;
                }
                let new_profile = LaunchProfile {
                    name: name.clone(),
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
                let mut updated = cfg.clone();
                updated.profiles.push(new_profile);
                updated.active_profile = Some(name.clone());
                if let Err(e) = storage.save(&updated) {
                    eprintln!("Ошибка сохранения профиля: {e}");
                } else {
                    println!("Профиль '{name}' создан и выбран активным.");
                }
            }
            "3" => {
                if let Some(active_name) = &cfg.active_profile {
                    if let Some(profile) = cfg.profiles.iter().find(|p| &p.name == active_name) {
                        match build_args(profile) {
                            Ok(args) => println!("Аргументы: {args:?}"),
                            Err(e) => eprintln!("Ошибка сборки аргументов: {e}"),
                        }
                    }
                } else {
                    println!("Нет активного профиля.");
                }
            }
            "4" => {
                let domain = read_line_trimmed("Введите домен или URL (или 'list' для просмотра): ");
                if domain == "list" {
                    println!("Домены ({}): {:?}", hostlist.domains.len(), hostlist.domains);
                } else if !domain.is_empty() {
                    match hostlist.add_domain(&domain) {
                        Ok(_) => println!("Домены в списке: {:?}", hostlist.domains),
                        Err(e) => eprintln!("Ошибка добавления домена: {e}"),
                    }
                }
            }
            "5" => {
                let target_input = read_line_trimmed("Путь к файлу [./lists/list-general.txt]: ");
                let target_path = if target_input.is_empty() {
                    PathBuf::from("./lists/list-general.txt")
                } else {
                    PathBuf::from(target_input)
                };
                match save_hostlist_to_file(&target_path, &hostlist.domains) {
                    Ok(_) => println!(
                        "Сохранено доменов: {} в {}",
                        hostlist.domains.len(),
                        target_path.display()
                    ),
                    Err(e) => eprintln!("{e}"),
                }
            }
            "6" => {
                if running {
                    println!("Движок уже запущен. Сначала остановите его (пункт 7).");
                    continue;
                }
                let Some(bin_path) = env_info.engine_binary else {
                    eprintln!("Бинарник winws2.exe не найден.");
                    continue;
                };
                let Some(active_name) = &cfg.active_profile else {
                    eprintln!("Нет активного профиля.");
                    continue;
                };
                let Some(profile) = cfg.profiles.iter().find(|p| &p.name == active_name) else {
                    eprintln!("Активный профиль не найден.");
                    continue;
                };
                match build_args(profile) {
                    Ok(args) => {
                        if let Err(e) = manager
                            .start(bin_path, Some(env_info.app_dir), &args)
                            .await
                        {
                            eprintln!("Ошибка запуска движка: {e}");
                        }
                    }
                    Err(e) => eprintln!("Ошибка аргументов: {e}"),
                }
            }
            "7" => {
                if let Err(e) = manager.stop().await {
                    eprintln!("Ошибка остановки: {e}");
                } else {
                    println!("Движок остановлен.");
                }
            }
            "8" => {
                println!("{env_info:#?}");
            }
            "0" => {
                let _ = manager.stop().await;
                break;
            }
            _ => println!("Неизвестный пункт."),
        }
    }
}
