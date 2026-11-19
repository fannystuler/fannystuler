mod config;
mod elevation;
mod engine;
mod lists;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex as AsyncMutex;

use config::{build_args, LaunchProfile, ProfileStorage, ProfilesConfig, Protocol, StrategyPreset};
use engine::{inspect_environment, EngineProcessManager};
use lists::HostlistManager;

pub struct AppState {
    pub storage: ProfileStorage,
    pub hostlist: Mutex<HostlistManager>,
    pub engine: Arc<AsyncMutex<EngineProcessManager>>,
}

#[derive(Serialize)]
pub struct EnvStatus {
    pub elevated: bool,
    pub engine_ready: bool,
    pub engine_binary: Option<String>,
    pub app_dir: String,
    pub running: bool,
}

#[derive(Serialize)]
pub struct CreateProfileInput {
    pub name: String,
    pub ports: Vec<u16>,
    pub ttl: u8,
    pub split_pos: usize,
    pub debug: bool,
}

#[tauri::command]
fn get_env_status(state: State<'_, AppState>) -> Result<EnvStatus, String> {
    let env = inspect_environment(None, "winws2.exe");
    let running = {
        let mgr = state
            .engine
            .try_lock()
            .map_err(|_| "Менеджер процесса занят".to_string())?;
        mgr.is_running()
    };

    Ok(EnvStatus {
        elevated: elevation::is_elevated(),
        engine_ready: env.is_ready(),
        engine_binary: env.engine_binary.map(|p| p.to_string_lossy().into_owned()),
        app_dir: env.app_dir.to_string_lossy().into_owned(),
        running,
    })
}

#[tauri::command]
fn list_profiles(state: State<'_, AppState>) -> Result<ProfilesConfig, String> {
    state
        .storage
        .load_or_init()
        .map_err(|e| format!("Ошибка загрузки профилей: {e}"))
}

#[tauri::command]
fn set_active_profile(state: State<'_, AppState>, name: String) -> Result<ProfilesConfig, String> {
    let mut cfg = state
        .storage
        .load_or_init()
        .map_err(|e| format!("Ошибка чтения профилей: {e}"))?;

    if !cfg.profiles.iter().any(|p| p.name == name) {
        return Err(format!("Профиль '{name}' не найден"));
    }

    cfg.active_profile = Some(name);
    state
        .storage
        .save(&cfg)
        .map_err(|e| format!("Ошибка сохранения: {e}"))?;
    Ok(cfg)
}

#[tauri::command]
fn create_https_profile(
    state: State<'_, AppState>,
    name: String,
    ports: Option<Vec<u16>>,
    ttl: Option<u8>,
    split_pos: Option<usize>,
    debug: Option<bool>,
) -> Result<ProfilesConfig, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Название профиля не может быть пустым".to_string());
    }

    let mut cfg = state
        .storage
        .load_or_init()
        .map_err(|e| format!("Ошибка чтения профилей: {e}"))?;

    let profile = LaunchProfile {
        name: trimmed.to_string(),
        protocol: Protocol::Tcp,
        ports: ports.filter(|p| !p.is_empty()).unwrap_or_else(|| vec![443]),
        hostlist_path: Some(PathBuf::from("./lists/list-general.txt")),
        hostlist_exclude_path: None,
        lua_script_path: None,
        strategy: StrategyPreset::FakeAndSplit {
            ttl: ttl.unwrap_or(3),
            split_pos: split_pos.unwrap_or(2),
        },
        debug: debug.unwrap_or(false),
    };

    cfg.profiles.push(profile);
    cfg.active_profile = Some(trimmed.to_string());
    state
        .storage
        .save(&cfg)
        .map_err(|e| format!("Ошибка сохранения: {e}"))?;

    Ok(cfg)
}

#[tauri::command]
fn preview_active_args(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let cfg = state
        .storage
        .load_or_init()
        .map_err(|e| format!("Ошибка чтения профилей: {e}"))?;

    let active_name = cfg
        .active_profile
        .ok_or_else(|| "Активный профиль не выбран".to_string())?;

    let profile = cfg
        .profiles
        .iter()
        .find(|p| p.name == active_name)
        .ok_or_else(|| format!("Профиль '{active_name}' не найден"))?;

    build_args(profile)
}

#[tauri::command]
fn add_host_domain(state: State<'_, AppState>, domain: String) -> Result<Vec<String>, String> {
    let mut hl = state
        .hostlist
        .lock()
        .map_err(|_| "Блокировка списка доменов повреждена".to_string())?;

    hl.add_domain(&domain)
        .map_err(|e| format!("Ошибка добавления домена: {e}"))?;

    Ok(hl.domains.clone())
}

#[tauri::command]
async fn start_engine(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let env = inspect_environment(None, "winws2.exe");
    let bin_path = env
        .engine_binary
        .ok_or_else(|| "Бинарник winws2.exe не найден".to_string())?;

    let cfg = state
        .storage
        .load_or_init()
        .map_err(|e| format!("Ошибка чтения профилей: {e}"))?;

    let active_name = cfg
        .active_profile
        .ok_or_else(|| "Активный профиль не выбран".to_string())?;

    let profile = cfg
        .profiles
        .iter()
        .find(|p| p.name == active_name)
        .ok_or_else(|| format!("Профиль '{active_name}' не найден"))?;

    let args = build_args(profile)?;
    let mut mgr = state.engine.lock().await;
    if mgr.is_running() {
        return Err("Движок уже запущен".to_string());
    }

    mgr.start(bin_path, Some(env.app_dir), &args)
        .await
        .map_err(|e| format!("Ошибка запуска движка: {e}"))?;

    Ok(args)
}

#[tauri::command]
async fn stop_engine(state: State<'_, AppState>) -> Result<(), String> {
    let mut mgr = state.engine.lock().await;
    mgr.stop()
        .await
        .map_err(|e| format!("Ошибка остановки движка: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let log_cb = Arc::new(move |entry: engine::LogEntry| {
                let _ = handle.emit("engine-log", &entry);
            });

            let app_state = AppState {
                storage: ProfileStorage::new("./config/profiles.json"),
                hostlist: Mutex::new(HostlistManager::new()),
                engine: Arc::new(AsyncMutex::new(EngineProcessManager::new(log_cb))),
            };

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_env_status,
            list_profiles,
            set_active_profile,
            create_https_profile,
            preview_active_args,
            add_host_domain,
            start_engine,
            stop_engine,
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска Tauri приложения");
}
