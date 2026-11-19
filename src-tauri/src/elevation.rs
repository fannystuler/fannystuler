#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use std::process::Command;

    // Быстрая проверка наличия прав администратора через `fltmc`
    // (в Windows при отсутствии прав администратора команда завершается с ошибкой).
    Command::new("fltmc")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub fn is_elevated() -> bool {
    false
}
