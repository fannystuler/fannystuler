mod config;
mod elevation;
mod engine;
mod lists;
mod menu;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("не удалось создать Tokio runtime");

    rt.block_on(async {
        menu::run_menu().await;
    });
}
