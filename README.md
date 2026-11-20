# zapret2-gui

Прототип консольного Rust-приложения для формирования аргументов `zapret2` и управления дочерним процессом на Windows.

## Возможности

- проверка прав администратора;
- конфигурация протокола и портов;
- предустановки DPI-десинхронизации;
- загрузка и сохранение доменных списков;
- запуск и остановка внешнего исполняемого файла с чтением вывода;

## Сборка

```powershell
cargo build --release
```

Для сборки Windows-бинарника нужен MSVC target:

```powershell
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

Сборка также запускается через GitHub Actions workflow `Build Windows EXE (Rust)`.
надо работать
