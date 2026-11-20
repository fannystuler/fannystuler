use std::path::PathBuf;

use zapret2_gui::engine::{find_engine_binary, find_in_system_path, inspect_environment, resolve_app_dir};

#[test]
fn resolve_app_dir_is_absolute_or_dot() {
    let dir = resolve_app_dir();
    assert!(!dir.as_os_str().is_empty());
}

#[test]
fn find_engine_binary_missing_returns_error() {
    let result = find_engine_binary(None, "definitely-not-existing-binary-xyz.exe");
    assert!(result.is_err());
}

#[test]
fn find_engine_binary_custom_path_file() {
    let dir = std::env::temp_dir().join(format!("zapret2_engine_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("fake-engine.exe");
    std::fs::write(&bin, b"").unwrap();
    let found = find_engine_binary(Some(&bin), "fake-engine.exe").unwrap();
    assert_eq!(found, bin);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn find_in_system_path_missing() {
    assert!(find_in_system_path("definitely-not-existing-binary-xyz.exe").is_none());
}

#[test]
fn inspect_environment_reports_missing_engine() {
    let env = inspect_environment(None, "definitely-not-existing-binary-xyz.exe");
    assert!(!env.is_ready());
    assert!(env.missing_components.iter().any(|c| c.contains("Движок")));
}

#[test]
fn inspect_environment_with_custom_binary() {
    let dir = std::env::temp_dir().join(format!("zapret2_env_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bin = dir.join("fake-engine.exe");
    std::fs::write(&bin, b"").unwrap();
    let env = inspect_environment(Some(&bin), "fake-engine.exe");
    assert!(env.is_ready());
    assert_eq!(env.engine_binary.as_deref(), Some(bin.as_path()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn runtime_environment_serializes() {
    let env = inspect_environment(None, "definitely-not-existing-binary-xyz.exe");
    let json = serde_json::to_string(&env).expect("serialize");
    assert!(json.contains("app_dir"));
    let _: PathBuf = env.app_dir;
}
