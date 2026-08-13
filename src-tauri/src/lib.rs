mod commands;
mod models;
mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::workspace::create_workspace,
            commands::workspace::open_workspace,
            commands::workspace::save_request,
            commands::workspace::delete_request,
            commands::workspace::save_environment,
            commands::workspace::delete_environment,
        ])
        .run(tauri::generate_context!())
        .expect("не удалось запустить ReqVault");
}
