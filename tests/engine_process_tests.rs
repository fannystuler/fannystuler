use std::sync::Arc;

use zapret2_gui::engine::{EngineProcessManager, LogEntry, ProcessError};

fn noop_logger() -> Arc<dyn Fn(LogEntry) + Send + Sync> {
    Arc::new(|_entry| {})
}

#[tokio::test]
async fn start_missing_binary_returns_error() {
    let manager = EngineProcessManager::new(noop_logger());
    let result = manager
        .start::<&std::path::Path>(
            std::path::Path::new("definitely-not-existing-binary-xyz.exe"),
            None,
            &[],
        )
        .await;
    assert!(matches!(result, Err(ProcessError::BinaryNotFound(_))));
}

#[tokio::test]
async fn is_running_false_when_not_started() {
    let manager = EngineProcessManager::new(noop_logger());
    assert!(!manager.is_running().await);
}

#[tokio::test]
async fn stop_when_not_running_is_ok() {
    let manager = EngineProcessManager::new(noop_logger());
    manager.stop().await.expect("stop should be ok");
}

#[tokio::test]
async fn start_and_stop_real_process() {
    // Используем кроссплатформенный способ запуска: на Windows — cmd /C, иначе — sh -c.
    #[cfg(windows)]
    let (bin, args): (&str, Vec<String>) = (
        "cmd",
        vec!["/C".to_string(), "echo hello".to_string()],
    );
    #[cfg(not(windows))]
    let (bin, args): (&str, Vec<String>) = (
        "sh",
        vec!["-c".to_string(), "echo hello".to_string()],
    );

    let manager = EngineProcessManager::new(noop_logger());
    manager
        .start(std::path::Path::new(bin), None, &args)
        .await
        .expect("start should succeed");

    // Даём процессу завершиться.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let _ = manager.is_running().await;
    manager.stop().await.expect("stop should succeed");
}
