use std::io::{self, BufRead, Write};
use std::sync::Arc;

use crate::config::{build_args, ProfileStorage};
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

pub async fn run_menu() {
    let storage = ProfileStorage::new("./config/profiles.json");
    let mut hostlist = HostlistManager::new();
    let manager = EngineProcessManager::new(Arc::new(|entry| {
        println!("[{}] {}", entry.stream, entry.message);
    }));

    loop {
        let env_info = inspect_environment(None, "winws2.exe");
        let cfg = storage.load_or_init().unwrap_or_default();

        println!("\n=== ZAPRET2 MANAGER ===");
        println!("Администратор: {}", elevation::is_elevated());
        println!("Движок найден: {}", env_info.is_ready());
        println!(
            "Активный профиль: {}",
            cfg.active_profile.as_deref().unwrap_or("не выбран")
        );
        println!("1. Показать профили и выбрать активный");
        println!("2. Показать аргументы запуска активного профиля");
        println!("3. Добавить домен в hostlist");
        println!("4. Запустить движок (активный профиль)");
        println!("5. Остановить движок");
        println!("6. Диагностика путей");
        println!("0. Выход");

        match read_line_trimmed("Выберите пункт: ").as_str() {
            "1" => {
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
                if let Some(active_name) = &cfg.active_profile {
                    if let Some(profile) = cfg.profiles.iter().find(|p| &p.name == active_name) {
                        match build_args(profile) {
                            Ok(args) => println!("Аргументы: {args:?}"),
                            Err(e) => eprintln!("Ошибка сборки аргументов: {e}"),
                        }
                    }
                }
            }
            "3" => {
                let domain = read_line_trimmed("Введите домен или URL: ");
                if !domain.is_empty() {
                    match hostlist.add_domain(&domain) {
                        Ok(_) => println!("Домены в списке: {:?}", hostlist.domains),
                        Err(e) => eprintln!("Ошибка добавления домена: {e}"),
                    }
                }
            }
            "4" => {
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
                    Err(e) => eprintln!("Ошибка аргументов: {e}"),
                }
            }
            "5" => {
                if let Err(e) = manager.stop().await {
                    eprintln!("Ошибка остановки: {e}");
                } else {
                    println!("Движок остановлен.");
                }
            }
            "6" => {
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
