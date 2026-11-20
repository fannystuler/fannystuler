use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub stream: String,
    pub message: String,
}

pub type LogCallback = Arc<dyn Fn(LogEntry) + Send + Sync>;

pub struct EngineEnv {
    pub app_dir: PathBuf,
    pub engine_binary: Option<PathBuf>,
}

impl EngineEnv {
    pub fn is_ready(&self) -> bool {
        self.engine_binary.is_some()
    }
}

pub fn inspect_environment(base_dir: Option<PathBuf>, binary_name: &str) -> EngineEnv {
    let app_dir = base_dir
        .or_else(|| std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    let candidates = [
        app_dir.join(binary_name),
        app_dir.join("bin").join(binary_name),
        PathBuf::from(".").join(binary_name),
        PathBuf::from("./bin").join(binary_name),
    ];

    let engine_binary = candidates.into_iter().find(|p| p.exists() && p.is_file());

    EngineEnv {
        app_dir,
        engine_binary,
    }
}

pub struct EngineProcessManager {
    child: Option<Child>,
    log_cb: LogCallback,
}

impl EngineProcessManager {
    pub fn new(log_cb: LogCallback) -> Self {
        Self {
            child: None,
            log_cb,
        }
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) | Err(_) => {
                    self.child = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub async fn start(
        &mut self,
        bin_path: PathBuf,
        current_dir: Option<PathBuf>,
        args: &[String],
    ) -> Result<(), String> {
        if self.is_running() {
            return Err("Процесс уже запущен".to_string());
        }

        let mut cmd = Command::new(&bin_path);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Не удалось запустить {}: {e}", bin_path.display()))?;

        if let Some(stdout) = child.stdout.take() {
            let cb = Arc::clone(&self.log_cb);
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                loop {
                    match reader.next_line().await {
                        Ok(Some(line)) => cb(LogEntry {
                            stream: "stdout".to_string(),
                            message: line,
                        }),
                        Ok(None) => break,
                        Err(e) => {
                            cb(LogEntry {
                                stream: "stdout".to_string(),
                                message: format!("[ошибка чтения stdout: {e}]"),
                            });
                            break;
                        }
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let cb = Arc::clone(&self.log_cb);
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                loop {
                    match reader.next_line().await {
                        Ok(Some(line)) => cb(LogEntry {
                            stream: "stderr".to_string(),
                            message: line,
                        }),
                        Ok(None) => break,
                        Err(e) => {
                            cb(LogEntry {
                                stream: "stderr".to_string(),
                                message: format!("[ошибка чтения stderr: {e}]"),
                            });
                            break;
                        }
                    }
                }
            });
        }

        self.child = Some(child);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            child
                .kill()
                .await
                .map_err(|e| format!("Не удалось остановить процесс: {e}"))?;
            let _ = child.wait().await;
        }
        Ok(())
    }
}

impl Drop for EngineProcessManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}
