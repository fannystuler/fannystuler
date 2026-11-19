use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Движок уже запущен")]
    AlreadyRunning,
    #[error("Исполняемый файл не найден: {0}")]
    BinaryNotFound(PathBuf),
    #[error("Ошибка запуска/управления процессом: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub stream: String,
    pub message: String,
}

pub type LogCallback = Arc<dyn Fn(LogEntry) + Send + Sync>;

#[derive(Clone)]
pub struct EngineProcessManager {
    child: Arc<Mutex<Option<Child>>>,
    on_log: LogCallback,
}

impl EngineProcessManager {
    pub fn new(on_log: LogCallback) -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            on_log,
        }
    }

    pub async fn is_running(&self) -> bool {
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    *guard = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub async fn start<P: AsRef<Path>>(
        &self,
        binary_path: P,
        working_dir: Option<P>,
        args: &[String],
    ) -> Result<(), ProcessError> {
        let binary_path = binary_path.as_ref().to_path_buf();
        if !binary_path.exists() {
            return Err(ProcessError::BinaryNotFound(binary_path));
        }

        let mut guard = self.child.lock().await;
        if let Some(existing) = guard.as_mut() {
            if existing.try_wait()?.is_none() {
                return Err(ProcessError::AlreadyRunning);
            }
        }

        let mut cmd = Command::new(&binary_path);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir.as_ref());
        }

        #[cfg(windows)]
{
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let log_cb = Arc::clone(&self.on_log);
        (log_cb)(LogEntry {
            stream: "system".into(),
            message: format!("Запущен процесс: {} {:?}", binary_path.display(), args),
        });

        if let Some(out) = stdout {
            let cb = Arc::clone(&self.on_log);
            tokio::spawn(async move {
                let mut reader = BufReader::new(out).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    (cb)(LogEntry {
                        stream: "stdout".into(),
                        message: line,
                    });
                }
            });
        }

        if let Some(err) = stderr {
            let cb = Arc::clone(&self.on_log);
            tokio::spawn(async move {
                let mut reader = BufReader::new(err).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    (cb)(LogEntry {
                        stream: "stderr".into(),
                        message: line,
                    });
                }
            });
        }

        *guard = Some(child);
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), ProcessError> {
        let maybe_child = {
            let mut guard = self.child.lock().await;
            guard.take()
        };

        if let Some(mut child) = maybe_child {
            if child.try_wait()?.is_none() {
                child.kill().await?;
            }
            let status = child.wait().await?;
            (self.on_log)(LogEntry {
                stream: "system".into(),
                message: format!("Процесс остановлен. Код выхода: {:?}", status.code()),
            });
        } else {
            (self.on_log)(LogEntry {
                stream: "system".into(),
                message: "Процесс уже остановлен.".into(),
            });
        }

        Ok(())
    }
}
